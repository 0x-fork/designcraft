//! Panel icons as textures, for craft-ui's iconic columns, which draw an `ImageSource`.
//!
//! DesignCraft draws its icons in code ([`crate::icons::paint`]). Each panel's icon is painted
//! once, white, into a scratch egui context; the shapes are tessellated there and the triangles
//! filled on the CPU. The column tints the texture with its text colour.

use std::collections::HashMap;

use egui::{
    Color32, ColorImage, Context, Id, ImageSource, LayerId, Pos2, RawInput, Rect, TextureHandle, TextureId, TextureOptions,
    epaint::{ImageData, Mesh, Primitive},
    load::SizedTexture,
    pos2, vec2,
};

/// Icons per row of the scratch canvas.
const COLUMNS: usize = 8;

#[derive(Clone, Default)]
struct Cache {
    /// Side of each texture in pixels.
    px: usize,
    textures: Vec<(String, TextureHandle)>,
}

fn cache_id() -> Id {
    Id::new("designcraft-panel-icon-textures")
}

/// Every dock panel's icon at `size` points, keyed by panel id. Textures are made once per
/// pixel size and kept in the context.
pub(crate) fn panel_icons(ctx: &Context, size: f32) -> HashMap<String, ImageSource<'static>> {
    let scale = size * ctx.pixels_per_point();
    let px = if scale.is_finite() { scale.round().clamp(8.0, 128.0) as usize } else { 20 };
    let cached = ctx.data(|d| d.get_temp::<Cache>(cache_id())).filter(|cache| cache.px == px);
    let cache = cached.unwrap_or_else(|| {
        let panels: Vec<(&str, &str)> = crate::dock::DOCK_TABS.iter().chain(crate::dock::ICON_PANELS).map(|(id, _, icon)| (*id, *icon)).collect();
        let images = render(px, &panels);
        let textures = panels
            .iter()
            .zip(images)
            .map(|((id, _), image)| ((*id).to_string(), ctx.load_texture(format!("designcraft-panel-icon-{id}"), image, TextureOptions::LINEAR)))
            .collect();
        let cache = Cache { px, textures };
        ctx.data_mut(|d| d.insert_temp(cache_id(), cache.clone()));
        cache
    });
    cache
        .textures
        .iter()
        .map(|(id, texture)| (id.clone(), ImageSource::Texture(SizedTexture::new(texture.id(), vec2(px as f32, px as f32)))))
        .collect()
}

/// The icons of `panels` (id, icon name), white on transparent, `px` pixels square.
fn render(px: usize, panels: &[(&str, &str)]) -> Vec<ColorImage> {
    let cell = px as f32;
    let rows = panels.len().div_ceil(COLUMNS).max(1);
    let (width, height) = (COLUMNS * px, rows * px);
    let cell_rect = |i: usize| Rect::from_min_size(pos2((i % COLUMNS) as f32 * cell, (i / COLUMNS) as f32 * cell), vec2(cell, cell));
    let scratch = Context::default();
    scratch.set_fonts(crate::theme::font_definitions(&[], "en"));
    let input = RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(width as f32, height as f32))), ..Default::default() };
    let mut output = scratch.run_ui(input, |ui| {
        let painter = ui.ctx().layer_painter(LayerId::background());
        for (i, (_, icon)) in panels.iter().enumerate() {
            let rect = cell_rect(i);
            crate::icons::paint(&painter.with_clip_rect(rect), rect, icon, Color32::WHITE);
        }
    });
    let atlas =
        output.textures_delta.set.get(&TextureId::default()).and_then(|deltas| deltas.iter().find(|delta| delta.pos.is_none())).map(|delta| {
            match &delta.image {
                ImageData::Color(image) => image.clone(),
            }
        });
    let shapes = std::mem::take(&mut output.shapes);
    let primitives = scratch.tessellate(shapes, output.pixels_per_point);
    output.textures_delta.clear();
    let mut canvas = vec![[0.0f32; 4]; width * height];
    for primitive in &primitives {
        if let Primitive::Mesh(mesh) = &primitive.primitive {
            fill(&mut canvas, width, height, mesh, primitive.clip_rect, atlas.as_deref());
        }
    }
    (0..panels.len())
        .map(|i| {
            let origin = cell_rect(i).min;
            let (x0, y0) = (origin.x as usize, origin.y as usize);
            let mut pixels = Vec::with_capacity(px * px * 4);
            for y in y0..y0 + px {
                for x in x0..x0 + px {
                    let [r, g, b, a] = canvas.get(y * width + x).copied().unwrap_or_default();
                    pixels.extend([r, g, b, a].map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8));
                }
            }
            ColorImage::from_rgba_premultiplied([px, px], &pixels)
        })
        .collect()
}

