//! Object Layer Options for placed PDFs: the file's optional-content layers, and a variant of the
//! file with some layers off (an appended catalog whose default configuration turns them off,
//! which the PDF renderer honours).

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use craft_pdf::syntax::Pdf;
use craft_pdf::syntax::object::{Array, Dict, MaybeRef, Name, ObjRef, Object, ObjectIdentifier};

/// (object reference, name, on by default) of each layer.
fn ocgs(pdf: &Pdf) -> Vec<(ObjRef, String, bool)> {
    let xref = pdf.xref();
    let Some(cat) = xref.get::<Dict<'_>>(xref.root_id()) else { return vec![] };
    let Some(ocp) = cat.get::<Dict<'_>>(b"OCProperties") else { return vec![] };
    let Some(list) = ocp.get::<Array<'_>>(b"OCGs") else { return vec![] };
    let cfg = ocp.get::<Dict<'_>>(b"D");
    let ids_in = |key: &[u8]| -> Vec<ObjRef> {
        cfg.as_ref().and_then(|c| c.get::<Array<'_>>(key)).map(|a| a.raw_iter().filter_map(|o| o.as_obj_ref()).collect()).unwrap_or_default()
    };
    let base_off = cfg.as_ref().and_then(|c| c.get::<Name<'_>>(b"BaseState")).is_some_and(|n| n.as_ref() == b"OFF");
    let (on, off) = (ids_in(b"ON"), ids_in(b"OFF"));
    list.raw_iter()
        .filter_map(|o| o.as_obj_ref())
        .filter_map(|r| {
            let d = xref.get::<Dict<'_>>(ObjectIdentifier::from(r))?;
            let name = d
                .get::<craft_pdf::syntax::object::String<'_>>(b"Name")
                .map(|s| String::from_utf8_lossy(s.as_bytes()).to_string())
                .unwrap_or_default();
            let visible = if base_off { on.contains(&r) } else { !off.contains(&r) };
            Some((r, name, visible))
        })
        .collect()
}

/// The layers of a placed PDF: (name, visible by default).
pub fn pdf_layers(bytes: &[u8]) -> Vec<(String, bool)> {
    craft_pdf::open_slice(bytes, None).map(|p| ocgs(&p).into_iter().map(|(_, n, v)| (n, v)).collect()).unwrap_or_default()
}

// Copy untouched dictionary entries as raw PDF tokens. In particular, do not reinterpret
// strings, numbers or unknown extension keys. The parser bounds container nesting.
fn patch_dict(dict: &Dict<'_>, updates: &[(&[u8], &[u8])]) -> Option<Vec<u8>> {
    use craft_pdf::syntax::reader::{Reader, ReaderExt};
    let data = if dict.is_empty() { b"<<>>".as_slice() } else { dict.data() };
    if data.len() > 4 * 1024 * 1024 {
        return None;
    }
    let mut reader = Reader::new(data);
    reader.forward_tag(b"<<")?;
    let mut out = b"<<".to_vec();
    let mut ended = false;
    for _ in 0..100_000 {
        reader.skip_white_spaces_and_comments();
        if reader.forward_tag(b">>").is_some() {
            ended = true;
            break;
        }
        let start = reader.offset();
        let name = reader.read_without_context::<Name<'_>>()?;
        reader.skip_white_spaces_and_comments();
        reader.skip::<MaybeRef<Object<'_>>>(false)?;
        if !updates.iter().any(|(key, _)| *key == name.as_ref()) {
            out.push(b' ');
            out.extend_from_slice(data.get(start..reader.offset())?);
        }
    }
    if !ended {
        return None;
    }
    for (key, value) in updates {
        if !value.is_empty() {
            out.extend_from_slice(b" /");
            out.extend_from_slice(key);
            out.push(b' ');
            out.extend_from_slice(value);
        }
    }
    out.extend_from_slice(b" >>");
    Some(out)
}

fn trailer_at(bytes: &[u8], offset: usize) -> Option<Dict<'_>> {
    use craft_pdf::syntax::reader::{Reader, ReaderExt};
    let data = bytes.get(offset..)?;
    let mut reader = Reader::new(data);
    if data.starts_with(b"xref") {
        let pos = data.get(..data.len().min(4 * 1024 * 1024))?.windows(7).position(|w| w == b"trailer")?;
        reader.skip_bytes(pos.checked_add(7)?)?;
    } else {
        // An xref stream's dictionary also holds the trailer entries.
        reader.read_without_context::<ObjectIdentifier>()?;
    }
    reader.skip_white_spaces_and_comments();
    reader.read_without_context::<Dict<'_>>()
}

/// `bytes` with exactly the layers named in `hidden` off (the others on).
/// Returns `None` for unreadable, encrypted, hybrid-reference or over-budget updates.
/// Untouched catalog/configuration entries and full object references are preserved.
pub fn pdf_hide_layers(bytes: &[u8], hidden: &[String]) -> Option<Vec<u8>> {
    crate::check_pdf_input_size(bytes.len()).ok()?;
    let pdf = craft_pdf::open_slice(bytes, None).ok()?;
    let layers = ocgs(&pdf);
    if layers.is_empty() {
        return None;
    }
    let tail = String::from_utf8_lossy(bytes.get(bytes.len().saturating_sub(1024)..)?);
    let prev = tail.rsplit_once("startxref")?.1.split_whitespace().next()?.parse::<usize>().ok()?;
    let trailer = trailer_at(bytes, prev)?;
    // Do not copy decrypted dictionaries into an encrypted file, or ignore a hybrid
    // xref table. Follow bounded Prev links because encryption can be inherited.
    let mut offset = Some(prev);
    for _ in 0..32 {
        let Some(at) = offset else { break };
        let t = trailer_at(bytes, at)?;
        if t.contains_key(b"Encrypt") || t.contains_key(b"XRefStm") {
            return None;
        }
        offset = t.get::<usize>(b"Prev");
        if offset.is_some_and(|n| n >= at) {
            return None;
        }
    }
    if offset.is_some() {
        return None;
    }
    let xref = pdf.xref();
    let root = xref.root_id();
    if root.obj_number <= 0 || !(0..=65535).contains(&root.gen_number) || trailer.get::<usize>(b"Size")? <= usize::try_from(root.obj_number).ok()? {
        return None;
    }
    let cat = xref.get::<Dict<'_>>(root)?;
    let ocp = cat.get::<Dict<'_>>(b"OCProperties")?;
    let cfg = ocp.get::<Dict<'_>>(b"D");
    let mut off = b"[".to_vec();
    for (id, name, _) in &layers {
        if hidden.contains(name) {
            off.extend_from_slice(format!("{} {} R ", id.obj_number, id.gen_number).as_bytes());
        }
    }
    off.push(b']');
    let empty = Dict::empty();
    let cfg = patch_dict(cfg.as_ref().unwrap_or(&empty), &[(b"BaseState", b"/ON"), (b"ON", b"[]"), (b"OFF", &off)])?;
    let ocp = patch_dict(&ocp, &[(b"D", &cfg)])?;
    let catalog = patch_dict(&cat, &[(b"OCProperties", &ocp)])?;
    let root_ref = format!("{} {} R", root.obj_number, root.gen_number);
    let prev_value = prev.to_string();
    let trailer = patch_dict(
        &trailer,
        &[
            (b"Root", root_ref.as_bytes()),
            (b"Prev", prev_value.as_bytes()),
            (b"Type", b""),
            (b"Length", b""),
            (b"Filter", b""),
            (b"DecodeParms", b""),
            (b"W", b""),
            (b"Index", b""),
        ],
    )?;
    let mut out = bytes.to_vec();
    out.push(b'\n');
    let at = out.len();
    // Redefine the existing catalog: no arbitrary fresh object number or Size increase.
    out.extend_from_slice(format!("{} {} obj\n", root.obj_number, root.gen_number).as_bytes());
    out.extend_from_slice(&catalog);
    out.extend_from_slice(b"\nendobj\n");
    let xref_at = out.len();
    out.extend_from_slice(format!("xref\n{} 1\n{at:010} {:05} n\r\ntrailer\n", root.obj_number, root.gen_number).as_bytes());
    out.extend_from_slice(&trailer);
    out.extend_from_slice(format!("\nstartxref\n{xref_at}\n%%EOF\n").as_bytes());
    Some(out)
}

/// Shared copies of layer variants (by source address and hidden set).
pub(crate) fn layered(data: &Arc<Vec<u8>>, hidden: &[String]) -> Arc<Vec<u8>> {
    type Key = (usize, Vec<String>);
    static CACHE: Mutex<Option<HashMap<Key, (Arc<Vec<u8>>, Arc<Vec<u8>>)>>> = Mutex::new(None);
    let key = (Arc::as_ptr(data) as usize, hidden.to_vec());
    let mut c = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    let map = c.get_or_insert_with(HashMap::new);
    if let Some(v) = map.get(&key) {
        return v.1.clone();
    }
    if map.len() > 64 {
        map.clear();
    }
    let v = pdf_hide_layers(data, hidden).map(Arc::new).unwrap_or_else(|| data.clone());
    map.insert(key, (data.clone(), v.clone()));
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use craft_pdf::syntax::reader::{Reader, ReaderExt};

    #[test]
    fn dictionary_patch_preserves_raw_values_and_bounds_updates() {
        let data = b"<</Unknown[(x) 12 1 R <ff>]/Base#53tate/OFF/Name(a(b)c)>>";
        let dict = Reader::new(data).read_without_context::<Dict<'_>>().unwrap();
        let out = patch_dict(&dict, &[(b"BaseState", b"/ON")]).unwrap();
        let text = std::str::from_utf8(&out).unwrap();
        assert!(text.contains("/Unknown[(x) 12 1 R <ff>]"));
        assert!(text.contains("/Name(a(b)c)"));
        assert!(!text.contains("/OFF"));
        assert!(text.contains("/BaseState /ON"));
        assert_eq!(patch_dict(&Dict::empty(), &[(b"BaseState", b"/ON")]).unwrap(), b"<< /BaseState /ON >>");
        let mut many = b"<< /Big (".to_vec();
        many.resize(4 * 1024 * 1024 + 1, b'x');
        many.extend_from_slice(b") >>");
        let dict = Reader::new(&many).read_without_context::<Dict<'_>>().unwrap();
        assert!(patch_dict(&dict, &[]).is_none());
    }
}
