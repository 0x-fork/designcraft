use hayro_syntax::Pdf;
use hayro_syntax::object::{Array, Dict, MaybeRef, Name, Object, ObjectIdentifier, Stream};
use hayro_syntax::reader::{Reader, ReaderExt};
use std::sync::Arc;

use crate::{PdfError, Result};

const MARKER: &[u8] = b"DesignCraftHiddenOCG";

fn invalid() -> PdfError {
    PdfError::Write("the placed PDF's optional-content state could not be preserved safely".into())
}

fn references(dict: &Dict<'_>, key: &[u8]) -> Result<Vec<ObjectIdentifier>> {
    if !dict.contains_key(key) {
        return Ok(Vec::new());
    }
    dict.get::<Array<'_>>(key).ok_or_else(invalid)?.raw_iter().map(|value| value.as_obj_ref().map(Into::into).ok_or_else(invalid)).collect()
}

fn edited(dict: &Dict<'_>, replacements: &[(&[u8], &[u8])]) -> Result<Vec<u8>> {
    let data = dict.data();
    let body = data.strip_prefix(b"<<").and_then(|d| d.strip_suffix(b">>")).ok_or_else(invalid)?;
    let mut reader = Reader::new(body);
    let mut out = b"<<".to_vec();
    loop {
        reader.skip_white_spaces_and_comments();
        if reader.at_end() {
            break;
        }
        let start = reader.offset();
        let name = reader.read_without_context::<Name<'_>>().ok_or_else(invalid)?;
        reader.skip_white_spaces_and_comments();
        reader.skip::<MaybeRef<Object<'_>>>(false).ok_or_else(invalid)?;
        if !replacements.iter().any(|(key, _)| *key == name.as_ref()) {
            out.push(b'\n');
            out.extend_from_slice(body.get(start..reader.offset()).ok_or_else(invalid)?);
        }
    }
    for (key, value) in replacements {
        if !value.is_empty() {
            out.extend_from_slice(b"\n/");
            out.extend_from_slice(key);
            out.push(b' ');
            out.extend_from_slice(value);
        }
    }
    out.extend_from_slice(b"\n>>");
    Ok(out)
}

fn append(bytes: &[u8], changes: &[(ObjectIdentifier, Vec<u8>)], root: ObjectIdentifier) -> Result<Vec<u8>> {
    let damaged = || PdfError::Write("the placed PDF's cross-reference table is damaged; layer changes could not be preserved safely".into());
    let at = crate::transitions::rfind(bytes, b"startxref").ok_or_else(damaged)?;
    let tail = std::str::from_utf8(bytes.get(at..).ok_or_else(damaged)?).map_err(|_| damaged())?;
    let prev = crate::transitions::int_after(tail, "startxref").ok_or_else(damaged)?;
    let section = bytes.get(prev..).ok_or_else(damaged)?;
    let trailer = if section.starts_with(b"xref") {
        let at = crate::transitions::find(section, b"trailer", 0).ok_or_else(damaged)?;
        let mut reader = Reader::new(section.get(at + 7..).ok_or_else(damaged)?);
        reader.skip_white_spaces_and_comments();
        reader.read_without_context::<Dict<'_>>().ok_or_else(damaged)?
    } else {
        let mut reader = Reader::new(section);
        reader.read_without_context::<ObjectIdentifier>().ok_or_else(damaged)?;
        reader.skip_white_spaces_and_comments();
        let stream = reader.read_without_context::<Stream<'_>>().ok_or_else(damaged)?;
        if stream.dict().get::<Name<'_>>(b"Type").as_ref().map(AsRef::as_ref) != Some(b"XRef") {
            return Err(damaged());
        }
        stream.dict().clone()
    };
    if trailer.contains_key(b"Encrypt") {
        return Err(invalid());
    }
    trailer.get::<i32>(b"Size").filter(|n| *n > 0).ok_or_else(damaged)?;
    let mut out = bytes.to_vec();
    out.push(b'\n');
    let mut offsets = Vec::new();
    for (id, body) in changes {
        offsets.push((*id, out.len()));
        out.extend_from_slice(format!("{} {} obj\n", id.obj_number, id.gen_number).as_bytes());
        out.extend_from_slice(body);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref = out.len();
    out.extend_from_slice(b"xref\n");
    offsets.sort_by_key(|(id, _)| *id);
    for (id, offset) in offsets {
        out.extend_from_slice(format!("{} 1\n{offset:010} {:05} n\r\n", id.obj_number, id.gen_number).as_bytes());
    }
    let root = format!("{} {} R", root.obj_number, root.gen_number);
    let prev = prev.to_string();
    let trailer = edited(
        &trailer,
        &[
            (b"Root", root.as_bytes()),
            (b"Prev", prev.as_bytes()),
            (b"XRefStm", b""),
            (b"Type", b""),
            (b"W", b""),
            (b"Index", b""),
            (b"Length", b""),
            (b"Filter", b""),
            (b"DecodeParms", b""),
        ],
    )?;
    out.extend_from_slice(b"trailer\n");
    out.extend_from_slice(&trailer);
    out.extend_from_slice(format!("\nstartxref\n{xref}\n%%EOF\n").as_bytes());
    Ok(out)
}

