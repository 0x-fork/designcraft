//! A CMYK transparency blend space on screen (Edit › Transparency Blend Space).
//!
//! InDesign blends a print document's transparency in CMYK: a blend mode works on each ink. So
//! Color Dodge with 100% K text turns what's under it to paper white where it has colour (black's
//! cyan, magenta and yellow are 0, their complements 1), while in RGB dodging with black changes
//! nothing. vello blends in RGB, so a spread whose top-level objects use separable blend modes is
//! composited here instead:
//! - the objects below each blended object are rendered on their own (without paper: blend modes
//!   don't blend with the paper);
//! - the blended object is rendered alone with its own inks (the plate renders, so [Black] stays
//!   0/0/0/100), and blended with the backdrop pixel by pixel in the working CMYK (the backdrop
//!   separated through the colour settings);
//! - the objects above are laid on top, and the result on the paper.
//!
//! PDF export needs none of this: the page group's CMYK blend space does it in the viewer.

use designcraft_color::BlendMode as DcBlend;
use designcraft_compose::Cache;
use designcraft_doc::{Document, ItemId, SpreadRef};
use designcraft_geom::{Affine, Rect};

use crate::{Placed, RenderOptions, Renderer};

/// Blend modes that blend channel by channel (Hue, Saturation, Color and Luminosity don't).
pub(crate) fn separable(b: DcBlend) -> bool {
    !matches!(b, DcBlend::Normal | DcBlend::Hue | DcBlend::Saturation | DcBlend::Color | DcBlend::Luminosity)
}

/// Top-level objects of spread `sr` in drawing order (layers back to front, a document spread's
/// parent items first in each layer, as `Renderer::draw_spread` draws them), each with whether
/// it blends in CMYK here (a spread object with a separable blend mode).
fn draw_order(doc: &Document, sr: SpreadRef, opts: &RenderOptions) -> Vec<(ItemId, bool)> {
    let mut out: Vec<(ItemId, bool)> = Vec::new();
    let Some(sp) = doc.spread(sr) else { return out };
    for layer in doc.layers.iter().rev() {
        if !layer.visible || opts.printing_only && !layer.printable {
            continue;
        }
        if let SpreadRef::Doc(si) = sr {
            let first = doc.first_page_of_spread(si);
            for (pi, page) in sp.pages.iter().enumerate() {
                if !page.show_parent_items {
                    continue;
                }
                let Some((ppi, _)) = doc.parent_page_for(first + pi) else { continue };
                let Some(parent) = doc.parents.get(ppi) else { continue };
                for it in parent.items.iter().filter(|it| it.layer == layer.id && !page.overridden.contains(&it.id)) {
                    if !out.iter().any(|(id, _)| *id == it.id) {
                        out.push((it.id, false));
                    }
                }
            }
        }
        for it in sp.items.iter().filter(|it| it.layer == layer.id) {
            let shown = !(it.hidden || opts.hidden.contains(&it.id) || opts.printing_only && it.nonprinting);
            out.push((it.id, shown && separable(it.blend)));
        }
    }
    out
}

