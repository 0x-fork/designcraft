//! PDF/X-1a:2003: the PDF/X-4 print file flattened by craft-prepress (no transparency left; RGB
//! and ICC color converted into the output intent's CMYK; inks, overprint and live text kept),
//! then identified as PDF 1.4 and PDF/X-1a, and checked.

use std::collections::HashSet;

use craft_prepress::flatten::pdf::ReadOptions;
use craft_prepress::flatten::{FlattenDocumentOptions, FlattenOptions, flatten_document};
use hayro_syntax::content::UntypedIter;
use hayro_syntax::object::{Array, Dict, Name, Object, ObjectIdentifier, Stream};
use hayro_syntax::page::Resources;

use crate::PdfError;
use crate::pdfx::{MAX_DEPTH, cmyk_output_intent};
use crate::transitions::{find, int_after, object, rfind};

/// `GTS_PDFXVersion` of PDF/X-1a:2003.
const VERSION: &str = "PDF/X-1:2003";
/// `GTS_PDFXConformance` of PDF/X-1a:2003.
const CONFORMANCE: &str = "PDF/X-1a:2003";

/// The PDF/X-1a file for `pdf`, the PDF/X-4 print file: flattened at `dpi`, identified, checked.
/// Fails when the flattener fails or the result still has transparency or color outside
/// DeviceCMYK, DeviceGray, Separation and DeviceN; other check findings become warnings.
pub(crate) fn flatten_x1a(pdf: &[u8], dpi: f32, date: Option<i64>, warnings: &mut Vec<String>) -> crate::Result<Vec<u8>> {
    let dpi = if dpi.is_finite() { dpi.clamp(36.0, 2400.0) } else { 300.0 };
    let opts = FlattenDocumentOptions {
        read: ReadOptions { raster_dpi: dpi, ..ReadOptions::default() },
        flatten: FlattenOptions { raster_dpi: dpi, ..FlattenOptions::default() },
        producer: "DesignCraft".into(),
        ..FlattenDocumentOptions::default()
    };
    let (flat, report) = flatten_document(pdf, &opts).map_err(|e| PdfError::Write(format!("PDF/X-1a: the transparency flattener failed: {e}")))?;
    let t = &report.totals;
    if t.complex_regions > 0 || t.complex_groups > 0 {
        let at = t.lowest_dpi.map(|d| format!(", some below {dpi:.0} ppi (lowest {d:.0} ppi)")).unwrap_or_default();
        warnings.push(format!("PDF/X-1a: transparency flattened: {} region(s) rasterized at {dpi:.0} ppi{at}", t.complex_regions + t.complex_groups));
    }
    if t.unreadable_images > 0 {
        warnings.push(format!("PDF/X-1a: {} image(s) couldn't be read by the flattener and were left out", t.unreadable_images));
    }
    if t.dropped_inks > 0 {
        warnings.push(format!("PDF/X-1a: {} ink(s) past the flattener's limit were converted to process", t.dropped_inks));
    }
    let bytes = make_pdfx1a(&flat, date).ok_or_else(|| PdfError::Write("PDF/X-1a: the flattened file couldn't be identified as PDF/X-1a".into()))?;
    let (fatal, other) = checks(&bytes);
    if !fatal.is_empty() {
        return Err(PdfError::Write(format!("PDF/X-1a: after flattening, {}", fatal.join("; "))));
    }
    warnings.extend(other.into_iter().map(|i| format!("PDF/X-1a: {i}")));
    Ok(bytes)
}

