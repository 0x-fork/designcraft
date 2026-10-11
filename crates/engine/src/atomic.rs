//! Crash-safe file replacement: how documents, exports and other user files reach the disk.
//!
//! [`write_atomic`] never writes the destination in place. It writes a uniquely named temporary
//! file in the same folder, flushes it (`sync_all`), renames it over the destination and then
//! flushes the folder (Unix) so the rename survives a power loss. When any step before the rename
//! fails, the temporary file is removed and the destination keeps its old bytes.
//!
//! - A replaced file keeps its permissions (Unix mode).
//! - A symbolic link keeps pointing at its file: the file it names is replaced.
//! - Windows: antivirus scanners and the search indexer briefly hold files open, which makes the
//!   replacing rename fail with `PermissionDenied`; the rename is retried with a short backoff.
//! - A failed rename (cross-device, unwritable folder) is returned as an error; there is no
//!   fallback to writing in place.
//!
//! The web has no file system, so this module is not built for `wasm32`.

use std::fs::{File, OpenOptions};
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// How many times a replacing rename is tried before its error is returned.
const RENAME_TRIES: u32 = if cfg!(windows) { 5 } else { 1 };
/// The pause before the first rename retry; it doubles up to [`RENAME_MAX_DELAY_MS`].
const RENAME_FIRST_DELAY_MS: u64 = 10;
const RENAME_MAX_DELAY_MS: u64 = 200;
/// Symbolic links followed before giving up (a loop, or an absurd chain).
const MAX_LINKS: usize = 40;

/// Atomically replace (or create) `path` with `bytes`. See the module docs.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    write_atomic_with(path, |f| f.write_all(bytes))
}

/// [`write_atomic`] for a command's `path` parameter: the error names the path.
pub(crate) fn write_file(path: &str, bytes: &[u8]) -> crate::Result<()> {
    write_atomic(Path::new(path), bytes).map_err(|e| crate::EngineError::Other(format!("{path}: {e}")))
}

/// Atomically replace (or create) `path` with what `fill` writes into the temporary file. When
/// `fill`, the flush or the rename fails, the temporary file is removed, `path` is untouched and
/// the error is returned.
pub fn write_atomic_with(path: &Path, fill: impl FnOnce(&mut File) -> io::Result<()>) -> io::Result<()> {
    let target = resolve_links(path)?;
    let name = target
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, format!("{}: not a file path", path.display())))?
        .to_string_lossy()
        .into_owned();
    let dir = match target.parent() {
        Some(d) if !d.as_os_str().is_empty() => d.to_path_buf(),
        _ => PathBuf::from("."),
    };
    let existing = std::fs::metadata(&target).ok();
    if existing.is_some() {
        check_writable(&target)?;
    }
    let (tmp, mut file) = create_temp(&dir, &name)?;
    let written = fill(&mut file).and_then(|()| file.sync_all()).and_then(|()| existing.map_or(Ok(()), |m| keep_permissions(&file, &m)));
    drop(file);
    let placed = written.and_then(|()| retry_rename(RENAME_TRIES, || std::fs::rename(&tmp, &target)));
    if let Err(e) = placed {
        // Best effort: the temporary file is all there is to clean up.
        let _ = std::fs::remove_file(&tmp);
        return Err(e);
    }
    // The new bytes are in place. A failed folder flush can't be undone and must not report the
    // save as failed.
    let _ = sync_dir(&dir);
    Ok(())
}

/// Create a fresh temporary file beside the target (the same folder, so the rename stays on one
/// volume). Names are unique per process and call; a leftover of the same name is skipped.
fn create_temp(dir: &Path, name: &str) -> io::Result<(PathBuf, File)> {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let mut last = None;
    for _ in 0..8 {
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let tmp = dir.join(format!(".{name}.{}-{n}.tmp", std::process::id()));
        match OpenOptions::new().write(true).create_new(true).open(&tmp) {
            Ok(f) => return Ok((tmp, f)),
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => last = Some(e),
            Err(e) => return Err(e),
        }
    }
    Err(last.unwrap_or_else(|| io::Error::from(io::ErrorKind::AlreadyExists)))
}

