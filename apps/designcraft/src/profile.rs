//! Native settings and recovery paths, including isolated automation profiles and portable installs.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// A file with this name next to the executable makes the install portable: settings, logs and
/// recovery data are kept in [`PORTABLE_DATA_DIR`] beside it. The Windows portable zip ships one.
const PORTABLE_MARKER: &str = "portable.txt";
const PORTABLE_DATA_DIR: &str = "DesignCraftData";

pub fn config_dir() -> Option<PathBuf> {
    config_dir_with(std::env::consts::OS, |key| std::env::var_os(key), executable_dir().as_deref())
}

fn config_dir_with(os: &str, env: impl Fn(&str) -> Option<OsString>, exe_dir: Option<&Path>) -> Option<PathBuf> {
    if let Some(root) = root_with(&env, exe_dir) {
        // An explicitly empty override must not fall back to the user's profile.
        return (!root.is_empty()).then(|| PathBuf::from(root));
    }
    match os {
        "macos" => env("HOME").map(|p| PathBuf::from(p).join("Library/Application Support/DesignCraft")),
        "windows" => env("APPDATA").map(|p| PathBuf::from(p).join("DesignCraft")),
        _ => {
            env("XDG_CONFIG_HOME").map(PathBuf::from).or_else(|| env("HOME").map(|p| PathBuf::from(p).join(".config"))).map(|p| p.join("designcraft"))
        }
    }
}

/// The folder that holds settings and recovery data instead of the user's profile:
/// `DESIGNCRAFT_CONFIG_DIR` (empty: none at all), else a portable install's data folder.
fn root_with(env: impl Fn(&str) -> Option<OsString>, exe_dir: Option<&Path>) -> Option<OsString> {
    env("DESIGNCRAFT_CONFIG_DIR").or_else(|| exe_dir.and_then(portable_dir).map(PathBuf::into_os_string))
}

/// `<exe_dir>/DesignCraftData` when `exe_dir` holds [`PORTABLE_MARKER`]. A folder of that name is
/// no marker. On macOS `exe_dir` is `DesignCraft.app/Contents/MacOS`, which ships none.
fn portable_dir(exe_dir: &Path) -> Option<PathBuf> {
    exe_dir.join(PORTABLE_MARKER).is_file().then(|| exe_dir.join(PORTABLE_DATA_DIR))
}

fn executable_dir() -> Option<PathBuf> {
    std::env::current_exe().ok()?.parent().map(Path::to_path_buf)
}

pub fn recovery_dir() -> Option<PathBuf> {
    recovery_dir_with(root_with(|key| std::env::var_os(key), executable_dir().as_deref()), designcraft_engine::recovery::default_dir)
}

fn recovery_dir_with(root: Option<OsString>, default: impl FnOnce() -> Option<PathBuf>) -> Option<PathBuf> {
    match root {
        Some(root) => (!root.is_empty()).then(|| PathBuf::from(root).join("Recovery")),
        None => default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn environment(key: &str) -> Option<OsString> {
        match key {
            "HOME" => Some("/user".into()),
            "APPDATA" => Some("/roaming".into()),
            "XDG_CONFIG_HOME" => Some("/xdg-config".into()),
            _ => None,
        }
    }

    #[test]
    fn isolated_profile_overrides_every_platform_and_recovery() {
        for os in ["macos", "windows", "linux"] {
            let dir = config_dir_with(os, |key| if key == "DESIGNCRAFT_CONFIG_DIR" { Some("/isolated".into()) } else { environment(key) }, None);
            assert_eq!(dir, Some(PathBuf::from("/isolated")));
        }
        assert_eq!(recovery_dir_with(Some("/isolated".into()), || Some("/user/recovery".into())), Some(PathBuf::from("/isolated/Recovery")));
    }

    #[test]
    fn empty_override_never_reads_or_writes_the_default_profile() {
        assert_eq!(
            config_dir_with("macos", |key| { if key == "DESIGNCRAFT_CONFIG_DIR" { Some(OsString::new()) } else { environment(key) } }, None),
            None
        );
        assert_eq!(recovery_dir_with(Some(OsString::new()), || Some("/user/recovery".into())), None);
    }

    /// A folder standing in for the executable's; `marker` puts `portable.txt` in it, or not.
    fn install(name: &str, marker: impl FnOnce(&Path)) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("designcraft-profile-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        marker(&dir.join(PORTABLE_MARKER));
        dir
    }

    #[test]
    fn portable_marker_keeps_settings_and_recovery_next_to_the_executable() {
        let exe = install("portable", |marker| std::fs::write(marker, "").unwrap());
        for os in ["macos", "windows", "linux"] {
            assert_eq!(config_dir_with(os, environment, Some(&exe)), Some(exe.join("DesignCraftData")));
        }
        assert_eq!(
            recovery_dir_with(root_with(environment, Some(&exe)), || Some("/user/recovery".into())),
            Some(exe.join("DesignCraftData/Recovery"))
        );
        let _ = std::fs::remove_dir_all(&exe);
    }

    #[test]
    fn without_a_marker_file_the_platform_paths_stay() {
        let plain = install("plain", |_| {});
        let folder = install("marker-folder", |marker| std::fs::create_dir(marker).unwrap());
        for exe in [&plain, &folder] {
            assert_eq!(config_dir_with("windows", environment, Some(exe)), Some("/roaming/DesignCraft".into()));
            assert_eq!(recovery_dir_with(root_with(environment, Some(exe)), || Some("/existing/recovery".into())), Some("/existing/recovery".into()));
        }
        let _ = std::fs::remove_dir_all(&plain);
        let _ = std::fs::remove_dir_all(&folder);
    }

    #[test]
    fn isolated_profile_wins_over_a_portable_install() {
        let exe = install("override", |marker| std::fs::write(marker, "").unwrap());
        let isolated = |key: &str| if key == "DESIGNCRAFT_CONFIG_DIR" { Some("/isolated".into()) } else { environment(key) };
        assert_eq!(config_dir_with("windows", isolated, Some(&exe)), Some("/isolated".into()));
        assert_eq!(recovery_dir_with(root_with(isolated, Some(&exe)), || None), Some("/isolated/Recovery".into()));
        let empty = |key: &str| if key == "DESIGNCRAFT_CONFIG_DIR" { Some(OsString::new()) } else { environment(key) };
        assert_eq!(config_dir_with("windows", empty, Some(&exe)), None);
        assert_eq!(recovery_dir_with(root_with(empty, Some(&exe)), || Some("/user/recovery".into())), None);
        let _ = std::fs::remove_dir_all(&exe);
    }

    #[test]
    fn unset_override_preserves_existing_platform_paths() {
        assert_eq!(config_dir_with("macos", environment, None), Some("/user/Library/Application Support/DesignCraft".into()));
        assert_eq!(config_dir_with("windows", environment, None), Some("/roaming/DesignCraft".into()));
        assert_eq!(config_dir_with("linux", environment, None), Some("/xdg-config/designcraft".into()));
        assert_eq!(
            config_dir_with("linux", |key| if key == "HOME" { environment(key) } else { None }, None),
            Some("/user/.config/designcraft".into())
        );
        assert_eq!(recovery_dir_with(None, || Some("/existing/recovery".into())), Some("/existing/recovery".into()));
    }
}