/// Identify a flattened PDF as PDF/X-1a:2003: the `%PDF-1.4` header, `GTS_PDFXVersion` and
/// `GTS_PDFXConformance` in the document information and the XMP metadata (with
/// `pdf:PDFVersion` 1.4, `xmp:MetadataDate` and `xmpMM:VersionID`), as an incremental update.
/// `date` (Unix seconds, UTC) is the metadata date. `None` back when the file isn't the simple
/// kind this understands (a classic cross-reference table, an information dictionary, an
/// uncompressed XMP stream) or is newer than PDF 1.7.
pub fn make_pdfx1a(pdf: &[u8], date: Option<i64>) -> Option<Vec<u8>> {
    // Same length, so no offset moves.
    if !pdf.starts_with(b"%PDF-1.") || !pdf.get(7).is_some_and(|v| (b'3'..=b'7').contains(v)) {
        return None;
    }
    let t_at = rfind(pdf, b"trailer")?;
    let trailer = String::from_utf8_lossy(pdf.get(t_at..)?).to_string();
    let size = int_after(&trailer, "/Size")?;
    let root = int_after(&trailer, "/Root")?;
    let prev = int_after(&trailer, "startxref")?;
    let info = int_after(&trailer, "/Info")?;
    let id = trailer.find("/ID").and_then(|i| trailer.get(i..)?.find(']').and_then(|j| trailer.get(i..i + j + 1)).map(str::to_string));
    let info_dict = String::from_utf8_lossy(pdf.get(object(pdf, info)?)?).to_string();
    let close = info_dict.rfind(">>")?;
    let mut info_body = info_dict.get(..close)?.trim().to_string();
    for key in ["/GTS_PDFXVersion", "/GTS_PDFXConformance"] {
        if info_body.contains(key) {
            return None;
        }
    }
    info_body.push_str(&format!("/GTS_PDFXVersion({VERSION})/GTS_PDFXConformance({CONFORMANCE})>>"));
    let mut objects = vec![(info, info_body)];
    let cat = String::from_utf8_lossy(pdf.get(object(pdf, root)?)?).to_string();
    if let Some(meta) = int_after(&cat, "/Metadata") {
        let body = pdf.get(object(pdf, meta)?)?;
        let s0 = find(body, b"stream", 0)? + b"stream".len();
        let s0 = s0 + body.get(s0..)?.iter().take_while(|c| **c == b'\r' || **c == b'\n').count();
        let s1 = rfind(body, b"endstream")?;
        let xml = String::from_utf8_lossy(body.get(s0..s1)?).trim_end().to_string();
        let dict = String::from_utf8_lossy(body.get(..find(body, b"stream", 0)?)?).to_string();
        if dict.contains("/Filter") || !xml.contains("</rdf:RDF>") {
            return None;
        }
        let (y, mo, d, h, mi, se) = crate::export::civil(date.unwrap_or(0));
        let stamp = format!("{y:04}-{mo:02}-{d:02}T{h:02}:{mi:02}:{se:02}Z");
        let added = format!(
            "<rdf:Description rdf:about=\"\" xmlns:pdfxid=\"http://www.npes.org/pdfx/ns/id/\" xmlns:pdfx=\"http://ns.adobe.com/pdfx/1.3/\">\
             <pdfxid:GTS_PDFXVersion>{VERSION}</pdfxid:GTS_PDFXVersion><pdfx:GTS_PDFXVersion>{VERSION}</pdfx:GTS_PDFXVersion>\
             <pdfx:GTS_PDFXConformance>{CONFORMANCE}</pdfx:GTS_PDFXConformance></rdf:Description>\
             <rdf:Description rdf:about=\"\" xmlns:xmp=\"http://ns.adobe.com/xap/1.0/\" xmlns:xmpMM=\"http://ns.adobe.com/xap/1.0/mm/\">\
             <xmp:MetadataDate>{stamp}</xmp:MetadataDate><xmpMM:VersionID>1</xmpMM:VersionID></rdf:Description>"
        );
        let mut xml = xml;
        for v in ["1.5", "1.6", "1.7"] {
            xml = xml.replace(&format!("<pdf:PDFVersion>{v}</pdf:PDFVersion>"), "<pdf:PDFVersion>1.4</pdf:PDFVersion>");
            xml = xml.replace(&format!("pdf:PDFVersion=\"{v}\""), "pdf:PDFVersion=\"1.4\"");
        }
        let xml = xml.replacen("</rdf:RDF>", &format!("{added}</rdf:RDF>"), 1);
        let i = dict.find("/Length")?;
        let rest = dict.get(i + 7..)?.trim_start();
        let end = rest.find(|c: char| !c.is_ascii_digit()).unwrap_or(rest.len());
        let dict = format!("{}/Length {}{}", dict.get(..i)?, xml.len(), rest.get(end..)?);
        objects.push((meta, format!("{}\nstream\n{xml}\nendstream", dict.trim())));
    }
    let mut out = pdf.to_vec();
    out.get_mut(..8)?.copy_from_slice(b"%PDF-1.4");
    if !out.ends_with(b"\n") {
        out.push(b'\n');
    }
    objects.sort_by_key(|(n, _)| *n);
    let mut xref = String::from("xref\n");
    for (n, body) in &objects {
        xref.push_str(&format!("{n} 1\n{:010} 00000 n\r\n", out.len()));
        out.extend_from_slice(format!("{n} 0 obj\n{body}\nendobj\n").as_bytes());
    }
    let xref_at = out.len();
    let id = id.unwrap_or_default();
    out.extend_from_slice(xref.as_bytes());
    out.extend_from_slice(
        format!("trailer\n<</Size {size}/Root {root} 0 R/Info {info} 0 R{id}/Prev {prev}>>\nstartxref\n{xref_at}\n%%EOF\n").as_bytes(),
    );
    Some(out)
}