/// The file a chain of symbolic links ends at (`path` itself when it is not a link). A dangling
/// link resolves to the missing file it names, which is then created.
fn resolve_links(path: &Path) -> io::Result<PathBuf> {
    let mut p = path.to_path_buf();
    for _ in 0..MAX_LINKS {
        match std::fs::symlink_metadata(&p) {
            Ok(m) if m.file_type().is_symlink() => {
                let to = std::fs::read_link(&p)?;
                p = match p.parent() {
                    Some(dir) if to.is_relative() => dir.join(to),
                    _ => to,
                };
            }
            _ => return Ok(p),
        }
    }
    Err(io::Error::new(io::ErrorKind::InvalidInput, format!("{}: too many levels of symbolic links", path.display())))
}

/// A rename replaces a file the user can't write (Unix only needs the folder to be writable);
/// refuse, as writing it in place would. Opening for writing without truncating changes nothing.
#[cfg(unix)]
fn check_writable(target: &Path) -> io::Result<()> {
    OpenOptions::new().write(true).open(target).map(drop)
}

/// Windows refuses to rename over a read-only file by itself.
#[cfg(not(unix))]
fn check_writable(_: &Path) -> io::Result<()> {
    Ok(())
}

/// The replacement keeps the replaced file's mode (Unix; Windows files keep only a read-only flag,
/// and a read-only file can't be replaced).
#[cfg(unix)]
fn keep_permissions(tmp: &File, replaced: &std::fs::Metadata) -> io::Result<()> {
    tmp.set_permissions(replaced.permissions())
}

#[cfg(not(unix))]
fn keep_permissions(_: &File, _: &std::fs::Metadata) -> io::Result<()> {
    Ok(())
}

/// Run `rename` up to `tries` times, pausing with a doubling delay while it fails with
/// `PermissionDenied` (a scanner holding the file on Windows). Other errors return at once.
fn retry_rename(tries: u32, mut rename: impl FnMut() -> io::Result<()>) -> io::Result<()> {
    let mut delay = RENAME_FIRST_DELAY_MS;
    let mut left = tries.max(1);
    loop {
        left = left.saturating_sub(1);
        match rename() {
            Err(e) if e.kind() == io::ErrorKind::PermissionDenied && left > 0 => {
                std::thread::sleep(std::time::Duration::from_millis(delay));
                delay = delay.saturating_mul(2).min(RENAME_MAX_DELAY_MS);
            }
            r => return r,
        }
    }
}

/// Flush a folder's entries (Unix); Windows can't open a folder as a file, and needs no flush.
#[cfg(unix)]
fn sync_dir(dir: &Path) -> io::Result<()> {
    File::open(dir)?.sync_all()
}