#[derive(Clone)]
pub(crate) struct Variant {
    pub data: Arc<Vec<u8>>,
    pub hidden: Vec<ObjectIdentifier>,
    pub warning: Option<String>,
    pub has_optional_content: bool,
}

pub(crate) fn prepare(bytes: &Arc<Vec<u8>>, hidden: &[String]) -> Result<Variant> {
    let pdf = Pdf::new(bytes.clone()).map_err(|_| invalid())?;
    let xref = pdf.xref();
    let root = xref.get::<Dict<'_>>(xref.root_id()).ok_or_else(invalid)?;
    if !root.contains_key(b"OCProperties") {
        let has_optional_content = pdf
            .objects()
            .into_iter()
            .any(|object| matches!(object, Object::Dict(dict) if dict.get::<Name<'_>>(b"Type").as_ref().map(AsRef::as_ref) == Some(b"OCG")));
        return Ok(Variant { data: bytes.clone(), hidden: Vec::new(), warning: None, has_optional_content });
    }
    let properties = root.get::<Dict<'_>>(b"OCProperties").ok_or_else(invalid)?;
    let mut groups = references(&properties, b"OCGs")?;
    let config = properties.get::<Dict<'_>>(b"D");
    let mut fallback = config.is_none();
    let config = config.unwrap_or_default();
    let base_off = match config.get::<Name<'_>>(b"BaseState").as_ref().map(AsRef::as_ref) {
        None if !config.contains_key(b"BaseState") => false,
        Some(b"ON") => false,
        Some(b"OFF") => true,
        None | Some(_) => {
            fallback = true;
            false
        }
    };
    let off = references(&config, b"OFF")?;
    let on = match references(&config, b"ON") {
        Ok(on) => on,
        Err(error) if !fallback && !base_off && off.is_empty() => {
            let value = config.get::<Object<'_>>(b"ON").ok_or(error)?;
            fallback = true;
            value.into_array().map(|array| array.raw_iter().filter_map(|value| value.as_obj_ref().map(Into::into)).collect()).unwrap_or_default()
        }
        Err(error) => return Err(error),
    };
    if on.iter().any(|id| off.contains(id)) {
        return Err(invalid());
    }
    let incomplete = on.iter().chain(&off).any(|id| !groups.contains(id));
    groups.extend(on.iter().chain(&off).copied());
    groups.sort();
    groups.dedup();
    // Aggregating applications use the default state, not /AS usage-event adjustments.
    let mut changes = Vec::new();
    let mut effective = Vec::new();
    let mut renamed = false;
    for id in groups {
        let group = xref.get::<Dict<'_>>(id).ok_or_else(invalid)?;
        if group.get::<Name<'_>>(b"Type").as_ref().map(AsRef::as_ref) != Some(b"OCG") {
            return Err(invalid());
        }
        let name = group.get::<hayro_syntax::object::String<'_>>(b"Name");
        let is_hidden = match (hidden.is_empty(), name.as_ref()) {
            (true, _) => !fallback && (off.contains(&id) || (base_off && !on.contains(&id))),
            (false, Some(name)) => {
                let name = String::from_utf8_lossy(name.as_bytes());
                hidden.iter().any(|n| n == name.as_ref())
            }
            (false, None) => return Err(invalid()),
        };
        if is_hidden {
            effective.push(id);
        }
        // Unmarked groups are visible when registered in the output catalog.
        if !is_hidden && name.is_some() {
            continue;
        }
        let marker = is_hidden.to_string();
        let body = match name {
            Some(_) => edited(&group, &[(MARKER, marker.as_bytes())])?,
            None => {
                if group.contains_key(b"Name") && group.get::<Object<'_>>(b"Name").is_none() {
                    return Err(invalid());
                }
                renamed = true;
                let label = format!("(Placed PDF layer {} {})", id.obj_number, id.gen_number);
                edited(&group, &[(MARKER, marker.as_bytes()), (b"Name", label.as_bytes())])?
            }
        };
        changes.push((id, body));
    }
    effective.sort();
    effective.dedup();
    let data = if changes.is_empty() { bytes.clone() } else { Arc::new(append(bytes, &changes, xref.root_id())?) };
    let mut warnings = Vec::new();
    if fallback {
        warnings.push("layer state not readable, all layers shown unless hidden by Object Layer Options");
    }
    if incomplete {
        warnings.push("layer list incomplete; layers named in the default configuration were added");
    }
    if renamed {
        warnings.push("layer display name not readable; generated a name, visibility preserved");
    }
    let warning = if warnings.is_empty() { None } else { Some(warnings.join("; ")) };
    Ok(Variant { data, hidden: effective, warning, has_optional_content: true })
}