/// PDF/X-1a:2003 checks on an exported file (empty = passes them): PDF 1.4 or lower, the
/// identification, a CMYK `GTS_PDFX` output intent, `/Trapped` True or False, trim boxes, no
/// encryption or interpolation; no transparency (soft masks, transparency groups, constant
/// opacity below 1, blend modes other than Normal); only DeviceCMYK, DeviceGray, Separation and
/// DeviceN (and Indexed and Pattern spaces over them), a spot's alternate space process; every
/// font embedded and none Type 3.
pub fn check_pdfx1a(pdf: &[u8]) -> Vec<String> {
    let (mut issues, other) = checks(pdf);
    issues.extend(other);
    issues
}

/// [`check_pdfx1a`]'s findings: transparency and color spaces first, then the rest.
fn checks(pdf: &[u8]) -> (Vec<String>, Vec<String>) {
    let mut issues = Vec::new();
    let text = String::from_utf8_lossy(pdf);
    if !pdf.starts_with(b"%PDF-1.") || pdf.get(7).is_none_or(|v| *v > b'4') {
        issues.push("the PDF version must be 1.4 or lower".into());
    }
    if !text.contains("/S/GTS_PDFX") {
        issues.push("no PDF/X output intent".into());
    }
    if !text.contains(&format!("/GTS_PDFXVersion({VERSION})")) {
        issues.push(format!("the document information doesn't name {VERSION}"));
    }
    if !text.contains(&format!("/GTS_PDFXConformance({CONFORMANCE})")) {
        issues.push(format!("the document information doesn't name the conformance {CONFORMANCE}"));
    }
    if !text.contains("/Trapped/False") && !text.contains("/Trapped/True") && !text.contains("/Trapped /False") && !text.contains("/Trapped /True") {
        issues.push("the document information needs /Trapped True or False".into());
    }
    if text.contains("/Interpolate true") {
        issues.push("an image asks for interpolation".into());
    }
    if text.contains("/Encrypt") {
        issues.push("the file is encrypted".into());
    }
    let pages = text.matches("/Type/Page/").count() + text.matches("/Type/Page>").count();
    let trims = text.matches("/TrimBox").count();
    if pages > 0 && trims < pages {
        issues.push("every page needs a trim box".into());
    }
    let (content, fonts) = content(pdf);
    issues.extend(fonts);
    (content, issues)
}