#[cfg(not(unix))]
fn sync_dir(_: &Path) -> io::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("dc-atomic-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// The names in `d`, sorted.
    fn names(d: &Path) -> Vec<String> {
        let mut v: Vec<String> = std::fs::read_dir(d).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
        v.sort();
        v
    }

    #[test]
    fn replaces_and_creates_without_leftovers() {
        let d = dir("replace");
        let path = d.join("doc.designcraft");
        write_atomic(&path, b"first").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"first");
        write_atomic(&path, b"second, longer").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"second, longer");
        write_atomic(&path, b"3").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"3");
        assert_eq!(names(&d), ["doc.designcraft"], "no temporary file is left");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn a_failed_write_leaves_the_original_and_no_temporary_file() {
        let d = dir("fail");
        let path = d.join("doc.designcraft");
        std::fs::write(&path, b"original").unwrap();
        let r = write_atomic_with(&path, |f| {
            f.write_all(b"half a docu")?;
            Err(io::Error::other("disk full"))
        });
        assert_eq!(r.unwrap_err().to_string(), "disk full");
        assert_eq!(std::fs::read(&path).unwrap(), b"original");
        assert_eq!(names(&d), ["doc.designcraft"], "the temporary file is removed");
        // A folder that doesn't exist: an error, nothing written.
        assert!(write_atomic(&d.join("missing").join("x.designcraft"), b"x").is_err());
        assert_eq!(names(&d), ["doc.designcraft"]);
        // A folder in the way: an error, the folder stays and nothing is left beside it.
        std::fs::create_dir(d.join("folder")).unwrap();
        assert!(write_atomic(&d.join("folder"), b"x").is_err());
        assert_eq!(names(&d), ["doc.designcraft", "folder"]);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn the_rename_is_retried_only_while_access_is_denied() {
        let mut calls = 0;
        let r = retry_rename(3, || {
            calls += 1;
            if calls < 3 { Err(io::Error::from(io::ErrorKind::PermissionDenied)) } else { Ok(()) }
        });
        assert!(r.is_ok() && calls == 3);
        calls = 0;
        let r = retry_rename(3, || {
            calls += 1;
            Err(io::Error::from(io::ErrorKind::PermissionDenied))
        });
        assert_eq!(r.map_err(|e| e.kind()), Err(io::ErrorKind::PermissionDenied));
        assert_eq!(calls, 3, "gives up after the last try");
        calls = 0;
        let r = retry_rename(3, || {
            calls += 1;
            Err(io::Error::from(io::ErrorKind::CrossesDevices))
        });
        assert!(r.is_err() && calls == 1, "other errors are not retried");
    }

    #[cfg(unix)]
    #[test]
    fn permissions_are_kept_and_unwritable_files_are_not_replaced() {
        use std::os::unix::fs::PermissionsExt as _;
        let d = dir("perms");
        let path = d.join("private.designcraft");
        std::fs::write(&path, b"old").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        write_atomic(&path, b"new").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"new");
        assert_eq!(std::fs::metadata(&path).unwrap().permissions().mode() & 0o777, 0o600);
        // A read-only file stays as it is (root may write it anyway).
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o400)).unwrap();
        if OpenOptions::new().write(true).open(&path).is_err() {
            assert!(write_atomic(&path, b"newer").is_err());
            assert_eq!(std::fs::read(&path).unwrap(), b"new");
            assert_eq!(names(&d), ["private.designcraft"]);
        }
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        let _ = std::fs::remove_dir_all(&d);
    }

    /// A folder the user can't write: the save fails and the original stays.
    #[cfg(unix)]
    #[test]
    fn an_unwritable_folder_fails_without_touching_the_file() {
        use std::os::unix::fs::PermissionsExt as _;
        let d = dir("ro-dir");
        let path = d.join("doc.designcraft");
        std::fs::write(&path, b"original").unwrap();
        std::fs::set_permissions(&d, std::fs::Permissions::from_mode(0o555)).unwrap();
        // Root writes anyway; the check only means something for an ordinary user.
        let probe = d.join("probe");
        let unwritable = std::fs::write(&probe, b"").is_err();
        if unwritable {
            assert!(write_atomic(&path, b"new").is_err());
            assert_eq!(std::fs::read(&path).unwrap(), b"original");
        }
        std::fs::set_permissions(&d, std::fs::Permissions::from_mode(0o755)).unwrap();
        let _ = std::fs::remove_file(&probe);
        assert_eq!(names(&d), ["doc.designcraft"]);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[cfg(unix)]
    #[test]
    fn a_symbolic_link_keeps_pointing_at_the_replaced_file() {
        let d = dir("link");
        std::fs::create_dir(d.join("real")).unwrap();
        let target = d.join("real").join("doc.designcraft");
        std::fs::write(&target, b"old").unwrap();
        let link = d.join("link.designcraft");
        std::os::unix::fs::symlink("real/doc.designcraft", &link).unwrap();
        write_atomic(&link, b"new").unwrap();
        assert!(std::fs::symlink_metadata(&link).unwrap().file_type().is_symlink());
        assert_eq!(std::fs::read(&target).unwrap(), b"new");
        assert_eq!(names(&d), ["link.designcraft", "real"]);
        assert_eq!(names(&d.join("real")), ["doc.designcraft"]);
        // A loop of links is an error, not a hang.
        std::os::unix::fs::symlink("b", d.join("a")).unwrap();
        std::os::unix::fs::symlink("a", d.join("b")).unwrap();
        assert!(write_atomic(&d.join("a"), b"x").is_err());
        let _ = std::fs::remove_dir_all(&d);
    }
}