/// Separable blend function `b` on additive values (backdrop `cb`, source `cs`), as in PDF.
fn blend(b: DcBlend, cb: f32, cs: f32) -> f32 {
    let multiply = |a: f32, c: f32| a * c;
    let screen = |a: f32, c: f32| a + c - a * c;
    let hard = |cb: f32, cs: f32| if cs <= 0.5 { multiply(cb, 2.0 * cs) } else { screen(cb, 2.0 * cs - 1.0) };
    match b {
        DcBlend::Multiply => multiply(cb, cs),
        DcBlend::Screen => screen(cb, cs),
        DcBlend::Overlay => hard(cs, cb),
        DcBlend::HardLight => hard(cb, cs),
        DcBlend::SoftLight => {
            if cs <= 0.5 {
                cb - (1.0 - 2.0 * cs) * cb * (1.0 - cb)
            } else {
                let d = if cb <= 0.25 { ((16.0 * cb - 12.0) * cb + 4.0) * cb } else { cb.sqrt() };
                cb + (2.0 * cs - 1.0) * (d - cb)
            }
        }
        DcBlend::ColorDodge => {
            if cb <= 0.0 {
                0.0
            } else if cs >= 1.0 {
                1.0
            } else {
                (cb / (1.0 - cs)).min(1.0)
            }
        }
        DcBlend::ColorBurn => {
            if cb >= 1.0 {
                1.0
            } else if cs <= 0.0 {
                0.0
            } else {
                1.0 - ((1.0 - cb) / cs).min(1.0)
            }
        }
        DcBlend::Darken => cb.min(cs),
        DcBlend::Lighten => cb.max(cs),
        DcBlend::Difference => (cb - cs).abs(),
        DcBlend::Exclusion => cb + cs - 2.0 * cb * cs,
        _ => cs,
    }
}

/// Blend CMYK inks `source` (blend mode `mode`) over `backdrop`: a subtractive space blends the
/// complements (PDF), so the formulas see light, not ink.
pub(crate) fn blend_inks(mode: DcBlend, backdrop: [f32; 4], source: [f32; 4]) -> [f32; 4] {
    let mut out = [0.0; 4];
    for i in 0..4 {
        out[i] = 1.0 - blend(mode, 1.0 - backdrop[i], 1.0 - source[i]).clamp(0.0, 1.0);
    }
    out
}

const RGB_N: usize = 9;

/// The working CMYK → sRGB table for the current colour settings (9⁴, built once per settings);
/// sample it with [`srgb_in`].
fn srgb_table() -> std::sync::Arc<Vec<[f32; 3]>> {
    const N: usize = RGB_N;
    type Lut = (designcraft_color::cms::ColorSettings, std::sync::Arc<Vec<[f32; 3]>>);
    static LUT: std::sync::Mutex<Option<Lut>> = std::sync::Mutex::new(None);
    let settings = designcraft_color::cms::active_settings();
    let mut g = LUT.lock().unwrap_or_else(|e| e.into_inner());
    match g.as_ref() {
        Some((s, t)) if *s == settings => t.clone(),
        _ => {
            let cms = designcraft_color::cms::active();
            let step = |i: usize| i as f32 / (N - 1) as f32;
            let t: Vec<[f32; 3]> = (0..N * N * N * N)
                .map(|i| cms.cmyk_to_srgb([step(i / (N * N * N)), step(i / (N * N) % N), step(i / N % N), step(i % N)], false))
                .collect();
            let t = std::sync::Arc::new(t);
            *g = Some((settings, t.clone()));
            t
        }
    }
}

/// Quadrilinear lookup of `cmyk` in an [`srgb_table`].
fn srgb_in(table: &[[f32; 3]], cmyk: [f32; 4]) -> [f32; 3] {
    const N: usize = RGB_N;
    let f = cmyk.map(|v| v.clamp(0.0, 1.0) * (N - 1) as f32);
    let i = f.map(|v| (v as usize).min(N - 2));
    let d = [f[0] - i[0] as f32, f[1] - i[1] as f32, f[2] - i[2] as f32, f[3] - i[3] as f32];
    let mut out = [0.0f32; 3];
    for corner in 0..16usize {
        let bit = |n: usize| corner >> (3 - n) & 1;
        let w: f32 = (0..4).map(|n| if bit(n) == 1 { d[n] } else { 1.0 - d[n] }).product();
        if w == 0.0 {
            continue;
        }
        let idx = (i[0] + bit(0)) * N * N * N + (i[1] + bit(1)) * N * N + (i[2] + bit(2)) * N + i[3] + bit(3);
        if let Some(rgb) = table.get(idx) {
            for k in 0..3 {
                out[k] += rgb[k] * w;
            }
        }
    }
    out
}