/// Transparency and color spaces, then fonts, walked through every page's content and what it
/// uses.
fn content(pdf: &[u8]) -> (Vec<String>, Vec<String>) {
    let Ok(file) = hayro_syntax::Pdf::new(pdf.to_vec()) else {
        return (vec!["the file couldn't be read to check its transparency and color spaces".into()], Vec::new());
    };
    let mut fonts = Vec::new();
    if !cmyk_output_intent(&file) {
        fonts.push("the output intent has no CMYK destination profile".into());
    }
    let mut found: Vec<(String, Vec<usize>)> = Vec::new();
    let mut font_found: Vec<(String, Vec<usize>)> = Vec::new();
    for (i, page) in file.pages().iter().enumerate() {
        let mut scan = Scan::default();
        let res = page.resources();
        if let Some(group) = page.raw().get::<Dict<'_>>(b"Group") {
            scan.group(&group);
        }
        if let Some(content) = page.page_stream() {
            scan.content(content, res, 0);
        }
        for (list, items) in [(&mut found, scan.found), (&mut font_found, scan.font_issues)] {
            for what in items {
                match list.iter_mut().find(|(w, _)| *w == what) {
                    Some((_, pages)) => pages.push(i + 1),
                    None => list.push((what, vec![i + 1])),
                }
            }
        }
    }
    let on_pages = |(what, pages): (String, Vec<usize>)| {
        let list: Vec<String> = pages.iter().take(10).map(|p| p.to_string()).collect();
        let more = if pages.len() > 10 { ", …" } else { "" };
        let s = if pages.len() == 1 { "" } else { "s" };
        format!("{what} on page{s} {}{more}", list.join(", "))
    };
    fonts.extend(font_found.into_iter().map(on_pages));
    (found.into_iter().map(on_pages).collect(), fonts)
}

/// Why a color space isn't allowed in PDF/X-1a, or `None` when it is.
fn disallowed(cs: &Object<'_>, res: &Resources<'_>, depth: usize) -> Option<String> {
    if depth > 4 {
        return Some("a color space nested too deeply".into());
    }
    match cs {
        Object::Name(n) => match n.as_ref() {
            b"DeviceCMYK" | b"CMYK" | b"DeviceGray" | b"G" | b"Pattern" => None,
            b"DeviceRGB" | b"RGB" => Some("DeviceRGB color".into()),
            b"Indexed" | b"I" => None,
            other => match res.get_color_space(&Name::new_unescaped(other)) {
                Some(named) if !matches!(named, Object::Name(_)) => disallowed(&named, res, depth + 1),
                Some(Object::Name(m)) if m.as_ref() != other => disallowed(&Object::Name(m), res, depth + 1),
                _ => Some(format!("the color space /{}", String::from_utf8_lossy(other))),
            },
        },
        Object::Array(a) => {
            let mut items = a.iter::<Object<'_>>();
            let family = match items.next() {
                Some(Object::Name(n)) => n,
                _ => return Some("an unreadable color space".into()),
            };
            match family.as_ref() {
                b"ICCBased" => Some("ICC-based color".into()),
                b"CalRGB" | b"CalGray" => Some("CIE-based (CalRGB/CalGray) color".into()),
                b"Lab" => Some("Lab color".into()),
                b"DeviceRGB" | b"RGB" => Some("DeviceRGB color".into()),
                b"DeviceCMYK" | b"CMYK" | b"DeviceGray" | b"G" => None,
                b"Indexed" | b"I" => items.next().and_then(|base| disallowed(&base, res, depth + 1)),
                b"Pattern" => items.next().and_then(|base| disallowed(&base, res, depth + 1)),
                b"Separation" | b"DeviceN" => {
                    let _names = items.next();
                    let alt = items.next()?;
                    disallowed(&alt, res, depth + 1).map(|why| format!("a spot color whose alternate space is {why}"))
                }
                other => Some(format!("the color space /{}", String::from_utf8_lossy(other))),
            }
        }
        _ => Some("an unreadable color space".into()),
    }
}

/// What one page has that PDF/X-1a doesn't allow, in the order found.
#[derive(Default)]
struct Scan {
    found: Vec<String>,
    font_issues: Vec<String>,
    seen: HashSet<ObjectIdentifier>,
    fonts: HashSet<ObjectIdentifier>,
}