pub(crate) fn register(bytes: &[u8]) -> Result<Vec<u8>> {
    #[cfg(test)]
    crate::export::registration_tests::REGISTRATIONS.with(|count| count.set(count.get() + 1));
    let pdf = Pdf::new(bytes.to_vec()).map_err(|_| invalid())?;
    let mut groups = Vec::new();
    let mut changes = Vec::new();
    for object in pdf.objects() {
        let Object::Dict(dict) = object else { continue };
        if dict.get::<Name<'_>>(b"Type").as_ref().map(AsRef::as_ref) != Some(b"OCG") {
            continue;
        }
        // Resource-only OCGs without a catalog configuration are visible by default.
        let hidden = if dict.contains_key(MARKER) { dict.get::<bool>(MARKER).ok_or_else(invalid)? } else { false };
        let id = dict.obj_id().ok_or_else(invalid)?;
        groups.push((id, hidden));
        changes.push((id, edited(&dict, &[(MARKER, b"")])?));
    }
    if groups.is_empty() {
        return Ok(bytes.to_vec());
    }
    groups.sort_by_key(|(id, _)| *id);
    let refs = |hidden_only: bool| {
        groups
            .iter()
            .filter(|(_, hidden)| !hidden_only || *hidden)
            .map(|(id, _)| format!("{} {} R", id.obj_number, id.gen_number))
            .collect::<Vec<_>>()
            .join(" ")
    };
    let all = refs(false);
    let properties = format!("<</OCGs[{all}]/D<</Name(Placed PDF layers)/BaseState/ON/OFF[{}]/Order[{all}]>>>>", refs(true));
    let root = pdf.xref().root_id();
    let catalog = pdf.xref().get::<Dict<'_>>(root).ok_or_else(invalid)?;
    changes.push((root, edited(&catalog, &[(b"OCProperties", properties.as_bytes())])?));
    append(bytes, &changes, root)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visible_named_groups_keep_original_buffer_with_repaired_xref() {
        let bytes = Arc::new(
            b"%PDF-1.5\n1 0 obj\n<</Type/Catalog/Pages 2 0 R/OCProperties<</OCGs[4 0 R]/D<</BaseState/ON>>>>>>\nendobj\n\
              2 0 obj\n<</Type/Pages/Kids[3 0 R]/Count 1>>\nendobj\n\
              3 0 obj\n<</Type/Page/Parent 2 0 R/MediaBox[0 0 100 100]/Resources<<>>>>\nendobj\n\
              4 0 obj\n<</Type/OCG/Name(Visible)>>\nendobj\n\
              trailer\n<</Size 5/Root 1 0 R>>\nstartxref\n1\n%%EOF\n"
                .to_vec(),
        );
        assert!(Pdf::new(bytes.clone()).is_ok());

        let variant = prepare(&bytes, &[]).unwrap();

        assert!(Arc::ptr_eq(&bytes, &variant.data), "visible groups must not copy the complete PDF or require incremental writes");
        assert!(variant.hidden.is_empty());
        assert!(variant.warning.is_none());
    }
}