/// Fill `mesh`'s triangles into a premultiplied RGBA canvas, sampling the font atlas for text.
/// Pixel centres inside a triangle take its interpolated colour; egui's feathered edges give
/// the anti-aliasing.
fn fill(canvas: &mut [[f32; 4]], width: usize, height: usize, mesh: &Mesh, clip: Rect, atlas: Option<&ColorImage>) {
    let bounds = clip.intersect(Rect::from_min_size(Pos2::ZERO, vec2(width as f32, height as f32)));
    if !bounds.is_positive() {
        return;
    }
    let texel = |u: f32, v: f32| -> [f32; 4] {
        if mesh.texture_id != TextureId::default() {
            return [1.0; 4];
        }
        let Some(atlas) = atlas else { return [1.0; 4] };
        let [w, h] = atlas.size;
        let x = ((u * w as f32) as usize).min(w.saturating_sub(1));
        let y = ((v * h as f32) as usize).min(h.saturating_sub(1));
        atlas.pixels.get(y * w + x).map_or([1.0; 4], |c| c.to_array().map(|v| f32::from(v) / 255.0))
    };
    let edge = |a: Pos2, b: Pos2, p: Pos2| (b.x - a.x) * (p.y - a.y) - (b.y - a.y) * (p.x - a.x);
    for triangle in mesh.indices.as_chunks::<3>().0 {
        let vertex = |i: usize| triangle.get(i).and_then(|&index| mesh.vertices.get(index as usize));
        let (Some(a), Some(b), Some(c)) = (vertex(0), vertex(1), vertex(2)) else { continue };
        let area = edge(a.pos, b.pos, c.pos);
        if !area.is_finite() || area.abs() < 1e-6 {
            continue;
        }
        let min = a.pos.min(b.pos).min(c.pos).max(bounds.min);
        let max = a.pos.max(b.pos).max(c.pos).min(bounds.max);
        if !(min.x < max.x && min.y < max.y) {
            continue;
        }
        let colors = [a.color, b.color, c.color].map(|c| c.to_array().map(|v| f32::from(v) / 255.0));
        for y in min.y.floor() as usize..(max.y.ceil() as usize).min(height) {
            for x in min.x.floor() as usize..(max.x.ceil() as usize).min(width) {
                let p = pos2(x as f32 + 0.5, y as f32 + 0.5);
                let wa = edge(b.pos, c.pos, p) / area;
                let wb = edge(c.pos, a.pos, p) / area;
                let wc = 1.0 - wa - wb;
                if wa < 0.0 || wb < 0.0 || wc < 0.0 {
                    continue;
                }
                let u = wa * a.uv.x + wb * b.uv.x + wc * c.uv.x;
                let v = wa * a.uv.y + wb * b.uv.y + wc * c.uv.y;
                let t = texel(u, v);
                let src: [f32; 4] = std::array::from_fn(|k| (wa * colors[0][k] + wb * colors[1][k] + wc * colors[2][k]) * t[k]);
                if let Some(dst) = canvas.get_mut(y * width + x) {
                    let keep = 1.0 - src[3];
                    *dst = std::array::from_fn(|k| src[k] + dst[k] * keep);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn panel_icons_are_painted_into_their_textures() {
        let images = super::render(20, &[("pages", "panel-pages"), ("character", "panel-character")]);
        assert_eq!(images.len(), 2);
        for image in &images {
            assert_eq!(image.size, [20, 20]);
            let inked = image.pixels.iter().filter(|p| p.a() > 128).count();
            assert!(inked > 10 && inked < 300, "{inked} inked pixels");
        }
    }
}
