//! CMYK print output, read back from the exported PDF: blending spaces, the output intent,
//! overprint and knockout groups.

use designcraft_color::cms::{self, Cms, ColorSettings};
use designcraft_compose::Cache;
use designcraft_doc::build::NewDocument;
use designcraft_doc::{BlendSpace, Document, Fill, Item, ItemId, SpreadRef};
use designcraft_geom::Rect;
use hayro_syntax::Pdf;
use hayro_syntax::object::{Array, Dict, Name, Object, Stream};

use crate::{PdfOptions, export_pdf};

/// A box filled with `swatch`; `edit` sets its other attributes.
fn add_box(d: &mut Document, r: Rect, swatch: &str, edit: impl FnOnce(&mut Item)) -> ItemId {
    let lid = d.default_layer();
    let id = ItemId(d.alloc());
    let mut b = Item::new(id, lid, designcraft_doc::Shape::Rectangle, designcraft_geom::shapes::rectangle(r));
    b.fill = Fill::swatch(swatch);
    edit(&mut b);
    d.insert_item(SpreadRef::Doc(0), b, None).unwrap();
    id
}

/// One page: a cyan box with a 50% black box over it at 50% opacity.
fn transparent_doc() -> Document {
    let mut d = Document::new(&NewDocument::default());
    add_box(&mut d, Rect::new(36.0, 36.0, 236.0, 236.0), "C=100 M=0 Y=0 K=0", |_| {});
    add_box(&mut d, Rect::new(136.0, 136.0, 336.0, 336.0), "Black", |b| b.opacity = 0.5);
    d
}

pub(crate) struct Intent {
    pub identifier: String,
    pub condition: String,
    pub registry: Option<String>,
    pub profile: Vec<u8>,
}

fn text(d: &Dict<'_>, key: &[u8]) -> Option<String> {
    d.get::<hayro_syntax::object::String>(key).map(|s| String::from_utf8_lossy(s.as_bytes()).into_owned())
}

/// The catalog's `GTS_PDFX` output intent.
pub(crate) fn output_intent(bytes: &[u8]) -> Option<Intent> {
    let pdf = Pdf::new(bytes.to_vec()).ok()?;
    let xref = pdf.xref();
    let catalog = xref.get::<Dict<'_>>(xref.root_id())?;
    let intents = catalog.get::<Array<'_>>(b"OutputIntents")?;
    let oi = intents.iter::<Dict<'_>>().find(|oi| oi.get::<Name<'_>>(b"S").is_some_and(|s| s.as_ref() == b"GTS_PDFX"))?;
    Some(Intent {
        identifier: text(&oi, b"OutputConditionIdentifier")?,
        condition: text(&oi, b"OutputCondition").unwrap_or_default(),
        registry: text(&oi, b"RegistryName"),
        profile: oi.get::<Stream<'_>>(b"DestOutputProfile")?.decoded().ok()?.into_owned(),
    })
}

/// The `/CS` of each page's transparency group (`None` for a page without one).
fn page_group_spaces(bytes: &[u8]) -> Vec<Option<String>> {
    let pdf = Pdf::new(bytes.to_vec()).unwrap();
    pdf.pages()
        .iter()
        .map(|p| {
            let g = p.raw().get::<Dict<'_>>(b"Group")?;
            match g.get::<Object<'_>>(b"CS")? {
                Object::Name(n) => Some(n.as_str().to_string()),
                Object::Array(a) => a.iter::<Name<'_>>().next().map(|n| n.as_str().to_string()),
                _ => None,
            }
        })
        .collect()
}

/// Every form XObject's transparency group dictionary.
fn form_groups(bytes: &[u8]) -> Vec<(Option<String>, bool, bool)> {
    let pdf = Pdf::new(bytes.to_vec()).unwrap();
    pdf.objects()
        .into_iter()
        .filter_map(|o| o.into_stream())
        .filter(|s| s.dict().get::<Name<'_>>(b"Subtype").is_some_and(|n| n.as_ref() == b"Form"))
        .filter_map(|s| {
            let g = s.dict().get::<Dict<'_>>(b"Group")?;
            let cs = g.get::<Name<'_>>(b"CS").map(|n| n.as_str().to_string());
            Some((cs, g.get::<bool>(b"I").unwrap_or(false), g.get::<bool>(b"K").unwrap_or(false)))
        })
        .collect()
}

/// Print output (a CMYK blend space) blends in DeviceCMYK and carries the CMYK working space
/// as its output intent; a screen document (RGB blend space) is written as before.
#[test]
fn print_output_blends_in_cmyk_with_the_working_space_as_output_intent() {
    let mut d = transparent_doc();
    let bytes = export_pdf(&d, &Cache::new(), &PdfOptions::default()).unwrap();
    assert_eq!(page_group_spaces(&bytes), [Some("DeviceCMYK".to_string())]);
    assert!(form_groups(&bytes).iter().all(|(cs, ..)| cs.is_none()), "content groups inherit the page group: {:?}", form_groups(&bytes));
    assert!(!crate::has_rgb_groups(&bytes));
    let intent = output_intent(&bytes).expect("output intent");
    assert_eq!(intent.identifier, "FOGRA39");
    assert_eq!(intent.profile, craft_cms::builtin::COATED_FOGRA39_ICC);
    assert!(String::from_utf8_lossy(&bytes).contains("/Trapped/False"));
    // A booklet of a print document too.
    let booklet = crate::export_booklet(&d, &Cache::new(), &crate::BookletOptions::default()).unwrap();
    assert!(page_group_spaces(&booklet.bytes).contains(&Some("DeviceCMYK".to_string())));
    assert!(output_intent(&booklet.bytes).is_some());

    d.settings.blend_space = BlendSpace::Rgb;
    let screen = export_pdf(&d, &Cache::new(), &PdfOptions::default()).unwrap();
    assert!(output_intent(&screen).is_none());
    assert_eq!(page_group_spaces(&screen), [None]);
    assert!(!String::from_utf8_lossy(&screen).contains("/Trapped"));
}

/// The output intent is the CMYK working space's profile and condition, not FOGRA39.
#[test]
fn output_intent_is_the_cmyk_working_space() {
    use craft_pdf::write::backend as krilla;
    let generic = Cms::new(&ColorSettings { cmyk: cms::GENERIC_CMYK.into(), ..ColorSettings::default() }).unwrap();
    let (settings, warning) = crate::pdfx::print_settings_with(&generic).unwrap();
    assert_eq!(warning, None);
    let mut pdf = krilla::Document::new_with(settings);
    let mut page = pdf.start_page_with(krilla::page::PageSettings::new(krilla::geom::Size::from_wh(100.0, 100.0).unwrap()));
    page.surface().finish();
    page.finish();
    let bytes = pdf.finish().unwrap();
    let intent = output_intent(&bytes).expect("output intent");
    let want = generic.output_intent().unwrap();
    assert_eq!((intent.identifier.as_str(), intent.condition.as_str()), ("Custom", cms::GENERIC_CMYK));
    assert_eq!(intent.registry, None, "an unregistered condition names no registry");
    assert_eq!(intent.profile, &*want.icc);
    assert_ne!(intent.profile, craft_cms::builtin::COATED_FOGRA39_ICC);
}