/// Premultiplied RGBA8 → straight 0..1 colour and alpha.
fn straight(px: &[u8]) -> ([f32; 3], f32) {
    let a = px.get(3).copied().unwrap_or(0) as f32 / 255.0;
    if a <= 0.0 {
        return ([0.0; 3], 0.0);
    }
    let c = |i: usize| (px.get(i).copied().unwrap_or(0) as f32 / 255.0 / a).min(1.0);
    ([c(0), c(1), c(2)], a)
}

impl Renderer {
    /// Composite spread `pl` again in the working CMYK where its top-level objects use separable
    /// blend modes, over the pages' pixels of `pixels` (`w`×`h`, premultiplied, rendered with
    /// `view`). Returns whether it did.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn cmyk_blend_spread(
        &mut self,
        doc: &Document,
        cache: &Cache,
        pl: &Placed,
        w: u32,
        h: u32,
        view: Affine,
        opts: &RenderOptions,
        pixels: &mut [u8],
    ) -> bool {
        let order = draw_order(doc, pl.spread, opts);
        if !order.iter().any(|(_, b)| *b) {
            return false;
        }
        let Some(sp) = doc.spread(pl.spread) else { return false };
        // The pages' device pixels (the pasteboard keeps the RGB result).
        let area = (view * pl.xf).transform_rect_bbox(sp.bounds()).intersect(Rect::new(0.0, 0.0, w as f64, h as f64));
        let (x0, y0) = (area.x0.floor().max(0.0) as u32, area.y0.floor().max(0.0) as u32);
        let (x1, y1) = (area.x1.ceil().min(w as f64) as u32, area.y1.ceil().min(h as f64) as u32);
        if x1 <= x0 || y1 <= y0 {
            return false;
        }
        let (rw, rh) = (x1 - x0, y1 - y0);
        let rview = Affine::translate((-(x0 as f64), -(y0 as f64))) * view;
        let ids: Vec<ItemId> = order.iter().map(|(id, _)| *id).collect();
        let stats = self.stats;
        // The spread's objects in `show` only (on paper or not), in a region-sized image.
        let sub = |me: &mut Renderer, d: &Document, show: &[ItemId], paper: bool, plate: Option<u8>| {
            let mut o = opts.clone();
            o.hidden.extend(ids.iter().filter(|i| !show.contains(i)));
            o.paper = paper;
            o.page_shadow = paper && opts.page_shadow;
            o.background = if paper { opts.background } else { None };
            o.blend_space_view = false;
            o.plate = plate;
            me.render(d, cache, std::slice::from_ref(pl), rw, rh, rview, &o).pixels
        };
        let first = order.iter().position(|(_, b)| *b).unwrap_or(order.len());
        let (to_cmyk, to_rgb) = (crate::cmyk_table(), srgb_table());
        let mut acc = sub(self, doc, ids.get(..first).unwrap_or(&[]), false, None);
        let blended: Vec<usize> = order.iter().enumerate().filter(|(_, (_, b))| *b).map(|(i, _)| i).collect();
        for (j, &pos) in blended.iter().enumerate() {
            let Some(&id) = ids.get(pos) else { continue };
            let Some(mode) = doc.item(id).map(|it| it.blend) else { continue };
            // The object alone, its blend mode left to us (opacity and effects stay).
            let mut alone = doc.clone();
            if let Some(it) = alone.item_mut(id) {
                it.blend = DcBlend::Normal;
            }
            let rgb = sub(self, &alone, &[id], false, None);
            let cmy = sub(self, &alone, &[id], false, Some(4));
            let k = sub(self, &alone, &[id], false, Some(3));
            for p in (0..acc.len()).step_by(4) {
                let (Some(s), Some(c), Some(kk)) = (rgb.get(p..p + 4), cmy.get(p..p + 4), k.get(p..p + 4)) else { continue };
                let (_, sa) = straight(s);
                if sa <= 0.0 {
                    continue;
                }
                let Some(b) = acc.get(p..p + 4) else { continue };
                let (brgb, ba) = straight(b);
                if ba <= 0.0 {
                    // Nothing below: the object as it is.
                    acc[p..p + 4].copy_from_slice(s);
                    continue;
                }
                let (cs, ca) = straight(c);
                let (ks, _) = straight(kk);
                let source = if ca > 0.0 { [1.0 - cs[0], 1.0 - cs[1], 1.0 - cs[2], 1.0 - ks[0]] } else { crate::cmyk_in(&to_cmyk, straight(s).0) };
                let backdrop = crate::cmyk_in(&to_cmyk, brgb);
                let mixed = blend_inks(mode, backdrop, source);
                // PDF compositing: the blend counts where the backdrop is.
                let ar = ba + sa - ba * sa;
                let t = sa / ar;
                let mut out = [0.0f32; 4];
                for i in 0..4 {
                    let m = (1.0 - ba) * source[i] + ba * mixed[i];
                    out[i] = (1.0 - t) * backdrop[i] + t * m;
                }
                let rgb_out = srgb_in(&to_rgb, out);
                let a8 = (ar * 255.0).round().clamp(0.0, 255.0);
                for i in 0..3 {
                    acc[p + i] = (rgb_out[i].clamp(0.0, 1.0) * a8).round() as u8;
                }
                acc[p + 3] = a8 as u8;
            }
            // The objects up to the next blended one, laid on top.
            let next = blended.get(j + 1).copied().unwrap_or(ids.len());
            if let Some(between) = ids.get(pos + 1..next).filter(|b| !b.is_empty()) {
                let n = sub(self, doc, between, false, None);
                over(&mut acc, &n);
            }
        }
        // On the paper (and the canvas background).
        let mut base = sub(self, doc, &[], opts.paper, None);
        over(&mut base, &acc);
        for y in 0..rh {
            let (src, dst) = (((y * rw) * 4) as usize, (((y0 + y) * w + x0) * 4) as usize);
            let len = (rw * 4) as usize;
            if let (Some(s), Some(d)) = (base.get(src..src + len), pixels.get_mut(dst..dst + len)) {
                d.copy_from_slice(s);
            }
        }
        self.stats = stats;
        true
    }
}