impl Scan {
    fn add(&mut self, what: String) {
        if !self.found.contains(&what) {
            self.found.push(what);
        }
    }

    fn space(&mut self, cs: &Object<'_>, res: &Resources<'_>) {
        if let Some(why) = disallowed(cs, res, 0) {
            self.add(why);
        }
    }

    fn content(&mut self, data: &[u8], res: &Resources<'_>, depth: usize) {
        if depth > MAX_DEPTH {
            return;
        }
        let mut ops = UntypedIter::new(data);
        while let Some(op) = ops.next() {
            let operator: &[u8] = op.operator;
            let last = op.operands().last().cloned();
            let name = || last.clone().and_then(Object::into_name);
            match operator {
                b"rg" | b"RG" => self.add("DeviceRGB color".into()),
                b"cs" | b"CS" => {
                    if let Some(n) = name() {
                        self.space(&Object::Name(n), res);
                    }
                }
                b"sh" => {
                    if let Some(sh) = name().and_then(|n| res.get_shading(&n)) {
                        self.shading(sh, res);
                    }
                }
                b"scn" | b"SCN" => {
                    if let Some(p) = name().and_then(|n| res.get_pattern(&n)) {
                        self.pattern(p, res, depth);
                    }
                }
                b"gs" => {
                    if let Some(gs) = name().and_then(|n| res.get_ext_g_state(&n)) {
                        self.ext_g_state(&gs, res, depth);
                    }
                }
                b"Tf" => {
                    let font = op.operands().next().cloned().and_then(Object::into_name).and_then(|n| res.get_font(&n));
                    if let Some(font) = font {
                        self.font(&font);
                    }
                }
                b"Do" => {
                    if let Some(xo) = name().and_then(|n| res.get_x_object(&n)) {
                        match xo.dict().get::<Name<'_>>(b"Subtype").as_deref() {
                            Some(b"Image") => self.image(&xo, res),
                            Some(b"Form") => self.form(&xo, res, depth),
                            _ => {}
                        }
                    }
                }
                b"BI" | b"EI" => {
                    if let Some(Object::Stream(img)) = last
                        && let Some(cs) = img.dict().get::<Object<'_>>(b"CS").or_else(|| img.dict().get::<Object<'_>>(b"ColorSpace"))
                    {
                        self.space(&cs, res);
                    }
                }
                _ => {}
            }
        }
    }

    fn ext_g_state(&mut self, gs: &Dict<'_>, res: &Resources<'_>, depth: usize) {
        match gs.get::<Object<'_>>(b"SMask") {
            None => {}
            Some(Object::Name(n)) if n.as_ref() == b"None" => {}
            Some(mask) => {
                self.add("a soft mask (transparency)".into());
                if let Some(form) = match mask {
                    Object::Dict(d) => d.get::<Stream<'_>>(b"G"),
                    _ => None,
                } {
                    self.form(&form, res, depth);
                }
            }
        }
        for key in [&b"CA"[..], b"ca"] {
            if gs.get::<f32>(key).is_some_and(|a| a < 1.0) {
                self.add("constant opacity below 100% (transparency)".into());
            }
        }
        let normal = |n: &[u8]| matches!(n, b"Normal" | b"Compatible");
        match gs.get::<Object<'_>>(b"BM") {
            Some(Object::Name(n)) if !normal(n.as_ref()) => {
                self.add(format!("the blend mode /{} (transparency)", String::from_utf8_lossy(n.as_ref())))
            }
            Some(Object::Array(a)) if a.iter::<Name<'_>>().any(|n| !normal(n.as_ref())) => {
                self.add("a blend mode other than Normal (transparency)".into())
            }
            _ => {}
        }
    }

    fn image(&mut self, img: &Stream<'_>, res: &Resources<'_>) {
        if !self.seen.insert(img.obj_id()) {
            return;
        }
        let d = img.dict();
        if d.get::<Object<'_>>(b"SMask").is_some() {
            self.add(format!("an image with a soft mask (object {}, transparency)", img.obj_id().obj_number));
        }
        if d.get::<i32>(b"SMaskInData").is_some_and(|v| v != 0) {
            self.add(format!("an image with transparency in its data (object {})", img.obj_id().obj_number));
        }
        if d.get::<bool>(b"ImageMask") == Some(true) {
            return;
        }
        match d.get::<Object<'_>>(b"ColorSpace") {
            Some(cs) => {
                if let Some(why) = disallowed(&cs, res, 0) {
                    let (w, h) = (d.get::<i32>(b"Width").unwrap_or(0), d.get::<i32>(b"Height").unwrap_or(0));
                    self.add(format!("{why} in an image (object {}, {w} × {h} px)", img.obj_id().obj_number));
                }
            }
            // JPX images may carry their own space; X-1a doesn't allow them.
            None => self.add(format!("an image without a color space (object {})", img.obj_id().obj_number)),
        }
    }

    fn form(&mut self, form: &Stream<'_>, parent: &Resources<'_>, depth: usize) {
        if depth >= MAX_DEPTH || !self.seen.insert(form.obj_id()) {
            return;
        }
        let res = Resources::from_parent(form.dict().get::<Dict<'_>>(b"Resources").unwrap_or_default(), parent.clone());
        if let Some(group) = form.dict().get::<Dict<'_>>(b"Group") {
            self.group(&group);
        }
        if let Ok(data) = form.decoded() {
            self.content(&data, &res, depth + 1);
        }
    }

    fn group(&mut self, group: &Dict<'_>) {
        if group.get::<Name<'_>>(b"S").is_some_and(|s| s.as_ref() == b"Transparency") {
            self.add("a transparency group".into());
        }
    }

    fn shading(&mut self, sh: Object<'_>, res: &Resources<'_>) {
        let dict = match sh {
            Object::Dict(d) => d,
            Object::Stream(s) => s.dict().clone(),
            _ => return,
        };
        if let Some(cs) = dict.get::<Object<'_>>(b"ColorSpace") {
            self.space(&cs, res);
        }
    }

    fn pattern(&mut self, p: Object<'_>, res: &Resources<'_>, depth: usize) {
        match p {
            Object::Dict(d) => {
                if let Some(sh) = d.get::<Object<'_>>(b"Shading") {
                    self.shading(sh, res);
                }
                if let Some(gs) = d.get::<Dict<'_>>(b"ExtGState") {
                    self.ext_g_state(&gs, res, depth);
                }
            }
            Object::Stream(s) => self.form(&s, res, depth),
            _ => {}
        }
    }

    /// Embedded and not Type 3: a simple font's descriptor, or a Type 0 font's descendant's,
    /// has a font file.
    fn font(&mut self, font: &Dict<'_>) {
        if let Some(id) = font.obj_id()
            && !self.fonts.insert(id)
        {
            return;
        }
        let name = font.get::<Name<'_>>(b"BaseFont").map(|n| String::from_utf8_lossy(n.as_ref()).to_string()).unwrap_or_else(|| "?".into());
        let subtype = font.get::<Name<'_>>(b"Subtype");
        let descriptor = match subtype.as_deref() {
            Some(b"Type3") => return self.font_issues.push(format!("the Type 3 font {name}")),
            Some(b"Type0") => {
                font.get::<Array<'_>>(b"DescendantFonts").and_then(|a| a.iter::<Dict<'_>>().next()).and_then(|d| d.get::<Dict<'_>>(b"FontDescriptor"))
            }
            _ => font.get::<Dict<'_>>(b"FontDescriptor"),
        };
        let embedded = descriptor.is_some_and(|d| [&b"FontFile"[..], b"FontFile2", b"FontFile3"].iter().any(|k| d.get::<Stream<'_>>(k).is_some()));
        if !embedded {
            self.font_issues.push(format!("the font {name} isn't embedded"));
        }
    }
}