/// `top` over `dst` (premultiplied RGBA8, same size).
fn over(dst: &mut [u8], top: &[u8]) {
    for (d, s) in dst.chunks_exact_mut(4).zip(top.chunks_exact(4)) {
        let inv = 255 - s[3] as u32;
        for i in 0..4 {
            d[i] = (s[i] as u32 + (d[i] as u32 * inv + 127) / 255).min(255) as u8;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_dodge_with_black_whitens_coloured_inks_in_cmyk() {
        // 100% K over a lilac (C 30 M 35): the inks go, K stays the backdrop's.
        let r = blend_inks(DcBlend::ColorDodge, [0.3, 0.35, 0.0, 0.0], [0.0, 0.0, 0.0, 1.0]);
        assert_eq!(r, [0.0, 0.0, 0.0, 0.0]);
        // Multiply adds inks.
        let r = blend_inks(DcBlend::Multiply, [0.3, 0.0, 0.0, 0.0], [0.0, 0.5, 0.0, 0.0]);
        assert!((r[0] - 0.3).abs() < 1e-6 && (r[1] - 0.5).abs() < 1e-6);
        // Multiplying by paper changes nothing; screening with 100% K clears the coloured inks too.
        let near = |a: [f32; 4], b: [f32; 4]| a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-5);
        assert!(near(blend_inks(DcBlend::Multiply, [0.2, 0.4, 0.6, 0.1], [0.0; 4]), [0.2, 0.4, 0.6, 0.1]));
        assert!(near(blend_inks(DcBlend::Screen, [0.2, 0.4, 0.6, 0.1], [0.0, 0.0, 0.0, 1.0]), [0.0, 0.0, 0.0, 0.1]));
        assert!(!separable(DcBlend::Normal) && !separable(DcBlend::Luminosity) && separable(DcBlend::ColorDodge));
    }
}
