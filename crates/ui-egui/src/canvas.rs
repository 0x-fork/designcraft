//! The document canvas: pasteboard, spreads (rendered by `designcraft-render`), guides, frame edges,
//! selection, text ports and threads, the text caret, rulers, and pointer/keyboard input.

use designcraft_compose as compose;
use designcraft_doc::{Content, Document, GuideSel, Item, SpreadRef};
use designcraft_engine::doc::Selection;
use designcraft_engine::{ImeMutation, ImeTextContext};
use designcraft_geom::{Affine, Point, Rect as DRect, Vec2 as DVec2};
use designcraft_tools::{CanvasLayout, Cursor, Mods, Overlay, PointerEvent, PointerKind, SpreadSlot, ToolKey};
use egui::ime_context::{CommittedText, ImeMutationStamp, ImeObservation, ImeTarget};
use egui::output::IMESurroundingText;
use egui::{Color32, Pos2, Rect, Sense, Stroke, StrokeKind, pos2, vec2};
use serde_json::{Value, json};

use crate::{DesignApp, ScreenMode, View, theme::Tokens};

pub const RULER: f32 = 15.0;
/// Width of the view's scrollbars.
pub const SCROLLBAR: f32 = 10.0;
/// Shortest scrollbar thumb.
const MIN_THUMB: f32 = 18.0;

/// Canvas ↔ screen transform.
#[derive(Clone, Copy, Debug)]
pub struct Xf {
    /// Top left of the (unrotated) view rectangle.
    pub min: Pos2,
    pub origin: Point,
    pub zoom: f64,
    /// Quarter turns clockwise about `pivot` (the view centre).
    pub rot: u8,
    pub pivot: Pos2,
}

/// Rotate `p` by `q` quarter turns clockwise about `c` (screen space, y down).
fn quarter(p: Pos2, c: Pos2, q: u8) -> Pos2 {
    let (dx, dy) = (p.x - c.x, p.y - c.y);
    let (x, y) = match q % 4 {
        0 => (dx, dy),
        1 => (-dy, dx),
        2 => (-dx, -dy),
        _ => (dy, -dx),
    };
    pos2(c.x + x, c.y + y)
}

/// The unrotated view rectangle for a screen rectangle and a rotation (width and height swap
/// for quarter turns).
pub fn view_rect(screen: Rect, rot: u8) -> Rect {
    if rot % 2 == 1 { Rect::from_center_size(screen.center(), vec2(screen.height(), screen.width())) } else { screen }
}

impl Xf {
    pub fn new(rect: Rect, v: &View) -> Self {
        Xf { min: rect.min, origin: v.origin, zoom: v.zoom, rot: v.rotation % 4, pivot: rect.center() }
    }
    pub fn to_screen(&self, p: Point) -> Pos2 {
        let s = pos2(self.min.x + ((p.x - self.origin.x) * self.zoom) as f32, self.min.y + ((p.y - self.origin.y) * self.zoom) as f32);
        if self.rot == 0 { s } else { quarter(s, self.pivot, self.rot) }
    }
    /// A screen position in the unrotated view.
    pub fn unrotate(&self, p: Pos2) -> Pos2 {
        if self.rot == 0 { p } else { quarter(p, self.pivot, 4 - self.rot) }
    }
    /// A screen-space movement in the unrotated view.
    pub fn unrotate_delta(&self, d: egui::Vec2) -> egui::Vec2 {
        self.unrotate(self.pivot + d) - self.pivot
    }
    pub fn to_canvas(&self, p: Pos2) -> Point {
        let p = self.unrotate(p);
        Point::new(self.origin.x + (p.x - self.min.x) as f64 / self.zoom, self.origin.y + (p.y - self.min.y) as f64 / self.zoom)
    }
    pub fn rect(&self, r: DRect) -> Rect {
        Rect::from_two_pos(self.to_screen(Point::new(r.x0, r.y0)), self.to_screen(Point::new(r.x1, r.y1)))
    }
    /// Canvas → screen affine (for vello rendering: pixels = points × ppp).
    pub fn affine(&self, ppp: f64, area_min: Pos2) -> Affine {
        let unrotated = Affine::translate((self.min.x as f64, self.min.y as f64))
            * Affine::scale(self.zoom)
            * Affine::translate((-self.origin.x, -self.origin.y));
        let (cx, cy) = (self.pivot.x as f64, self.pivot.y as f64);
        let rot = Affine::translate((cx, cy)) * Affine::rotate(self.rot as f64 * std::f64::consts::FRAC_PI_2) * Affine::translate((-cx, -cy));
        Affine::scale(ppp) * Affine::translate((-(area_min.x as f64), -(area_min.y as f64))) * rot * unrotated
    }
}

fn c32(c: [u8; 3]) -> Color32 {
    Color32::from_rgb(c[0], c[1], c[2])
}

/// Fit the current spread (or page) in the canvas.
pub fn fit(app: &mut DesignApp, rect: Rect, what: &str) {
    let Some(st) = app.session.active() else { return };
    let layout = CanvasLayout::new(&st.doc, st.editing_parents);
    let cur = current_slot(app, &layout).unwrap_or(0);
    let Some(slot) = layout.slots.get(cur) else { return };
    let mut b = slot.bounds;
    if what == "page"
        && let Some(sp) = st.doc.spread(slot.spread)
    {
        let center = app.view().map(|v| v.origin.x + rect.width() as f64 / 2.0 / v.zoom).unwrap_or(0.0);
        let c = slot.to_spread(Point::new(center, slot.bounds.center().y));
        let pi = sp.page_at_x(c.x).unwrap_or(0);
        b = slot.xf.transform_rect_bbox(sp.pages[pi].bounds());
    }
    if what == "all" {
        b = layout.slots.iter().map(|s| s.bounds).reduce(|a, b| a.union(b)).unwrap_or(b);
    }
    if what == "selection" {
        // Fit Selection in Window: the selected objects (on their spread's slot).
        let sel: Vec<designcraft_geom::Rect> = st
            .selection
            .items
            .iter()
            .filter_map(|id| {
                let loc = st.doc.find(*id)?;
                let slot = layout.slots.iter().find(|s| s.spread == loc.spread)?;
                Some(slot.xf.transform_rect_bbox(st.doc.item(*id)?.bounds()))
            })
            .collect();
        let Some(u) = sel.into_iter().reduce(|a, b| a.union(b)) else { return };
        b = u.inflate(u.width().max(u.height()) * 0.05 + 1.0, u.width().max(u.height()) * 0.05 + 1.0);
    }
    fit_bounds(app, rect, b);
}

/// Zoom and scroll so canvas rectangle `b` fills the canvas `rect`.
fn fit_bounds(app: &mut DesignApp, rect: Rect, b: DRect) {
    let pad = 40.0;
    let zoom =
        ((rect.width() as f64 - 2.0 * pad) / b.width().max(1e-6)).min((rect.height() as f64 - 2.0 * pad) / b.height().max(1e-6)).clamp(0.05, 40.0);
    let origin = Point::new(b.center().x - rect.width() as f64 / 2.0 / zoom, b.center().y - rect.height() as f64 / 2.0 / zoom);
    if let Some(v) = app.view_mut() {
        *v = View { zoom, origin, fitted: true, rotation: v.rotation };
    }
}

/// Fit spread `r` (a document or parent spread on the canvas as it is laid out now) in the canvas.
/// Before the canvas has a size, the next frame fits whatever spread is current.
pub fn show_spread(app: &mut DesignApp, r: SpreadRef) {
    let Some(st) = app.session.active() else { return };
    let Some(b) = CanvasLayout::new(&st.doc, st.editing_parents).slot(r).map(|s| s.bounds) else { return };
    match app.canvas_rect {
        Some(rect) => fit_bounds(app, rect, b),
        None => {
            if let Some(v) = app.view_mut() {
                v.fitted = false;
            }
        }
    }
}

/// Index of the slot nearest the view centre.
pub fn current_slot(app: &DesignApp, layout: &CanvasLayout) -> Option<usize> {
    let v = app.view()?;
    let r = app.canvas_rect?;
    let c = Point::new(v.origin.x + r.width() as f64 / 2.0 / v.zoom, v.origin.y + r.height() as f64 / 2.0 / v.zoom);
    layout
        .slots
        .iter()
        .enumerate()
        .min_by(|a, b| {
            let da = (a.1.bounds.center().y - c.y).abs();
            let db = (b.1.bounds.center().y - c.y).abs();
            da.total_cmp(&db)
        })
        .map(|(i, _)| i)
}

/// The spread under the view centre (a parent spread while parents are edited).
pub fn current_spread(app: &DesignApp) -> Option<SpreadRef> {
    let st = app.session.active()?;
    let layout = CanvasLayout::new(&st.doc, st.editing_parents);
    Some(layout.slots.get(current_slot(app, &layout)?)?.spread)
}

/// The spread nearest the view centre and the centre in that spread's coordinates.
pub fn view_centre(app: &DesignApp) -> Option<(SpreadRef, Point)> {
    let st = app.session.active()?;
    let layout = CanvasLayout::new(&st.doc, st.editing_parents);
    let slot = layout.slots.get(current_slot(app, &layout)?)?;
    let v = app.view()?;
    let r = app.canvas_rect?;
    let c = Point::new(v.origin.x + r.width() as f64 / 2.0 / v.zoom, v.origin.y + r.height() as f64 / 2.0 / v.zoom);
    Some((slot.spread, slot.to_spread(c)))
}

/// Where a file dropped on the window lands: the spread under the pointer, or under the view
/// centre when the pointer isn't over the canvas, and that point in spread coordinates.
pub fn drop_point(app: &DesignApp, screen: Option<Pos2>) -> Option<(SpreadRef, Point)> {
    let rect = app.canvas_rect?;
    let Some(pos) = screen.filter(|p| rect.contains(*p)) else { return view_centre(app) };
    let st = app.session.active()?;
    let layout = CanvasLayout::new(&st.doc, st.editing_parents);
    layout.spread_at(Xf::new(rect, app.view()?).to_canvas(pos))
}

/// Absolute page index under the view centre.
pub fn current_page(app: &DesignApp) -> Option<usize> {
    let st = app.session.active()?;
    let layout = CanvasLayout::new(&st.doc, st.editing_parents);
    let i = current_slot(app, &layout)?;
    let slot = layout.slots.get(i)?;
    let SpreadRef::Doc(si) = slot.spread else { return Some(0) };
    let v = app.view()?;
    let r = app.canvas_rect?;
    let c = slot.to_spread(Point::new(v.origin.x + r.width() as f64 / 2.0 / v.zoom, slot.bounds.center().y));
    let pi = st.doc.spreads[si].page_at_x(c.x).unwrap_or(0);
    Some(st.doc.first_page_of_spread(si) + pi)
}

/// The spread (document or parent) under the view centre and the page index within it: where
/// commands that act "on the page I am working on" should go.
pub fn current_spread_page(app: &DesignApp) -> Option<(SpreadRef, usize)> {
    let st = app.session.active()?;
    let layout = CanvasLayout::new(&st.doc, st.editing_parents);
    let i = current_slot(app, &layout)?;
    let slot = layout.slots.get(i)?;
    let v = app.view()?;
    let r = app.canvas_rect?;
    let c = slot.to_spread(Point::new(v.origin.x + r.width() as f64 / 2.0 / v.zoom, slot.bounds.center().y));
    let pg = st.doc.spread(slot.spread)?.page_at_x(c.x).unwrap_or(0);
    Some((slot.spread, pg))
}

/// While parent spreads are being edited: (parent spread index, page index within it) under the
/// view centre. `None` when document pages are shown.
pub fn current_parent_page(app: &DesignApp) -> Option<(usize, usize)> {
    let st = app.session.active()?;
    if !st.editing_parents {
        return None;
    }
    let layout = CanvasLayout::new(&st.doc, true);
    let i = current_slot(app, &layout)?;
    let slot = layout.slots.get(i)?;
    let SpreadRef::Parent(pi) = slot.spread else { return None };
    let v = app.view()?;
    let r = app.canvas_rect?;
    let c = slot.to_spread(Point::new(v.origin.x + r.width() as f64 / 2.0 / v.zoom, slot.bounds.center().y));
    let qi = st.doc.parents.get(pi)?.page_at_x(c.x).unwrap_or(0);
    Some((pi, qi))
}

/// Scroll so absolute page `abs` is centred (keeps zoom). A jump Go Back can return from.
pub fn go_to_page(app: &mut DesignApp, abs: usize) {
    note_jump(app, false);
    centre_page(app, abs);
    note_jump(app, true);
}

fn centre_page(app: &mut DesignApp, abs: usize) {
    let Some(st) = app.session.active() else { return };
    let layout = CanvasLayout::new(&st.doc, false);
    let Some(pr) = layout.page_rect(&st.doc, abs) else { return };
    let Some(rect) = app.canvas_rect else { return };
    if let Some(v) = app.view_mut() {
        v.origin = Point::new(pr.center().x - rect.width() as f64 / 2.0 / v.zoom, pr.center().y - rect.height() as f64 / 2.0 / v.zoom);
    }
}

/// The document's pages by id, in page order.
fn page_ids(doc: &Document) -> Vec<designcraft_doc::ids::PageId> {
    doc.spreads.iter().flat_map(|s| s.pages.iter().map(|p| p.id)).collect()
}

/// The page in view and the view centre's offset from that page's centre. Parent pages have none,
/// nor has a view still waiting to be fitted (leaving parent pages for a document page).
pub(crate) fn page_in_view(app: &DesignApp) -> Option<(designcraft_doc::ids::PageId, DVec2)> {
    let st = app.session.active().filter(|st| !st.editing_parents)?;
    let (v, r) = (app.view().filter(|v| v.fitted)?, app.canvas_rect?);
    let abs = current_page(app)?;
    let id = st.doc.page(abs)?.id;
    let pr = CanvasLayout::new(&st.doc, false).page_rect(&st.doc, abs)?;
    let c = Point::new(v.origin.x + r.width() as f64 / 2.0 / v.zoom, v.origin.y + r.height() as f64 / 2.0 / v.zoom);
    Some((id, c - pr.center()))
}

/// Layout ▸ Go Back / Go Forward record jumps (Go to Page, the page and spread commands, the Pages
/// panel), not scrolling: where the view is when a jump starts, and where the jump took it.
fn note_jump(app: &mut DesignApp, to: bool) {
    let Some((id, offset)) = page_in_view(app) else { return };
    if let Some(h) = app.page_history_mut() {
        if to { h.visit(id, offset) } else { h.leave(id, offset) }
    }
}

/// Whether Layout ▸ Go Back (`step` < 0) or Go Forward has a page to go to from the view as it is.
pub fn can_go_through_history(app: &DesignApp, step: isize) -> bool {
    let Some(st) = app.session.active().filter(|st| !st.editing_parents) else { return false };
    // A view still waiting to be fitted is refitted next frame, which would replace the destination.
    if !app.view().is_some_and(|v| v.fitted) {
        return false;
    }
    let ids = page_ids(&st.doc);
    let Some(mut h) = app.page_history().cloned() else { return false };
    if let Some((id, offset)) = page_in_view(app) {
        h.leave(id, offset);
    }
    h.can_go(step, |id| ids.contains(&id))
}

/// Layout ▸ Go Back (`step` < 0) or Go Forward: show the page viewed before or after this one, where
/// the view was on it (zoom kept).
pub fn go_through_history(app: &mut DesignApp, step: isize) {
    if !app.view().is_some_and(|v| v.fitted) {
        return;
    }
    note_jump(app, false);
    let Some(ids) = app.session.active().filter(|st| !st.editing_parents).map(|st| page_ids(&st.doc)) else { return };
    let Some((id, offset)) = app.page_history_mut().and_then(|h| h.go(step, |id| ids.contains(&id))) else { return };
    let Some(st) = app.session.active() else { return };
    let Some(pr) = ids.iter().position(|p| *p == id).and_then(|abs| CanvasLayout::new(&st.doc, false).page_rect(&st.doc, abs)) else { return };
    let Some(rect) = app.canvas_rect else { return };
    let c = pr.center() + offset;
    if let Some(v) = app.view_mut() {
        v.origin = Point::new(c.x - rect.width() as f64 / 2.0 / v.zoom, c.y - rect.height() as f64 / 2.0 / v.zoom);
    }
}

/// Presentation: the spread in view, centred and as large as `rect` allows.
fn present_fit(app: &mut DesignApp, rect: Rect) {
    let Some(st) = app.session.active() else { return };
    let layout = CanvasLayout::new(&st.doc, st.editing_parents);
    let Some(b) = current_slot(app, &layout).and_then(|i| layout.slots.get(i)).map(|s| s.bounds) else { return };
    if b.width() <= 0.0 || b.height() <= 0.0 {
        return;
    }
    let zoom = (rect.width() as f64 / b.width()).min(rect.height() as f64 / b.height()).clamp(0.05, 40.0);
    let origin = Point::new(b.center().x - rect.width() as f64 / 2.0 / zoom, b.center().y - rect.height() as f64 / 2.0 / zoom);
    if let Some(v) = app.view_mut() {
        v.zoom = zoom;
        v.origin = origin;
    }
}

/// Centre the spread `delta` spreads after (or before) the one in view (keeps zoom); stops at the
/// first and last spread. Parent spreads while they are being edited.
pub fn step_spread(app: &mut DesignApp, delta: isize) {
    // At the first or last spread only the view moves, within the spread: no jump for Go Back.
    let jump = spread_step(app, delta).is_some_and(|(cur, to)| cur != to);
    if jump {
        note_jump(app, false);
    }
    centre_spread(app, delta);
    if jump {
        note_jump(app, true);
    }
}

/// The slot in view and the one `delta` spreads after (or before) it, stopping at the ends.
fn spread_step(app: &DesignApp, delta: isize) -> Option<(usize, usize)> {
    let st = app.session.active()?;
    let layout = CanvasLayout::new(&st.doc, st.editing_parents);
    let cur = current_slot(app, &layout)?;
    Some((cur, cur.saturating_add_signed(delta).min(layout.slots.len().saturating_sub(1))))
}

fn centre_spread(app: &mut DesignApp, delta: isize) {
    let Some((_, to)) = spread_step(app, delta) else { return };
    let Some(st) = app.session.active() else { return };
    let layout = CanvasLayout::new(&st.doc, st.editing_parents);
    let Some(b) = layout.slots.get(to).map(|s| s.bounds) else { return };
    let Some(rect) = app.canvas_rect else { return };
    if let Some(v) = app.view_mut() {
        v.origin = Point::new(b.center().x - rect.width() as f64 / 2.0 / v.zoom, b.center().y - rect.height() as f64 / 2.0 / v.zoom);
    }
}

pub fn zoom_at(app: &mut DesignApp, screen: Pos2, factor: f64) {
    let Some(rect) = app.canvas_rect else { return };
    let Some(v) = app.view_mut() else { return };
    let xf = Xf::new(rect, v);
    let c = xf.to_canvas(screen);
    let screen = xf.unrotate(screen);
    let nz = (v.zoom * factor).clamp(0.05, 40.0);
    v.zoom = nz;
    v.origin = Point::new(c.x - (screen.x - rect.min.x) as f64 / nz, c.y - (screen.y - rect.min.y) as f64 / nz);
}

pub fn set_zoom(app: &mut DesignApp, z: f64) {
    let Some(rect) = app.canvas_rect else { return };
    let cur = app.view().map(|v| v.zoom).unwrap_or(1.0);
    zoom_at(app, rect.center(), z / cur);
}

/// How long egui (0.37 `WheelState`) keeps a scroll action's latched modifiers after the last
/// wheel event: 150 ms, or a second while fingers rest on a trackpad. The Alt latch below ends
/// with the action, exactly as egui ends Ctrl/Cmd, so the rest of an ended motion scrolls for
/// both keys and a later plain scroll is never taken for a zoom.
const SCROLL_ACTION_TIMEOUT: f64 = 0.150;
const RESTING_FINGER_TIMEOUT: f64 = 1.0;

/// Whether the wheel motion now arriving was made with Alt down. As egui does for Ctrl/Cmd, a
/// motion keeps its key to its end: the wheel is smoothed over a few frames, and a trackpad swipe
/// coasts on after the key is let go.
fn wheel_alt(ui: &egui::Ui) -> bool {
    let id = egui::Id::unique("canvas_wheel_alt");
    // Alt, whether a trackpad swipe is under way, and the time of the last wheel event.
    let (mut alt, mut swipe, mut last): (bool, bool, f64) = ui.data(|d| d.get_temp(id)).unwrap_or((false, false, f64::NEG_INFINITY));
    ui.input(|i| {
        for e in &i.events {
            let egui::Event::MouseWheel { phase, modifiers, source, .. } = e else { continue };
            // egui ends the action (and drops its latched modifiers) when a new event arrives
            // after this pause; the Alt latch ends there too.
            let timeout = if swipe && *source == egui::MouseWheelSource::Trackpad { RESTING_FINGER_TIMEOUT } else { SCROLL_ACTION_TIMEOUT };
            if timeout < i.time - last {
                alt = false;
                swipe = false;
            }
            last = i.time;
            (alt, swipe) = match phase {
                egui::TouchPhase::Start => (modifiers.alt, true),
                egui::TouchPhase::Move => (modifiers.alt || (swipe && alt), swipe),
                egui::TouchPhase::End | egui::TouchPhase::Cancel => (false, false),
            };
        }
    });
    ui.data_mut(|d| d.insert_temp(id, (alt, swipe, last)));
    alt
}

pub fn apply_view_request(app: &mut DesignApp, p: &Value) {
    if let Some(r) = p.get("showSpread").and_then(|v| serde_json::from_value::<SpreadRef>(v.clone()).ok()) {
        show_spread(app, r);
    }
    // Power Zoom (Hand tool, Alt-press): zoom out to the spread, aim a view-sized rectangle, zoom
    // back in where it was released.
    if let Some(phase) = p.get("powerZoom").and_then(Value::as_str) {
        let at = p.get("at").and_then(Value::as_array).map(|a| Point::new(a[0].as_f64().unwrap_or(0.0), a[1].as_f64().unwrap_or(0.0)));
        match phase {
            "start" => {
                if let (Some(v), Some(rect)) = (app.view().copied(), app.canvas_rect) {
                    let centre = Xf::new(rect, &v).to_canvas(rect.center());
                    app.power_zoom = Some((v.zoom, at.unwrap_or(centre)));
                    fit(app, rect, "spread");
                }
            }
            "move" => {
                if let (Some(pz), Some(a)) = (&mut app.power_zoom, at) {
                    pz.1 = a;
                }
            }
            _ => {
                if let (Some((z, c)), Some(rect)) = (app.power_zoom.take(), app.canvas_rect)
                    && let Some(v) = app.view_mut()
                {
                    let c = at.unwrap_or(c);
                    v.zoom = z;
                    v.origin = Point::new(c.x - rect.width() as f64 / 2.0 / z, c.y - rect.height() as f64 / 2.0 / z);
                }
            }
        }
    }
    if let Some(d) = p.get("pan").and_then(Value::as_array) {
        let (dx, dy) = (d.first().and_then(Value::as_f64).unwrap_or(0.0), d.get(1).and_then(Value::as_f64).unwrap_or(0.0));
        if let Some(v) = app.view_mut() {
            v.origin = Point::new(v.origin.x - dx / v.zoom, v.origin.y - dy / v.zoom);
        }
    }
    if let (Some(a), Some(rect)) = (p.get("zoomAt").and_then(Value::as_array), app.canvas_rect)
        && let Some(v) = app.view()
    {
        let c = Point::new(a[0].as_f64().unwrap_or(0.0), a[1].as_f64().unwrap_or(0.0));
        let xf = Xf::new(rect, v);
        let s = xf.to_screen(c);
        zoom_at(app, s, p.get("factor").and_then(Value::as_f64).unwrap_or(2.0));
    }
}

fn mods(i: &egui::InputState, space: bool) -> Mods {
    Mods { shift: i.modifiers.shift, alt: i.modifiers.alt, cmd: i.modifiers.command, ctrl: i.modifiers.ctrl, space }
}

/// Held Space belongs to navigation only when neither the document nor a text field is typing.
fn navigation_space(app: &mut DesignApp, ctx: &egui::Context) -> bool {
    ctx.input(|i| i.focused && i.key_down(egui::Key::Space)) && !app.session.wants_text() && !ctx.text_edit_focused()
}

/// Allow temporary Zoom only for Cmd+Space with no pointer press and no busy tool.
fn spring_zoom_mode(app: &DesignApp, ctx: &egui::Context, space: bool) -> bool {
    let down = ctx.data(|d| d.get_temp::<bool>(egui::Id::unique(("canvas_pointer_down", app.pane)))).unwrap_or(false);
    space && ctx.input(|i| i.modifiers.command) && !down && !app.session.tool_busy()
}

#[derive(Clone, Default)]
struct SpringZoom {
    tool: designcraft_tools::ZoomTool,
    down: bool,
}

/// Lend the Zoom tool the gesture; the selected tool and document selection stay intact.
fn spring_zoom_input(app: &mut DesignApp, ui: &egui::Ui, resp: &egui::Response, xf: &Xf, active: bool) -> bool {
    let id = egui::Id::unique(("canvas_spring_zoom", app.pane));
    let mut zoom: SpringZoom = ui.data(|d| d.get_temp(id)).unwrap_or_default();
    if !ui.input(|i| i.focused) {
        ui.data_mut(|d| d.remove::<SpringZoom>(id));
        return false;
    }
    if !zoom.down && (!active || ui.input(|i| i.pointer.middle_down())) {
        return false;
    }
    let (pointer, m) = ui.input(|i| (i.pointer.clone(), mods(i, true)));
    let vi = app.view_info();
    let mut events = Vec::new();
    let canvas = if xf.rot == 0 { app.canvas_rect } else { Some(resp.rect) };
    if pointer.primary_pressed()
        && resp.hovered()
        && let Some(p) = pointer.press_origin().filter(|p| canvas.is_some_and(|r| r.contains(*p)))
    {
        events.push(PointerEvent { kind: PointerKind::Down, pos: xf.to_canvas(p), mods: m, clicks: 1 });
        zoom.down = true;
        resp.request_focus();
    }
    if zoom.down
        && let Some(p) = pointer.interact_pos()
    {
        if resp.dragged() && resp.drag_delta() != egui::Vec2::ZERO {
            events.push(PointerEvent { kind: PointerKind::Drag, pos: xf.to_canvas(p), mods: m, clicks: 1 });
        }
        if pointer.primary_released() {
            events.push(PointerEvent { kind: PointerKind::Up, pos: xf.to_canvas(p), mods: m, clicks: 1 });
            zoom.down = false;
        }
    }
    for e in events {
        if let Err(err) = app.session.pointer_with_tool(&mut zoom.tool, &e, vi) {
            app.status(err.to_string());
        }
    }
    app.after_engine();
    // Keep ownership until pointer release even if the shortcut was released first, so an Up
    // cannot leak into the previous tool and edit the document.
    ui.data_mut(|d| d.insert_temp(id, zoom));
    true
}

/// Lay out the canvas, scrollbars and rulers, presentation mode, fit/view state, input handling and painting.
pub fn show(app: &mut DesignApp, ui: &mut egui::Ui) {
    let t = Tokens::get(ui.ctx());
    let outer = ui.available_rect_before_wrap();
    let rot = app.view().map_or(0, |v| v.rotation % 4);
    let presenting = app.ui.screen_mode == ScreenMode::Presentation;
    // Scrollbars sit outside the view (right and bottom); rulers and the view share the rest.
    let bars = !presenting && app.session.active().is_some();
    let full = if bars { Rect::from_min_max(outer.min, outer.max - vec2(SCROLLBAR, SCROLLBAR)) } else { outer };
    // Rulers measure the unrotated layout, so they hide while the view is rotated.
    let rulers = app.ui.rulers && !presenting && rot == 0;
    let screen = if rulers { Rect::from_min_max(full.min + vec2(RULER, RULER), full.max) } else { full };
    let rect = view_rect(screen, rot);
    app.canvas_rect = Some(rect);
    let resp = ui.allocate_rect(full, Sense::click_and_drag());
    if presenting {
        // One spread at a time, as large as the screen allows; a click shows the next one,
        // Shift-click the previous one. Tools and selection don't see the input.
        present_fit(app, rect);
        if resp.clicked() {
            // The release that made the click says whether Shift was down.
            let shift = ui.input(|i| {
                i.modifiers.shift
                    || i.events.iter().any(|e| matches!(e, egui::Event::PointerButton { pressed: false, modifiers, .. } if modifiers.shift))
            });
            step_spread(app, if shift { -1 } else { 1 });
            present_fit(app, rect);
        }
    } else {
        if !app.view().is_some_and(|v| v.fitted) {
            fit(app, rect, "spread");
        }
        handle_input(app, ui, &resp, rect);
    }
    if bars {
        scrollbars(app, ui, outer, screen, rect, &t);
    }
    let Some(st) = app.session.active() else { return };
    let Some(&v) = app.view() else { return };
    let xf = Xf::new(rect, &v);
    let doc = st.doc.clone();
    let mut layout = CanvasLayout::new(&doc, st.editing_parents);
    if presenting && let Some(cur) = current_slot(app, &layout) {
        layout.slots = layout.slots.get(cur).copied().into_iter().collect();
    }
    let painter = ui.painter_at(screen);
    // Preview, Bleed, Slug and Presentation are all preview modes: guides, frame edges and the
    // other non-printing marks go, and only the shown area of each page keeps its content. They
    // differ in how much of the page shows, which `shown_area` gives.
    let preview = app.ui.screen_mode != ScreenMode::Normal;
    let bg = if app.ui.screen_mode == ScreenMode::Presentation { Color32::BLACK } else { t.pasteboard };
    painter.rect_filled(screen, 0.0, bg);
    // Page shadow: a hard 1.5 pt black offset on the right and bottom (InDesign 2026).
    for slot in &layout.slots {
        let r = xf.rect(slot.bounds);
        painter.rect_filled(r.translate(vec2(1.5, 1.5)), 0.0, Color32::BLACK);
    }
    // Paper under the rendered content: a page scrolled or jumped to shows its paper colour,
    // not the shadow's black, until its render arrives.
    let [r, g, b, _] = doc.resolve_color(designcraft_color::swatch::PAPER, 1.0).unwrap_or(designcraft_color::Color::WHITE).to_rgba8(1.0);
    let paper = Color32::from_rgb(r, g, b);
    for slot in &layout.slots {
        for page in doc.spread(slot.spread).map(|sp| sp.pages.as_slice()).unwrap_or_default() {
            painter.rect_filled(xf.rect(slot.xf.transform_rect_bbox(page.bounds())), 0.0, paper);
        }
    }
    // Rendered content.
    render_texture(app, ui.ctx(), rect, &xf, &layout, preview);
    if let (Some(tex), Some(sh)) = (&app.canvas.texture, app.canvas.shown) {
        // Place the texture where its region is in the current view (shifted while panning,
        // scaled while a zoom re-render is pending).
        // The texture's corners in canvas space, placed through the view (rotation included).
        let (w, h) = (sh.size.0 as f64 / sh.zoom, sh.size.1 as f64 / sh.zoom);
        let o = sh.origin;
        let corners = [
            (o, pos2(0.0, 0.0)),
            (Point::new(o.x + w, o.y), pos2(1.0, 0.0)),
            (Point::new(o.x + w, o.y + h), pos2(1.0, 1.0)),
            (Point::new(o.x, o.y + h), pos2(0.0, 1.0)),
        ];
        let mut mesh = egui::Mesh::with_texture(tex.id());
        for (c, uv) in corners {
            mesh.vertices.push(egui::epaint::Vertex { pos: xf.to_screen(c), uv, color: Color32::WHITE });
        }
        mesh.indices.extend([0, 1, 2, 0, 2, 3]);
        if preview {
            // Preview: only the shown area of each spread keeps its content.
            for slot in &layout.slots {
                let r = shown_area(app, &doc, &xf, slot);
                painter.with_clip_rect(r.intersect(screen)).add(egui::Shape::mesh(mesh.clone()));
            }
        } else {
            painter.add(egui::Shape::mesh(mesh));
        }
    }
    crate::separations::paint(app, ui.ctx(), &painter, &xf, &layout, screen);
    let hair = 1.0 / ui.ctx().pixels_per_point();
    for slot in &layout.slots {
        painter.rect_stroke(xf.rect(slot.bounds), 0.0, Stroke::new(hair, Color32::from_rgb(0x4a, 0x4a, 0x4a)), StrokeKind::Outside);
    }
    if !preview {
        let drag = ui.data(|d| d.get_temp::<GuideDrag>(egui::Id::unique(GUIDE_DRAG)));
        let press = ui.data(|d| d.get_temp::<GuidePress>(egui::Id::unique(GUIDE_PRESS)));
        let moving = drag.as_ref().zip(press).and_then(|(g, press)| moving_guides(app, &xf, rect, g, press));
        draw_guides(app, &painter, &xf, &doc, &layout, &t, moving.as_ref());
        if let Some(g) = drag
            && let Some(p) = g.at
        {
            let col = Color32::from_rgb(74, 227, 255);
            let spot = guide_spot(app, &xf, &g);
            let line = spot.as_ref().map_or(p, |s| s.screen);
            let (a, b) = match g.orientation {
                designcraft_doc::Orientation::Horizontal => (pos2(rect.min.x, line.y), pos2(rect.max.x, line.y)),
                designcraft_doc::Orientation::Vertical => (pos2(line.x, rect.min.y), pos2(line.x, rect.max.y)),
            };
            // What caught it, as for a moved object.
            for mark in spot.iter().flat_map(|s| &s.marks) {
                if let Overlay::Guide { a, b } = mark {
                    painter.line_segment([xf.to_screen(*a), xf.to_screen(*b)], Stroke::new(1.0, t.smart_guide));
                }
            }
            // A new guide from the rulers with the Page tool is a liquid guide: dashed.
            if g.from.is_none() && app.session.tool_id() == "page" {
                dashed(&painter, a, b, Stroke::new(1.0, col), 6.0, 3.0);
            } else {
                painter.line_segment([a, b], Stroke::new(1.0, col));
            }
            // Where it is, beside the pointer (#1282).
            if let Some(text) = guide_readout(app, &xf, &g) {
                let galley = painter.layout_no_wrap(text, egui::FontId::proportional(11.0), t.text);
                let at = readout_at(p, galley.size(), rect);
                let bg = Rect::from_min_size(at, galley.size()).expand(3.0);
                painter.rect_filled(bg, 2.0, t.panel);
                painter.rect_stroke(bg, 2.0, Stroke::new(1.0, t.border), StrokeKind::Inside);
                painter.galley(at, galley, t.text);
            }
        }
        draw_frames(app, &painter, &xf, &doc, &layout);
        if app.ui.hidden_characters {
            draw_hidden_characters(app, &painter, &xf, &doc, &layout);
        }
        if app.ui.flattener_preview {
            draw_flattener_preview(&painter, &xf, &doc, &layout);
        }
        if app.ui.dynamic_spelling && !preview {
            draw_dynamic_spelling(app, ui.ctx(), &painter, &xf, &doc, &layout);
        }
    }
    if presenting {
        return;
    }
    let sel_rect = draw_selection(app, &painter, &xf, &doc, &layout);
    // Where the pointer moved on to another of the app's windows, this window's last pointer
    // position is stale: no hover feedback or tool cursor there.
    let elsewhere = pointer_elsewhere(app, ui.ctx());
    if !preview {
        // Content grabbers: the selected frame's and the hovered one's (Selection tool).
        let hover = ui.ctx().pointer_hover_pos().filter(|p| screen.contains(*p) && !elsewhere).map(|p| xf.to_canvas(p));
        let vi = app.view_info();
        for c in app.session.content_grabbers(hover, vi) {
            let c = xf.to_screen(c);
            painter.circle_stroke(c, 8.0, Stroke::new(1.5, Color32::from_white_alpha(170)));
            painter.circle_stroke(c, 4.0, Stroke::new(1.5, Color32::from_white_alpha(170)));
        }
    }
    draw_tool_overlays(app, &painter, &xf);
    ime_canvas_pass(app, ui.ctx(), &xf);
    if rulers {
        draw_rulers(app, ui, full, rect, &xf, &doc, &layout, &t);
        ruler_units_menus(app, &resp, full, rect, &doc);
    }
    context_menu(app, ui, &resp);
    if let Some(r) = sel_rect
        && !preview
        && r.intersects(screen)
    {
        crate::taskbar::show(app, &ui.ctx().clone(), r, screen);
    }
    // Cursor.
    if let Some(p) = resp.hover_pos().filter(|p| screen.contains(*p) && !elsewhere) {
        let wants_text = app.session.wants_text();
        let space = navigation_space(app, ui.ctx());
        let zoom = spring_zoom_mode(app, ui.ctx(), space);
        let middle = ui.input(|i| i.pointer.middle_down());
        let alt_hand: bool = ui.data(|d| d.get_temp(egui::Id::unique(("canvas_alt_hand", app.pane)))).unwrap_or(false);
        let tool_drag: bool = ui.data(|d| d.get_temp(egui::Id::unique(("canvas_pointer_down", app.pane)))).unwrap_or(false);
        let alt_text = wants_text && !tool_drag && ui.input(|i| i.modifiers.alt);
        let c = if middle || alt_hand {
            Cursor::HandGrab
        } else if zoom {
            if ui.input(|i| i.modifiers.alt) { Cursor::ZoomOut } else { Cursor::ZoomIn }
        } else if (space && !ui.input(|i| i.modifiers.command)) || alt_text {
            Cursor::Hand
        } else {
            app.session.cursor(xf.to_canvas(p), ui.input(|i| mods(i, false)), app.view_info())
        };
        // A column guide that a press would drag, or one being dragged: a resize cursor across it.
        let dragging = ui.data(|d| d.get_temp::<ColumnDrag>(egui::Id::unique((COLUMN_DRAG, app.pane)))).map(|g| g.spread);
        let column = dragging.or_else(|| {
            let menu_open = context_menu_open(ui.ctx());
            (!space && !zoom && !middle && !alt_hand && !tool_drag && !menu_open && xf.rot == 0)
                .then(|| column_guide_at(app, &xf, p).map(|h| h.0))
                .flatten()
        });
        let c = match column {
            Some(r) if spread_x_across(app, r) => Cursor::ResizeH,
            Some(_) => Cursor::ResizeV,
            None => c,
        };
        let icon = crate::cursors::show(ui.ctx(), &mut app.cursors, c, p);
        ui.ctx().set_cursor_icon(icon);
        draw_cursor_badge(&painter, p, c, &t);
    }
    // A second viewport has no panels, dialogs or Quick Apply after its canvas: observe here,
    // once, as the end of its pass. The root viewport observes at the end of `DesignApp::ui`.
    if ui.ctx().viewport_id() != egui::ViewportId::ROOT {
        ime_observe(app, ui.ctx());
    }
}

/// Whether the pointer this canvas's window last saw is in another of the app's windows: a
/// floating panel's OS window, or the other window of New Window.
fn pointer_elsewhere(app: &DesignApp, ctx: &egui::Context) -> bool {
    let mut others = crate::panel_docking::native_viewports(app, ctx);
    if app.second_window && !ctx.embed_viewports() {
        others.extend([egui::ViewportId::ROOT, crate::second_window_viewport()]);
    }
    crate::cursors::pointer_elsewhere(ctx, &others)
}

/// The loaded text cursor's badge beside the system cursor: lines of text, a closed chain
/// (a click threads) or a broken one (a click unthreads). Drawn in code.
fn draw_cursor_badge(painter: &egui::Painter, p: Pos2, c: Cursor, t: &Tokens) {
    if !matches!(c, Cursor::LoadedText | Cursor::ThreadLink | Cursor::Unthread) {
        return;
    }
    let r = Rect::from_min_size(p + vec2(12.0, 12.0), vec2(20.0, 16.0));
    painter.rect_filled(r, 3.0, t.measure_bg);
    let ink = Stroke::new(1.5, t.measure_text);
    let c0 = r.center();
    match c {
        Cursor::LoadedText => {
            for (dy, w) in [(-4.0, 12.0), (0.0, 12.0), (4.0, 8.0)] {
                painter.line_segment([pos2(c0.x - 6.0, c0.y + dy), pos2(c0.x - 6.0 + w, c0.y + dy)], ink);
            }
        }
        _ => {
            let gap = if c == Cursor::Unthread { 2.5 } else { -2.0 };
            for side in [-1.0, 1.0] {
                let link = Rect::from_center_size(pos2(c0.x + side * (3.5 + gap / 2.0), c0.y), vec2(9.0, 6.0));
                painter.rect_stroke(link, 3.0, ink, StrokeKind::Middle);
            }
            if c == Cursor::Unthread {
                painter.line_segment([pos2(c0.x + 2.0, c0.y - 6.0), pos2(c0.x - 2.0, c0.y + 6.0)], ink);
            }
        }
    }
}

/// Render options for the canvas texture.
fn screen_options(app: &DesignApp, layout: &CanvasLayout, preview: bool, ppp: f64) -> designcraft_render::RenderOptions {
    designcraft_render::RenderOptions {
        printing_only: preview,
        greek_below_px: 3.0 * ppp,
        highlight_missing_fonts: !preview && app.session.prefs.highlight_substituted_fonts,
        highlight_hj: !preview && app.session.prefs.highlight_hj,
        condition_indicators: !preview,
        note_indicators: !preview && app.session.prefs.show_note_anchors,
        change_markup: !preview && app.session.prefs.show_added_text,
        tag_markers: !preview && app.ui.tag_markers,
        highlight_keeps: !preview && app.session.prefs.highlight_keeps,
        highlight_custom_tracking: !preview && app.session.prefs.highlight_custom_tracking,
        quality: app.ui.display_quality,
        rich_black: app.ui.rich_black,
        overprint_preview: app.ui.overprint_preview,
        blend_space_view: !app.ui.proof_colors,
        document_grid: if app.ui.document_grid && !preview { grid_areas(layout) } else { vec![] },
        ..Default::default()
    }
}

/// The area each spread's document grid covers, in spread coordinates: the full-width band of
/// the pasteboard nearest that spread, the band in which `CanvasLayout::spread_at` (and so
/// snapping) picks it.
fn grid_areas(layout: &CanvasLayout) -> Vec<(designcraft_doc::SpreadRef, DRect)> {
    let pb = layout.pasteboard;
    let slots = &layout.slots;
    slots
        .iter()
        .enumerate()
        .map(|(i, slot)| {
            let lo = i.checked_sub(1).and_then(|j| slots.get(j)).map_or(pb.y0, |prev| (prev.bounds.y1 + slot.bounds.y0) / 2.0);
            let hi = slots.get(i + 1).map_or(pb.y1, |next| (slot.bounds.y1 + next.bounds.y0) / 2.0);
            (slot.spread, slot.xf.inverse().transform_rect_bbox(DRect::new(pb.x0, lo, pb.x1, hi)))
        })
        .collect()
}

/// The display options that change how the canvas texture looks without touching the document.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DisplayKey {
    quality: designcraft_render::DisplayQuality,
    rich_black: bool,
    overprint_preview: bool,
    proof_colors: bool,
    tag_markers: bool,
    document_grid: bool,
    highlight_substituted_fonts: bool,
    highlight_hj: bool,
    show_note_anchors: bool,
    show_added_text: bool,
    highlight_keeps: bool,
    highlight_custom_tracking: bool,
}

fn display_key(app: &DesignApp) -> DisplayKey {
    let (ui, prefs) = (&app.ui, &app.session.prefs);
    DisplayKey {
        quality: ui.display_quality,
        rich_black: ui.rich_black,
        overprint_preview: ui.overprint_preview,
        proof_colors: ui.proof_colors,
        tag_markers: ui.tag_markers,
        document_grid: ui.document_grid,
        highlight_substituted_fonts: prefs.highlight_substituted_fonts,
        highlight_hj: prefs.highlight_hj,
        show_note_anchors: prefs.show_note_anchors,
        show_added_text: prefs.show_added_text,
        highlight_keeps: prefs.highlight_keeps,
        highlight_custom_tracking: prefs.highlight_custom_tracking,
    }
}

/// When a display option changed, drop the texture and any render in flight: they were drawn with the old options.
fn invalidate_on_display_change(app: &mut DesignApp) {
    let key = display_key(app);
    if app.canvas.display_key != Some(key) {
        app.canvas.display_key = Some(key);
        app.canvas.shown = None;
        app.canvas.pending = None;
        app.canvas.pending_doc = None;
    }
}

/// Keep the canvas texture current. Panning inside the overscanned region just shifts the
/// texture; zooming shows the old texture scaled until the background render arrives; document
/// edits render synchronously so editing never lags.
fn render_texture(app: &mut DesignApp, ctx: &egui::Context, rect: Rect, xf: &Xf, layout: &CanvasLayout, preview: bool) {
    invalidate_on_display_change(app);
    let Some(st) = app.session.active() else { return };
    let ppp = ctx.pixels_per_point() as f64;
    let doc_key = (st.uid, std::sync::Arc::as_ptr(&st.doc) as u64, st.revision, preview);
    let editing_parents = st.editing_parents;
    // Overscan: 25% of the viewport on every side, within the GPU's maximum texture size.
    let max_side = ctx.input(|i| i.max_texture_side) as f32 / ppp as f32;
    let (ow, oh) =
        ((rect.width() * 0.25).min((max_side - rect.width()) / 2.0).max(0.0), (rect.height() * 0.25).min((max_side - rect.height()) / 2.0).max(0.0));
    let want = |origin: Point| crate::Shown {
        doc: doc_key,
        editing_parents,
        origin: Point::new(origin.x - ow as f64 / xf.zoom, origin.y - oh as f64 / xf.zoom),
        zoom: xf.zoom,
        size: (rect.width() + 2.0 * ow, rect.height() + 2.0 * oh),
    };
    let target = want(xf.origin);
    // Does the current texture cover the viewport at this zoom and document state?
    let covers = |s: &crate::Shown| {
        s.doc == doc_key && s.editing_parents == editing_parents && (s.zoom - xf.zoom).abs() < 1e-9 && {
            let x0 = (xf.origin.x - s.origin.x) * s.zoom;
            let y0 = (xf.origin.y - s.origin.y) * s.zoom;
            x0 >= -0.5 && y0 >= -0.5 && x0 + rect.width() as f64 <= s.size.0 as f64 + 0.5 && y0 + rect.height() as f64 <= s.size.1 as f64 + 0.5
        }
    };
    // Collect finished background renders.
    #[cfg(not(target_arch = "wasm32"))]
    if let (Some(w), Some((tok, shown))) = (app.canvas.worker.as_ref(), app.canvas.pending)
        && let Some(done) = w.poll()
        && done.token == tok
    {
        upload(app, ctx, done.image);
        app.canvas.shown = Some(shown);
        app.canvas.shown_doc = app.canvas.pending_doc.take();
        app.canvas.pending = None;
        app.perf.render_ms = done.ms;
    }
    if app.canvas.shown.is_some_and(|s| covers(&s)) {
        return;
    }
    let doc_changed = app.canvas.shown.is_none_or(|s| s.doc != doc_key);
    let mode_changed = app.canvas.shown.is_some_and(|s| s.editing_parents != editing_parents);
    let zoom_same = app.canvas.shown.is_some_and(|s| (s.zoom - xf.zoom).abs() < 1e-9);
    let job = |app: &DesignApp| {
        let st = app.session.active()?;
        let placed: Vec<designcraft_render::Placed> =
            layout.slots.iter().map(|s| designcraft_render::Placed { spread: s.spread, xf: s.xf }).collect();
        let w = (target.size.0 as f64 * ppp).round().max(1.0) as u32;
        let h = (target.size.1 as f64 * ppp).round().max(1.0) as u32;
        let view = Affine::scale(ppp) * Affine::scale(xf.zoom) * Affine::translate((-target.origin.x, -target.origin.y));
        let opts = screen_options(app, layout, preview, ppp);
        Some((st.doc.clone(), placed, w, h, view, opts))
    };
    // Synchronous path: document edits (keep editing crisp), first frame, or no worker.
    #[cfg(not(target_arch = "wasm32"))]
    if !app.canvas.worker_started {
        app.canvas.worker_started = true;
        app.canvas.worker = render_worker_spawn(ctx);
    }
    #[cfg(not(target_arch = "wasm32"))]
    let has_worker = app.canvas.worker.is_some();
    #[cfg(target_arch = "wasm32")]
    let has_worker = false;
    // An edit with the view unchanged: repaint just the regions it changed.
    if doc_changed && zoom_same && patch_texture(app, layout, ppp, doc_key, preview) {
        return;
    }
    // A mode switch changes every spread's meaning, even if the document has no damage.
    // Show the new canvas immediately, including when fitting it changes the zoom.
    if !has_worker || app.canvas.shown.is_none() || mode_changed || (doc_changed && zoom_same) {
        let t0 = crate::now_ms();
        let Some((doc, placed, w, h, view, opts)) = job(app) else { return };
        let img = app.canvas.renderer.render(&doc, &app.session.cache, &placed, w, h, view, &opts);
        upload(app, ctx, img);
        app.canvas.shown = Some(target);
        app.canvas.shown_doc = Some(doc);
        app.canvas.pending = None;
        app.canvas.pending_doc = None;
        app.perf.render_ms = crate::now_ms() - t0;
        return;
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        // Already rendering this view?
        if app.canvas.pending.is_some_and(|(_, p)| {
            p.doc == doc_key && p.editing_parents == editing_parents && (p.zoom - xf.zoom).abs() < 1e-9 && {
                let x0 = (xf.origin.x - p.origin.x) * p.zoom;
                let y0 = (xf.origin.y - p.origin.y) * p.zoom;
                x0 >= 0.0 && y0 >= 0.0 && x0 + rect.width() as f64 <= p.size.0 as f64 && y0 + rect.height() as f64 <= p.size.1 as f64
            }
        }) {
            return;
        }
        let Some((doc, placed, w, h, view, opts)) = job(app) else { return };
        app.canvas.pending_doc = Some(doc.clone());
        app.canvas.token += 1;
        let token = app.canvas.token;
        if let Some(wk) = app.canvas.worker.as_ref() {
            wk.submit(crate::render_worker::Job { token, doc, cache: app.session.cache.clone(), placed, w, h, view, opts });
        }
        app.canvas.pending = Some((token, target));
    }
}

/// Repaint only the damaged regions of the current texture after an edit (typing, nudging).
/// Returns false when a full render is needed (view moved, too much changed, or damage unknown).
fn patch_texture(app: &mut DesignApp, layout: &CanvasLayout, ppp: f64, doc_key: (u64, u64, u64, bool), preview: bool) -> bool {
    let (Some(sh), Some(old), Some(st)) = (app.canvas.shown, app.canvas.shown_doc.clone(), app.session.active()) else { return false };
    if sh.doc.0 != doc_key.0 || sh.doc.3 != doc_key.3 || sh.editing_parents != st.editing_parents {
        return false;
    }
    let (tw, th) = ((sh.size.0 as f64 * ppp).round() as i64, (sh.size.1 as f64 * ppp).round() as i64);
    // Patches are in full-resolution pixels: a texture `upload` shrank to the GPU's limit
    // (a viewport wider than `max_texture_side`) takes full renders instead.
    let full = [usize::try_from(tw).unwrap_or(0), usize::try_from(th).unwrap_or(0)];
    if app.canvas.texture.as_ref().is_none_or(|t| t.size() != full) {
        return false;
    }
    let new = st.doc.clone();
    let Some(regions) = designcraft_render::damage::damage_with(&old, &new, Some(&app.session.cache)) else { return false };
    let t0 = crate::now_ms();
    let k = sh.zoom * ppp;
    let mut px: Option<(i64, i64, i64, i64)> = None;
    for (r, rect) in &regions {
        let c = layout.xf(*r).transform_rect_bbox(*rect);
        let x0 = ((c.x0 - sh.origin.x) * k).floor() as i64 - 1;
        let y0 = ((c.y0 - sh.origin.y) * k).floor() as i64 - 1;
        let x1 = ((c.x1 - sh.origin.x) * k).ceil() as i64 + 1;
        let y1 = ((c.y1 - sh.origin.y) * k).ceil() as i64 + 1;
        px = Some(px.map_or((x0, y0, x1, y1), |p| (p.0.min(x0), p.1.min(y0), p.2.max(x1), p.3.max(y1))));
    }
    let done = |app: &mut DesignApp| {
        app.canvas.shown = Some(crate::Shown { doc: doc_key, ..sh });
        app.canvas.shown_doc = Some(new.clone());
        // A background render of the old document must not overwrite this.
        app.canvas.pending = None;
        app.canvas.pending_doc = None;
    };
    let Some((x0, y0, x1, y1)) = px.map(|p| (p.0.max(0), p.1.max(0), p.2.min(tw), p.3.min(th))) else {
        done(app);
        return true;
    };
    if x1 <= x0 || y1 <= y0 {
        done(app);
        return true;
    }
    // Big changes: a full render costs about the same.
    if ((x1 - x0) * (y1 - y0)) as f64 > 0.5 * (tw * th) as f64 {
        return false;
    }
    let placed: Vec<designcraft_render::Placed> = layout.slots.iter().map(|s| designcraft_render::Placed { spread: s.spread, xf: s.xf }).collect();
    let view = Affine::translate((-(x0 as f64), -(y0 as f64)))
        * Affine::scale(ppp)
        * Affine::scale(sh.zoom)
        * Affine::translate((-sh.origin.x, -sh.origin.y));
    let opts = screen_options(app, layout, preview, ppp);
    let img = app.canvas.patcher.render(&new, &app.session.cache, &placed, (x1 - x0) as u32, (y1 - y0) as u32, view, &opts);
    let ci = egui::ColorImage::from_rgba_premultiplied([img.width as usize, img.height as usize], &img.pixels);
    if let Some(tex) = app.canvas.texture.as_mut() {
        tex.set_partial([x0 as usize, y0 as usize], ci, egui::TextureOptions::LINEAR);
    }
    done(app);
    app.canvas.patches += 1;
    app.perf.render_ms = crate::now_ms() - t0;
    true
}

#[cfg(not(target_arch = "wasm32"))]
fn render_worker_spawn(ctx: &egui::Context) -> Option<crate::render_worker::Worker> {
    if std::env::var_os("DESIGNCRAFT_SYNC_RENDER").is_some() {
        return None;
    }
    crate::render_worker::Worker::spawn(ctx.clone())
}

fn upload(app: &mut DesignApp, ctx: &egui::Context, mut img: designcraft_render::Rendered) {
    if app.ui.proof_colors {
        designcraft_render::proof_view(&mut img, &app.ui.proof_setup);
    }
    let ci = egui::ColorImage::from_rgba_premultiplied([img.width as usize, img.height as usize], &img.pixels);
    let ci = crate::widgets::fit_texture(ci, ctx.input(|i| i.max_texture_side));
    match &mut app.canvas.texture {
        Some(tex) => tex.set(ci, egui::TextureOptions::LINEAR),
        None => app.canvas.texture = Some(ctx.load_texture("canvas", ci, egui::TextureOptions::LINEAR)),
    }
}

fn dashed(painter: &egui::Painter, a: Pos2, b: Pos2, stroke: Stroke, dash: f32, gap: f32) {
    painter.extend(egui::Shape::dashed_line(&[a, b], stroke, dash, gap));
}

/// The most baseline grid lines drawn on a page: one per point of the tallest page (216 in).
const MAX_BASELINES: usize = 216 * 72;

/// Baseline grid lines inside a page, measured from its top: `start + n * increment` for n >= 0 (the bottom
/// edge is not a line), `start` from [`BaselineGrid::origin`](designcraft_doc::BaselineGrid::origin). Lines
/// above the page are skipped without walking them, however far up the start is.
fn baseline_grid_lines(start: f64, increment: f64, height: f64) -> impl Iterator<Item = f64> {
    let step = increment.max(1.0);
    let first = if start < 0.0 { start.rem_euclid(step) } else { start };
    (0..MAX_BASELINES).map(move |n| first + n as f64 * step).take_while(move |y| *y < height)
}

/// What a screen mode shows of a spread, in screen coordinates: the trim, or out to the bleed or
/// the slug box. Slug shows every printing element, so the bleed as well — and a slug narrower
/// than the bleed does not pull the edge back in.
fn shown_area(app: &DesignApp, doc: &Document, xf: &Xf, slot: &SpreadSlot) -> Rect {
    let trim = xf.rect(slot.bounds);
    let boxes: &[[f64; 4]] = match app.ui.screen_mode {
        ScreenMode::Bleed => &[doc.settings.bleed],
        ScreenMode::Slug => &[doc.settings.bleed, doc.settings.slug],
        _ => &[],
    };
    let Some(sp) = doc.spread(slot.spread) else { return trim };
    boxes.iter().filter_map(|o| sp.outset_rect(*o)).map(|r| xf.rect(slot.xf.transform_rect_bbox(r))).fold(trim, |a, b| a.union(b))
}

/// `moving`: guides being dragged, drawn where they would be dropped instead of where they are.
fn draw_guides(app: &DesignApp, painter: &egui::Painter, xf: &Xf, doc: &Document, layout: &CanvasLayout, t: &Tokens, moving: Option<&MovingGuides>) {
    let selection: std::collections::HashSet<GuideSel> =
        app.session.active().map(|st| st.selection.guides.iter().copied().collect()).unwrap_or_default();
    for slot in &layout.slots {
        let Some(sp) = doc.spread(slot.spread) else { continue };
        if app.ui.guides {
            draw_bleed_and_slug(painter, xf, doc, slot, sp);
        }
        let pasteboard = slot.xf.inverse().transform_rect_bbox(layout.pasteboard);
        for (pi, p) in sp.pages.iter().enumerate() {
            let pr = p.bounds();
            draw_parent_overlay(painter, xf, doc, slot, pi, p, pr);
            if !app.ui.guides {
                continue;
            }
            draw_page_grids(painter, xf, doc, slot, p, pr, app.ui.baseline_grid);
            draw_page_guides(painter, xf, doc, slot, sp, pi, p, pr, pasteboard, moving, &selection);
        }
    }
    let _ = t;
}

/// The bleed and slug outlines of a spread, in their own colours.
fn draw_bleed_and_slug(painter: &egui::Painter, xf: &Xf, doc: &Document, slot: &SpreadSlot, sp: &designcraft_doc::Spread) {
    let s = &doc.settings;
    let rect = |r: DRect| xf.rect(slot.xf.transform_rect_bbox(r));
    // Bleed, then slug: one rectangle each around the spread (the inside offset only applies
    // at the spread's outer edges).
    for (r, color) in [(sp.bleed_rect(s.bleed), s.bleed_color), (sp.slug_rect(s.slug), s.slug_color)] {
        if let Some(r) = r {
            painter.rect_stroke(rect(r), 0.0, Stroke::new(hair(painter), c32(color)), StrokeKind::Middle);
        }
    }
}

/// Page tool › Show Parent Page Overlay: the parent page's size over the page.
fn draw_parent_overlay(painter: &egui::Painter, xf: &Xf, doc: &Document, slot: &SpreadSlot, pi: usize, p: &designcraft_doc::Page, pr: DRect) {
    let rect = |r: DRect| xf.rect(slot.xf.transform_rect_bbox(r));
    if p.show_parent_overlay
        && let SpreadRef::Doc(si) = slot.spread
        && let Some((ppi, ppg)) = doc.parent_page_for(doc.first_page_of_spread(si) + pi)
        && let Some(pp) = doc.parents.get(ppi).and_then(|ps| ps.pages.get(ppg))
    {
        let r = rect(DRect::new(pr.x0, pr.y0, pr.x0 + pp.width, pr.y0 + pp.height));
        painter.rect_filled(r, 0.0, Color32::from_rgba_unmultiplied(110, 110, 230, 28));
        painter.rect_stroke(r, 0.0, Stroke::new(1.0, Color32::from_rgb(110, 110, 230)), StrokeKind::Middle);
    }
}

/// A page's baseline grid (when `baseline` is on and the view is zoomed in far enough), margins
/// and columns.
fn draw_page_grids(painter: &egui::Painter, xf: &Xf, doc: &Document, slot: &SpreadSlot, p: &designcraft_doc::Page, pr: DRect, baseline: bool) {
    let s = &doc.settings;
    let pt = |q: Point| xf.to_screen(slot.xf * q);
    let rect = |r: DRect| xf.rect(slot.xf.transform_rect_bbox(r));
    // Baseline grid.
    if baseline && xf.zoom >= s.baseline_grid.view_threshold {
        let g = &s.baseline_grid;
        for y in baseline_grid_lines(g.origin(p), g.increment, p.height) {
            painter.line_segment(
                [pt(Point::new(pr.x0, pr.y0 + y)), pt(Point::new(pr.x1, pr.y0 + y))],
                Stroke::new(hair(painter), c32(g.color).gamma_multiply(0.8)),
            );
        }
    }
    // Margins (magenta) and columns (violet).
    painter.rect_stroke(rect(p.margin_rect()), 0.0, Stroke::new(hair(painter), c32(s.margin_color)), StrokeKind::Middle);
    let cols = p.column_rects();
    if cols.len() > 1 {
        for (i, c) in cols.iter().enumerate() {
            let col = Stroke::new(hair(painter), c32(s.column_color));
            if i > 0 {
                painter.line_segment([pt(Point::new(c.x0, c.y0)), pt(Point::new(c.x0, c.y1))], col);
            }
            if i + 1 < cols.len() {
                painter.line_segment([pt(Point::new(c.x1, c.y0)), pt(Point::new(c.x1, c.y1))], col);
            }
        }
    }
}

/// The page's own guides, then those it shows from its parent page. `moving` draws dragged
/// guides where they would be dropped.
fn draw_page_guides(
    painter: &egui::Painter,
    xf: &Xf,
    doc: &Document,
    slot: &SpreadSlot,
    sp: &designcraft_doc::Spread,
    pi: usize,
    p: &designcraft_doc::Page,
    pr: DRect,
    pasteboard: DRect,
    moving: Option<&MovingGuides>,
    selection: &std::collections::HashSet<GuideSel>,
) {
    let own = p.guides.len();
    for (gi, mut g) in p.guides.iter().cloned().chain(doc.parent_guides(p)).enumerate().filter(|(_, g)| g.visible_in(doc)) {
        let key = GuideSel { spread: slot.spread, page: pi, index: gi };
        // Selected guides are drawn thicker, in their layer's color.
        let selected = gi < own && selection.contains(&key);
        if gi < own
            && let Some(m) = moving
            && m.guides.contains(&key)
        {
            g.position += match g.orientation {
                designcraft_doc::Orientation::Vertical => m.dx,
                designcraft_doc::Orientation::Horizontal => m.dy,
            };
        }
        draw_guide(painter, xf, doc, slot, sp, pr, pasteboard, &g, selected);
    }
}

/// One guide as a line: across the pasteboard for a spread guide, across its page otherwise.
fn draw_guide(
    painter: &egui::Painter,
    xf: &Xf,
    doc: &Document,
    slot: &SpreadSlot,
    sp: &designcraft_doc::Spread,
    pr: DRect,
    pasteboard: DRect,
    g: &designcraft_doc::Guide,
    selected: bool,
) {
    let pt = |q: Point| xf.to_screen(slot.xf * q);
    // Spread guides cross the pasteboard; page guides their page.
    let span = if g.spread { pasteboard } else { pr };
    let (a, bb) = match g.orientation {
        designcraft_doc::Orientation::Horizontal => (Point::new(span.x0, g.position), Point::new(span.x1, g.position)),
        designcraft_doc::Orientation::Vertical => {
            let span = if g.spread { sp.bounds().inflate(0.0, 36.0) } else { pr };
            (Point::new(g.position, span.y0), Point::new(g.position, span.y1))
        }
    };
    let stroke = if selected {
        let color = g.layer.and_then(|l| doc.layer(l)).map_or(Color32::from_rgb(79, 153, 255), |l| c32(l.color));
        Stroke::new(2.0, color)
    } else {
        Stroke::new(hair(painter), Color32::from_rgb(74, 227, 255))
    };
    if g.liquid {
        // Liquid guides are dashed.
        painter.extend(egui::Shape::dashed_line(&[pt(a), pt(bb)], stroke, 6.0, 3.0));
    } else {
        painter.line_segment([pt(a), pt(bb)], stroke);
    }
}

/// Spread-space → canvas transform for an item at `loc`.
fn item_canvas_xf(doc: &Document, layout: &CanvasLayout, id: designcraft_doc::ItemId) -> Option<(Affine, &'static str)> {
    let loc = doc.find(id)?;
    Some((layout.xf(loc.spread) * doc.parent_xf(&loc), ""))
}

fn path_screen(it: &Item, xf: &Xf, a: Affine) -> Vec<Vec<Pos2>> {
    let bp = it.path.to_bezpath();
    let full = a * it.xf;
    let mut out = Vec::new();
    let mut cur: Vec<Pos2> = Vec::new();
    designcraft_geom::kurbo::flatten(&bp, 0.25 / xf.zoom.max(0.01), |el| {
        use designcraft_geom::PathEl::*;
        match el {
            MoveTo(p) => {
                if cur.len() > 1 {
                    out.push(std::mem::take(&mut cur));
                }
                cur.clear();
                cur.push(xf.to_screen(full * p));
            }
            LineTo(p) => cur.push(xf.to_screen(full * p)),
            ClosePath => {
                if let Some(f) = cur.first().copied() {
                    cur.push(f);
                }
            }
            _ => {}
        }
    });
    if cur.len() > 1 {
        out.push(cur);
    }
    out
}

fn draw_frames(app: &DesignApp, painter: &egui::Painter, xf: &Xf, doc: &Document, layout: &CanvasLayout) {
    if !app.ui.frame_edges {
        return;
    }
    let ported = ported_frames(app, doc);
    for slot in &layout.slots {
        let Some(sp) = doc.spread(slot.spread) else { continue };
        let a = slot.xf;
        // Parent items on document pages, and base parents' items on a parent spread: dotted edges.
        for (pi, page) in sp.pages.iter().enumerate() {
            for (ppi, ppg) in doc.shown_parent_pages(slot.spread, pi) {
                let Some(parent) = doc.parents.get(ppi) else { continue };
                let Some(ppage) = parent.pages.get(ppg) else { continue };
                let dx = page.x - ppage.x;
                for it in &parent.items {
                    if (parent.pages.len() > 1 && parent.page_at_x(it.bounds().center().x) != Some(ppg)) || page.overridden.contains(&it.id) {
                        continue;
                    }
                    let col = doc.layer(it.layer).map(|l| c32(l.color)).unwrap_or(Color32::LIGHT_BLUE);
                    for poly in path_screen(it, xf, a * Affine::translate((dx, 0.0))) {
                        for w in poly.windows(2) {
                            dashed(painter, w[0], w[1], Stroke::new(hair(painter), col.gamma_multiply(0.8)), 2.0, 2.0);
                        }
                    }
                }
            }
        }
        for it in &sp.items {
            draw_item_edges(painter, xf, doc, it, a);
        }
        // Every overset frame, selected or not and inside groups too, shows its red mark at the
        // out port (InDesign shows it only on the selected frame; a report in Preflight is easy
        // to miss while laying out).
        for (it, c) in overset_marks(app, doc, layout, &sp.items, xf.zoom, &ported) {
            let c = xf.to_screen(c);
            let s = designcraft_tools::thread::PORT_SIZE_PX as f32;
            painter.rect_filled(Rect::from_center_size(c, vec2(s, s)), 0.0, Color32::WHITE);
            painter.rect_stroke(Rect::from_center_size(c, vec2(s, s)), 0.0, Stroke::new(hair(painter), layer_color(doc, it)), StrokeKind::Inside);
            let red = Color32::from_rgb(230, 20, 20);
            painter.line_segment([c - vec2(2.5, 0.0), c + vec2(2.5, 0.0)], Stroke::new(1.5, red));
            painter.line_segment([c - vec2(0.0, 2.5), c + vec2(0.0, 2.5)], Stroke::new(1.5, red));
        }
        // Tagged frames in their tag's colour.
        if app.ui.tagged_frames {
            for top in &sp.items {
                top.walk(&mut |it: &Item| {
                    if it.xml_tag.is_empty() {
                        return;
                    }
                    let Some(t) = doc.xml.tags.iter().find(|t| t.name == it.xml_tag) else { return };
                    let col = Color32::from_rgb(t.color[0], t.color[1], t.color[2]);
                    for poly in path_screen(it, xf, a) {
                        for w in poly.windows(2) {
                            painter.line_segment([w[0], w[1]], Stroke::new(2.5, col));
                        }
                    }
                });
            }
        }
    }
}

fn draw_item_edges(painter: &egui::Painter, xf: &Xf, doc: &Document, it: &Item, a: Affine) {
    if it.hidden || doc.layer(it.layer).is_some_and(|l| !l.visible) {
        return;
    }
    let col = doc.layer(it.layer).map(|l| c32(l.color)).unwrap_or(Color32::LIGHT_BLUE);
    if let Content::Group { items } = &it.content {
        for c in items {
            draw_item_edges(painter, xf, doc, c, a * it.xf);
        }
        return;
    }
    // Frames without a stroke show their edge; shapes with strokes are visible already.
    let show = it.stroke.is_none() || matches!(it.content, Content::Text(_) | Content::Graphic(_));
    if show {
        for poly in path_screen(it, xf, a) {
            painter.add(egui::Shape::line(poly, Stroke::new(hair(painter), col.gamma_multiply(0.75))));
        }
    }
    // Empty graphic frame: the X.
    if it.object_style == designcraft_doc::BASIC_GRAPHICS_FRAME && matches!(it.content, Content::Unassigned) {
        let r = it.inner_bounds();
        let m = a * it.xf;
        let p = |x: f64, y: f64| xf.to_screen(m * Point::new(x, y));
        painter.line_segment([p(r.x0, r.y0), p(r.x1, r.y1)], Stroke::new(hair(painter), col.gamma_multiply(0.75)));
        painter.line_segment([p(r.x1, r.y0), p(r.x0, r.y1)], Stroke::new(hair(painter), col.gamma_multiply(0.75)));
    }
}

/// Layer colour of the first selected item: live-corner marks are outlined in it.
fn selection_color(app: &DesignApp) -> Color32 {
    let fallback = Color32::from_rgb(79, 153, 255);
    let Some(st) = app.session.active() else { return fallback };
    st.selection.items.first().and_then(|id| st.doc.item(*id)).map_or(fallback, |it| layer_color(&st.doc, it))
}

fn layer_color(doc: &Document, it: &Item) -> Color32 {
    doc.layer(it.layer).map(|l| c32(l.color)).unwrap_or(Color32::from_rgb(79, 153, 255))
}

/// Draw selection chrome; returns the selection's screen bounds (for the Contextual Task Bar).
fn draw_selection(app: &DesignApp, painter: &egui::Painter, xf: &Xf, doc: &Document, layout: &CanvasLayout) -> Option<Rect> {
    let st = app.session.active()?;
    let sel: &Selection = &st.selection;
    let mut text_rect = None;
    // Text selection / caret.
    if let Some(ts) = sel.text {
        draw_text_selection(app, painter, xf, doc, layout, ts, sel.cells);
        let fid = ts.frame.or_else(|| {
            let cs = app.session.cache.get(doc, ts.story, None);
            compose::caret(&cs, ts.focus).and_then(|c| cs.frames.get(c.0)).map(|f| f.frame)
        });
        if let Some(fid) = fid
            && let (Some(it), Some((a, _))) = (doc.item(fid), item_canvas_xf(doc, layout, fid))
        {
            text_rect = Some(xf.rect(a.transform_rect_bbox(it.bounds())));
        }
    }
    let mut union: Option<Rect> = None;
    let mut color = Color32::from_rgb(79, 153, 255);
    for id in &sel.items {
        let (Some(it), Some((a, _))) = (doc.item(*id), item_canvas_xf(doc, layout, *id)) else { continue };
        color = layer_color(doc, it);
        for poly in path_screen(it, xf, a) {
            painter.add(egui::Shape::line(poly, Stroke::new(1.0, color)));
        }
        if sel.items.len() == 1 && sel.text.is_none() {
            // InDesign draws a 1 pt bounding box around the selection.
            painter.rect_stroke(xf.rect(a.transform_rect_bbox(it.bounds())), 0.0, Stroke::new(1.0, color), StrokeKind::Middle);
        } else if sel.key == Some(*id) {
            // The key object for Align: a thicker bounding box, as in InDesign.
            painter.rect_stroke(xf.rect(a.transform_rect_bbox(it.bounds())), 0.0, Stroke::new(3.0, color), StrokeKind::Middle);
        }
        let b = xf.rect(a.transform_rect_bbox(it.bounds()));
        union = Some(union.map_or(b, |u| u.union(b)));
        // Text frame ports, also of the frames inside a selected group (so an overset frame in a
        // group shows its red mark).
        let mut frames = Vec::new();
        text_frames_in(it, 0, &mut frames);
        for (frame, story) in frames {
            draw_ports(app, painter, xf, doc, layout, frame, story, color);
        }
        if let Content::Graphic(g) = &it.content
            && sel.content
        {
            let gb = xf.rect((a * it.xf).transform_rect_bbox(g.xf.transform_rect_bbox(DRect::new(0.0, 0.0, g.size.0, g.size.1))));
            let brown = Color32::from_rgb(196, 111, 43);
            painter.rect_stroke(gb, 0.0, Stroke::new(1.0, brown), StrokeKind::Middle);
            // Its own handles resize the graphic (InDesign's brown content bounding box).
            if sel.items.len() == 1 {
                for h in designcraft_tools::select::handles(DRect::new(gb.min.x as f64, gb.min.y as f64, gb.max.x as f64, gb.max.y as f64)) {
                    let r = Rect::from_center_size(pos2(h.x as f32, h.y as f32), vec2(6.5, 6.5));
                    painter.rect_filled(r, 0.0, Color32::WHITE);
                    painter.rect_stroke(r, 0.0, Stroke::new(hair(painter), brown), StrokeKind::Inside);
                }
            }
        }
    }
    let direct = matches!(app.session.tool_id(), "directSelection" | "pen" | "addAnchor" | "deleteAnchor" | "convertDirection");
    if direct {
        for id in &sel.items {
            let (Some(it), Some((a, _))) = (doc.item(*id), item_canvas_xf(doc, layout, *id)) else { continue };
            let m = a * it.xf;
            let col = layer_color(doc, it);
            for sp in &it.path.subpaths {
                for an in &sp.anchors {
                    let p = xf.to_screen(m * an.p);
                    for (h, has) in [(an.h_in, an.has_in()), (an.h_out, an.has_out())] {
                        if has {
                            let hp = xf.to_screen(m * h);
                            painter.line_segment([p, hp], Stroke::new(1.0, col));
                            painter.circle_filled(hp, 2.5, col);
                        }
                    }
                    let r = Rect::from_center_size(p, vec2(5.0, 5.0));
                    painter.rect_filled(r, 0.0, Color32::WHITE);
                    painter.rect_stroke(r, 0.0, Stroke::new(1.0, col), StrokeKind::Inside);
                }
            }
        }
    }
    if let Some(u) = union
        && sel.text.is_none()
        && !direct
    {
        if sel.items.len() > 1 {
            painter.rect_stroke(u, 0.0, Stroke::new(1.0, color), StrokeKind::Middle);
        }
        let hs = 6.5;
        // Centre point. The Selection tool draws the live-corner widget as an overlay.
        painter.rect_filled(Rect::from_center_size(u.center(), vec2(3.5, 3.5)), 0.0, color);
        // A selected placed graphic has its own handles instead of the frame's.
        let graphic_selected =
            sel.content && sel.items.len() == 1 && doc.item(sel.items[0]).is_some_and(|i| matches!(i.content, Content::Graphic(_)));
        for h in designcraft_tools::select::handles(DRect::new(u.min.x as f64, u.min.y as f64, u.max.x as f64, u.max.y as f64))
            .into_iter()
            .filter(|_| !graphic_selected)
        {
            let r = Rect::from_center_size(pos2(h.x as f32, h.y as f32), vec2(hs, hs));
            painter.rect_filled(r, 0.0, Color32::WHITE);
            painter.rect_stroke(r, 0.0, Stroke::new(hair(painter), color), StrokeKind::Inside);
        }
    }
    // Threads.
    if app.ui.text_threads || sel.items.iter().any(|i| doc.item(*i).is_some_and(|x| x.is_text_frame())) {
        for id in &sel.items {
            let Some(sid) = doc.item(*id).and_then(|i| i.text_frame()).map(|t| t.story) else { continue };
            let Some(story) = doc.story(sid) else { continue };
            for w in story.frames.windows(2) {
                let (Some(a), Some(b)) = (port_pos(doc, layout, w[0], true), port_pos(doc, layout, w[1], false)) else { continue };
                let col = doc.item(w[0]).map(|i| layer_color(doc, i)).unwrap_or(color);
                painter.line_segment([xf.to_screen(a), xf.to_screen(b)], Stroke::new(1.0, col));
            }
        }
    }
    text_rect.or(union)
}

/// How deep a selected group is walked for the text frames whose ports are drawn.
const PORT_GROUP_DEPTH: usize = 32;

/// The text frames whose ports `draw_selection` draws: the selected text frames and the text
/// frames inside selected groups.
fn ported_frames(app: &DesignApp, doc: &Document) -> std::collections::HashSet<designcraft_doc::ItemId> {
    let mut frames = Vec::new();
    if let Some(st) = app.session.active() {
        for it in st.selection.items.iter().filter_map(|id| doc.item(*id)) {
            text_frames_in(it, 0, &mut frames);
        }
    }
    frames.into_iter().map(|(f, _)| f.id).collect()
}

/// Canvas centres of the always-on overset marks among `items`: the out port of every visible
/// frame that ends an overset story, except the frames in `ported`, whose out port `draw_ports`
/// already marks.
fn overset_marks<'a>(
    app: &DesignApp,
    doc: &Document,
    layout: &CanvasLayout,
    items: &'a [std::sync::Arc<Item>],
    zoom: f64,
    ported: &std::collections::HashSet<designcraft_doc::ItemId>,
) -> Vec<(&'a Item, Point)> {
    let mut frames = Vec::new();
    overset_frames(app, doc, items, 0, &mut frames);
    frames
        .into_iter()
        .filter(|it| !ported.contains(&it.id))
        .filter_map(|it| Some((it, designcraft_tools::thread::port_center(doc, layout, it.id, true, zoom)?)))
        .collect()
}

/// The visible text frames among `items` (groups walked) that end an overset story.
fn overset_frames<'a>(app: &DesignApp, doc: &Document, items: &'a [std::sync::Arc<Item>], depth: usize, out: &mut Vec<&'a Item>) {
    if depth > PORT_GROUP_DEPTH {
        return;
    }
    for it in items {
        if it.hidden || doc.layer(it.layer).is_some_and(|l| !l.visible) {
            continue;
        }
        match &it.content {
            Content::Text(tf) => {
                let Some(story) = doc.story(tf.story) else { continue };
                if story.frames.last() == Some(&it.id) && app.session.cache.get(doc, tf.story, None).is_overset() {
                    out.push(it);
                }
            }
            Content::Group { items } => overset_frames(app, doc, items, depth + 1, out),
            _ => {}
        }
    }
}

/// `it` when it is a text frame, or every text frame nested in it when it is a group, each with
/// its story.
fn text_frames_in<'a>(it: &'a Item, depth: usize, out: &mut Vec<(&'a Item, designcraft_doc::StoryId)>) {
    match &it.content {
        Content::Text(tf) => out.push((it, tf.story)),
        Content::Group { items } if depth < PORT_GROUP_DEPTH => {
            for child in items {
                text_frames_in(child, depth + 1, out);
            }
        }
        _ => {}
    }
}

/// Port location (canvas): out port at bottom-right, in port at top-left.
fn port_pos(doc: &Document, layout: &CanvasLayout, id: designcraft_doc::ItemId, out: bool) -> Option<Point> {
    let it = doc.item(id)?;
    let (a, _) = item_canvas_xf(doc, layout, id)?;
    let r = it.inner_bounds();
    let p = if out { Point::new(r.x1 - 10.0, r.y1) } else { Point::new(r.x0 + 10.0, r.y0) };
    Some(a * it.xf * p)
}

#[allow(clippy::too_many_arguments)]
fn draw_ports(
    app: &DesignApp,
    painter: &egui::Painter,
    xf: &Xf,
    doc: &Document,
    layout: &CanvasLayout,
    it: &Item,
    sid: designcraft_doc::StoryId,
    color: Color32,
) {
    let Some(story) = doc.story(sid) else { return };
    let pos = story.frames.iter().position(|f| *f == it.id).unwrap_or(0);
    let is_last = pos + 1 == story.frames.len();
    let cs = app.session.cache.get(doc, sid, None);
    let overset = is_last && cs.is_overset();
    let s = designcraft_tools::thread::PORT_SIZE_PX as f32;
    // In port on the left edge below the top-left corner; out port on the right edge above the
    // bottom-right corner. The Selection tool hit-tests the same places.
    let port = |out: bool| designcraft_tools::thread::port_center(doc, layout, it.id, out, xf.zoom).map(|c| xf.to_screen(c));
    let (Some(inp), Some(outp)) = (port(false), port(true)) else { return };
    for (c, has_link, is_out) in [(inp, pos > 0, false), (outp, !is_last, true)] {
        let pr = Rect::from_center_size(c, vec2(s, s));
        painter.rect_filled(pr, 0.0, Color32::WHITE);
        painter.rect_stroke(pr, 0.0, Stroke::new(hair(painter), color), StrokeKind::Inside);
        if is_out && overset {
            let red = Color32::from_rgb(230, 20, 20);
            painter.line_segment([c - vec2(2.5, 0.0), c + vec2(2.5, 0.0)], Stroke::new(1.5, red));
            painter.line_segment([c - vec2(0.0, 2.5), c + vec2(0.0, 2.5)], Stroke::new(1.5, red));
        } else if has_link {
            painter.add(egui::Shape::convex_polygon(vec![c + vec2(-2.0, -2.5), c + vec2(2.5, 0.0), c + vec2(-2.0, 2.5)], color, Stroke::NONE));
        }
    }
}

fn draw_text_selection(
    app: &DesignApp,
    painter: &egui::Painter,
    xf: &Xf,
    doc: &Document,
    layout: &CanvasLayout,
    ts: designcraft_doc::TextSel,
    cells: Option<designcraft_doc::TableSel>,
) {
    let cs = app.session.cache.get(doc, ts.story, None);
    let range = ts.range();
    let tint = Color32::from_rgba_unmultiplied(80, 140, 255, 110);
    // Selected table cells: tint every selected cell.
    if let Some(sel) = cells.filter(|c| c.story == ts.story) {
        let selected = doc.story(sel.story).and_then(|s| s.tables.get(&sel.table)).map(|t| t.selected_owners(sel.range)).unwrap_or_default();
        for ft in &cs.frames {
            let (Some((a, _)), Some(it)) = (item_canvas_xf(doc, layout, ft.frame), doc.item(ft.frame)) else { continue };
            let m = a * doc.text_xf(it);
            for t in ft.tables.iter().filter(|t| t.table == sel.table) {
                for c in t.cells.iter().filter(|c| selected.binary_search(&(c.row, c.col)).is_ok()) {
                    let q = [
                        xf.to_screen(m * Point::new(c.rect.x0, c.rect.y0)),
                        xf.to_screen(m * Point::new(c.rect.x1, c.rect.y0)),
                        xf.to_screen(m * Point::new(c.rect.x1, c.rect.y1)),
                        xf.to_screen(m * Point::new(c.rect.x0, c.rect.y1)),
                    ];
                    painter.add(egui::Shape::convex_polygon(q.to_vec(), Color32::from_rgba_unmultiplied(80, 140, 255, 90), Stroke::NONE));
                }
            }
        }
        return;
    }
    if let Some(cell) = ts.cell {
        draw_cell_selection(app, painter, xf, doc, layout, &cs, ts, cell);
        return;
    }
    let mut runs: Vec<(usize, Affine, Vec<[Point; 4]>, Vec<(usize, usize)>)> = Vec::new();
    for (fi, ft) in cs.frames.iter().enumerate() {
        let Some((a, _)) = item_canvas_xf(doc, layout, ft.frame) else { continue };
        let Some(it) = doc.item(ft.frame) else { continue };
        let m = a * doc.text_xf(it);
        // Show the frame edge while typing.
        for poly in path_screen(it, xf, a) {
            painter.add(egui::Shape::line(poly, Stroke::new(1.0, layer_color(doc, it).gamma_multiply(0.6))));
        }
        if range.is_empty() {
            continue;
        }
        // Type on a path: the highlight bends with the text.
        if let Some(map) = compose::PathTextMap::new(it, ft) {
            for l in &ft.lines {
                let (s, e) = (range.start.max(l.range.start), range.end.min(l.range.end));
                if s >= e {
                    continue;
                }
                for q in compose::highlight_quads(l, s, e, false) {
                    let (x0, x1) = (q[0].x.min(q[1].x), q[0].x.max(q[1].x));
                    let (y0, y1) = (q[0].y.min(q[2].y), q[0].y.max(q[2].y));
                    // Short slices, each turned with the path.
                    let n = ((x1 - x0) / 2.0).ceil().clamp(1.0, 2000.0) as usize;
                    for k in 0..n {
                        let (a, b) = (x0 + (x1 - x0) * k as f64 / n as f64, x0 + (x1 - x0) * (k + 1) as f64 / n as f64);
                        let poly =
                            [Point::new(a, y0), Point::new(b, y0), Point::new(b, y1), Point::new(a, y1)].map(|p| xf.to_screen(m * map.to_path(p)));
                        painter.add(egui::Shape::convex_polygon(poly.to_vec(), tint, Stroke::NONE));
                    }
                }
            }
            continue;
        }
        let mut quads = Vec::new();
        let mut glyphs = Vec::new();
        for (li, l) in ft.lines.iter().enumerate() {
            let s = range.start.max(l.range.start);
            let e = range.end.min(l.range.end);
            if s > e || (s == e && !(l.range.end < range.end && e == l.range.end)) {
                continue;
            }
            let past = e == l.range.end && range.end > l.range.end;
            quads.extend(compose::highlight_quads(l, s, e, past));
            // A selected drop cap is highlighted down to its baseline.
            if let Some(dc) = l.drop_cap {
                let sel = l.glyphs.iter().filter(|g| g.len > 0 && g.byte < dc.end && g.byte >= s && g.byte < e);
                let (a, b) = sel.fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), g| (a.min(g.x), b.max(g.x + g.adv)));
                if a < b {
                    let r = dc.rect;
                    quads.push([Point::new(a, r.y0), Point::new(b, r.y0), Point::new(b, r.y1), Point::new(a, r.y1)]);
                }
            }
            for (gi, g) in l.glyphs.iter().enumerate() {
                if g.visible && g.len > 0 && g.byte >= s && g.byte < e {
                    glyphs.push((li, gi));
                }
            }
        }
        if !quads.is_empty() {
            runs.push((fi, m, quads, glyphs));
        }
    }
    if !runs.is_empty() {
        draw_inverse_highlight(app, painter, xf, doc, &cs, ts.story, &runs);
    }
    if ts.is_caret()
        && let Some((fi, x, bl, asc, desc)) = compose::caret(&cs, ts.focus)
        && let Some(ft) = cs.frames.get(fi)
        && let (Some((a, _)), Some(it)) = (item_canvas_xf(doc, layout, ft.frame), doc.item(ft.frame))
    {
        let m = a * doc.text_xf(it);
        // Type on a path: the caret stands across the path where it is along it.
        let on = |p: Point| compose::PathTextMap::new(it, ft).map_or(p, |map| map.to_path(p));
        let p0 = xf.to_screen(m * on(Point::new(x, bl - asc)));
        let p1 = xf.to_screen(m * on(Point::new(x, bl + desc)));
        let blink = (painter.ctx().input(|i| i.time) * 1.6) as i64 % 2 == 0;
        if blink {
            painter.line_segment([p0, p1], Stroke::new(1.0, Color32::BLACK));
        }
        painter.ctx().request_repaint_after(std::time::Duration::from_millis(330));
    }
}

/// A text field had keyboard focus when this frame began, per window: egui keeps one focus per
/// viewport, so a field focused in the main window must not mute the second window's canvas.
fn field_focused_id(ctx: &egui::Context) -> egui::Id {
    egui::Id::unique("canvas_field_focused").with(ctx.viewport_id())
}

/// Called before the panels draw. A panel field gives up focus on Enter or Tab while it draws,
/// before the canvas: without this the canvas would take that same key for the text.
pub fn begin_frame(ctx: &egui::Context) {
    let (id, focused) = (field_focused_id(ctx), ctx.text_edit_focused());
    ctx.data_mut(|d| d.insert_temp(id, focused));
    #[cfg(not(target_arch = "wasm32"))]
    {
        let now = ctx.input(|i| i.modifiers);
        // The modifiers last frame ended with are the ones this frame's events began with.
        ctx.data_mut(|d| {
            let (_, end) = d.get_temp::<(egui::Modifiers, egui::Modifiers)>(modifiers_id()).unwrap_or((now, now));
            d.insert_temp(modifiers_id(), (end, now));
        });
    }
}

/// (held when this frame's events began, held after them).
#[cfg(not(target_arch = "wasm32"))]
fn modifiers_id() -> egui::Id {
    egui::Id::unique("canvas-frame-modifiers")
}

/// Caret / text highlight inside a table cell (the cell's lines are offset by the cell origin).
#[allow(clippy::too_many_arguments)]
fn draw_cell_selection(
    _app: &DesignApp,
    painter: &egui::Painter,
    xf: &Xf,
    doc: &Document,
    layout: &CanvasLayout,
    cs: &compose::ComposedStory,
    ts: designcraft_doc::TextSel,
    cell: designcraft_doc::CellAddr,
) {
    // Footnote text or a cell: its composed story and where it sits in the frame.
    let found = match cell.footnote_id() {
        Some(id) => compose::find_note(cs, id).map(|(fi, n)| (fi, n.origin, n.text.clone())),
        None => compose::find_cell(cs, cell.table, cell.row, cell.col).map(|(fi, _, pc)| (fi, pc.origin, pc.text.clone())),
    };
    let Some((fi, origin, text)) = found else { return };
    let Some(ft) = cs.frames.get(fi) else { return };
    let (Some((a, _)), Some(it)) = (item_canvas_xf(doc, layout, ft.frame), doc.item(ft.frame)) else { return };
    let m = a * doc.text_xf(it) * designcraft_geom::Affine::translate(origin.to_vec2());
    let range = ts.range();
    if !range.is_empty()
        && let Some(cft) = text.frames.first()
    {
        for l in &cft.lines {
            let s = range.start.max(l.range.start);
            let e = range.end.min(l.range.end);
            if s > e || (s == e && !(l.range.end < range.end && e == l.range.end)) {
                continue;
            }
            let past = e == l.range.end && range.end > l.range.end;
            for q in compose::highlight_quads(l, s, e, past) {
                let poly = q.map(|p| xf.to_screen(m * p));
                painter.add(egui::Shape::convex_polygon(poly.to_vec(), Color32::from_rgba_unmultiplied(80, 140, 255, 110), Stroke::NONE));
            }
        }
    }
    if ts.is_caret()
        && let Some((_, x, bl, asc, desc)) = match cell.footnote_id() {
            Some(id) => compose::note_caret(cs, id, ts.focus),
            None => compose::cell_caret(cs, cell.table, cell.row, cell.col, ts.focus),
        }
    {
        let m = a * doc.text_xf(it);
        let (p0, p1) = (xf.to_screen(m * Point::new(x, bl - asc)), xf.to_screen(m * Point::new(x, bl + desc)));
        let blink = (painter.ctx().input(|i| i.time) * 1.6) as i64 % 2 == 0;
        if blink {
            painter.line_segment([p0, p1], Stroke::new(1.0, Color32::BLACK));
        }
        painter.ctx().request_repaint_after(std::time::Duration::from_millis(330));
    }
}

/// InDesign draws selected text inverted: a black highlight with the selected glyphs in the
/// inverse of their colour (white for black text). The highlight and glyph outlines are
/// rasterised with `vello_cpu` into a small screen-space texture over the selection.
fn draw_inverse_highlight(
    app: &DesignApp,
    painter: &egui::Painter,
    xf: &Xf,
    doc: &Document,
    cs: &compose::ComposedStory,
    story: designcraft_doc::StoryId,
    runs: &[(usize, Affine, Vec<[Point; 4]>, Vec<(usize, usize)>)],
) {
    use designcraft_render::vello_cpu::{self, kurbo, peniko};
    let ctx = painter.ctx();
    let ppp = ctx.pixels_per_point();
    let to_screen = |m: Affine, p: Point| xf.to_screen(m * p);
    let mut bbox = Rect::NOTHING;
    for (_, m, quads, _) in runs {
        for q in quads {
            for p in q {
                bbox.extend_with(to_screen(*m, *p));
            }
        }
    }
    let bbox = bbox.intersect(painter.clip_rect());
    if !bbox.is_positive() {
        return;
    }
    // Snap to device pixels so the texture maps 1:1.
    let bbox = Rect::from_min_max(
        pos2((bbox.min.x * ppp).floor() / ppp, (bbox.min.y * ppp).floor() / ppp),
        pos2((bbox.max.x * ppp).ceil() / ppp, (bbox.max.y * ppp).ceil() / ppp),
    );
    let (w, h) = (((bbox.width() * ppp).round() as u32).clamp(1, 8192), ((bbox.height() * ppp).round() as u32).clamp(1, 8192));
    let Some(st) = app.session.active() else { return };
    let key = egui::Id::unique(("inverse_sel", st.uid, st.revision, std::sync::Arc::as_ptr(&st.doc) as usize, story.0))
        .with((runs.iter().map(|r| r.2.len() + r.3.len() * 7919).sum::<usize>(), w, h))
        .with((bbox.min.x.to_bits(), bbox.min.y.to_bits(), xf.zoom.to_bits(), st.selection.text.map(|t| (t.anchor, t.focus))));
    let cache_id = egui::Id::unique("inverse_sel_tex");
    let cached: Option<(egui::Id, egui::TextureHandle)> = ctx.data(|d| d.get_temp(cache_id));
    let tex = match cached {
        Some((k, t)) if k == key => t,
        _ => {
            let view = xf.affine(ppp as f64, bbox.min);
            let mut rc = vello_cpu::RenderContext::new_with(w as u16, h as u16, vello_cpu::RenderSettings { num_threads: 0, ..Default::default() });
            rc.set_paint(peniko::Color::from_rgba8(0, 0, 0, 255));
            for (_, m, quads, _) in runs {
                rc.set_transform(view * *m);
                for q in quads {
                    let mut bp = kurbo::BezPath::new();
                    bp.move_to(q[0]);
                    for p in &q[1..] {
                        bp.line_to(*p);
                    }
                    bp.close_path();
                    rc.fill_path(&bp);
                }
            }
            let db = designcraft_fonts::FontDb::global();
            for (fi, m, _, glyphs) in runs {
                let Some(ft) = cs.frames.get(*fi) else { continue };
                for &(li, gi) in glyphs {
                    let Some(l) = ft.lines.get(li) else { continue };
                    let Some(g) = l.glyphs.get(gi) else { continue };
                    let Some(style) = cs.styles.get(g.style as usize) else { continue };
                    let Some(c) = doc.resolve_color(&style.fill, style.fill_tint) else { continue };
                    let [r, gg, b, _] = c.to_rgba8(1.0);
                    rc.set_paint(peniko::Color::from_rgba8(255 - r, 255 - gg, 255 - b, 255));
                    let outline = db.outline(&g.face, g.gid);
                    let skew =
                        if style.skew != 0.0 { Affine::new([1.0, 0.0, -style.skew.to_radians().tan(), 1.0, 0.0, 0.0]) } else { Affine::IDENTITY };
                    let shatai = style.shatai_xf(g, l.baseline).unwrap_or(Affine::IDENTITY);
                    let ga = shatai * Affine::translate((g.x, l.baseline + g.y)) * skew * Affine::scale_non_uniform(g.sx, g.sy);
                    rc.set_transform(view * *m * ga);
                    rc.fill_path(&outline);
                }
            }
            rc.flush();
            let mut pm = vello_cpu::Pixmap::new(w as u16, h as u16);
            let mut res = vello_cpu::Resources::new();
            rc.render(&mut pm, &mut res);
            let ci = egui::ColorImage::from_rgba_premultiplied([w as usize, h as usize], pm.data_as_u8_slice());
            let t = crate::widgets::load_texture(ctx, "inverse_selection", ci, egui::TextureOptions::NEAREST);
            ctx.data_mut(|d| d.insert_temp(cache_id, (key, t.clone())));
            t
        }
    };
    painter.image(tex.id(), bbox, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
}

fn draw_tool_overlays(app: &mut DesignApp, painter: &egui::Painter, xf: &Xf) {
    if let (Some((z, c)), Some(rect)) = (app.power_zoom, app.canvas_rect) {
        // The view that releasing returns to.
        let (w, h) = (rect.width() as f64 / z, rect.height() as f64 / z);
        let r = xf.rect(designcraft_geom::Rect::new(c.x - w / 2.0, c.y - h / 2.0, c.x + w / 2.0, c.y + h / 2.0));
        painter.rect_stroke(r, 0.0, egui::Stroke::new(1.5, Color32::from_rgb(230, 40, 40)), egui::StrokeKind::Middle);
    }
    let tok = Tokens::get(painter.ctx());
    let ov = app.session.overlays(app.view_info());
    let doc = app.session.active().map(|st| st.doc.clone());
    let layout = app.session.layout();
    let sel_color = selection_color(app);
    for o in ov {
        match o {
            Overlay::Marquee(r) => {
                let r = xf.rect(r);
                painter.rect_filled(r, 0.0, Color32::from_rgba_unmultiplied(80, 140, 255, 30));
                for (a, b) in [
                    (r.left_top(), r.right_top()),
                    (r.right_top(), r.right_bottom()),
                    (r.right_bottom(), r.left_bottom()),
                    (r.left_bottom(), r.left_top()),
                ] {
                    dashed(painter, a, b, Stroke::new(1.0, Color32::from_gray(200)), 3.0, 3.0);
                }
            }
            Overlay::Measure { p, text } => {
                let s = xf.to_screen(p) + vec2(14.0, 14.0);
                let g = painter.layout_no_wrap(text, egui::FontId::proportional(11.0), tok.measure_text);
                let r = Rect::from_min_size(s, g.size() + vec2(10.0, 6.0));
                painter.rect_filled(r, 3.0, tok.measure_bg);
                painter.galley(r.min + vec2(5.0, 3.0), g, tok.measure_text);
            }
            Overlay::Line { a, b, color, dashed: d } => {
                let st = Stroke::new(1.0, c32(color));
                if d {
                    dashed(painter, xf.to_screen(a), xf.to_screen(b), st, 3.0, 3.0);
                } else {
                    painter.line_segment([xf.to_screen(a), xf.to_screen(b)], st);
                }
            }
            Overlay::Guide { a, b } => {
                painter.line_segment([xf.to_screen(a), xf.to_screen(b)], Stroke::new(1.0, tok.smart_guide));
            }
            Overlay::Gap { a, b, label } => {
                let (a, b) = (xf.to_screen(a), xf.to_screen(b));
                painter.line_segment([a, b], Stroke::new(1.0, tok.smart_guide));
                let s = a + (b - a) * 0.5;
                let g = painter.layout_no_wrap(label, egui::FontId::proportional(11.0), tok.measure_text);
                let r = Rect::from_min_size(s, g.size() + vec2(10.0, 6.0));
                painter.rect_filled(r, 3.0, tok.measure_bg);
                painter.galley(r.min + vec2(5.0, 3.0), g, tok.measure_text);
            }
            Overlay::Path { path, color, dashed: d } => draw_path_overlay(painter, xf, &path, c32(color), d),
            Overlay::ContentGhost(id) => {
                if let Some(doc) = &doc {
                    draw_content_ghost(painter, xf, doc, &layout, id);
                }
            }
            Overlay::LiveCornerWidget(p) => {
                let r = Rect::from_center_size(xf.to_screen(p), vec2(6.0, 6.0));
                painter.rect_filled(r, 0.0, tok.live_corner);
                painter.rect_stroke(r, 0.0, Stroke::new(1.0, sel_color), StrokeKind::Inside);
            }
            Overlay::LiveCornerDiamond(p) => {
                let c = xf.to_screen(p);
                let h = 4.5;
                let pts = vec![c + vec2(0.0, -h), c + vec2(h, 0.0), c + vec2(0.0, h), c + vec2(-h, 0.0)];
                painter.add(egui::Shape::convex_polygon(pts, tok.live_corner, Stroke::new(1.0, sel_color)));
            }
            Overlay::PageHandles(r) => {
                // The Page tool's selected page: outline and eight handles, as InDesign draws them.
                let r = xf.rect(r);
                let col = Color32::from_rgb(79, 153, 255);
                painter.rect_stroke(r, 0.0, Stroke::new(1.0, col), StrokeKind::Middle);
                for i in (0..9).filter(|i| *i != 4) {
                    let c = pos2(r.min.x + r.width() * (i % 3) as f32 / 2.0, r.min.y + r.height() * (i / 3) as f32 / 2.0);
                    let h = Rect::from_center_size(c, vec2(7.0, 7.0));
                    painter.rect_filled(h, 0.0, Color32::WHITE);
                    painter.rect_stroke(h, 0.0, Stroke::new(1.0, col), StrokeKind::Inside);
                }
            }
            Overlay::Tag { p, text } => {
                let text = crate::i18n::tr(&app.ui.language, &text).to_owned();
                let s = xf.to_screen(p);
                let g = painter.layout_no_wrap(text, egui::FontId::proportional(10.0), tok.measure_text);
                let r = Rect::from_min_size(s - vec2(0.0, g.size().y + 4.0), g.size() + vec2(8.0, 4.0));
                painter.rect_filled(r, 2.0, tok.measure_bg);
                painter.rect_stroke(r, 2.0, Stroke::new(1.0, Color32::from_rgb(79, 153, 255)), StrokeKind::Inside);
                painter.galley(r.min + vec2(4.0, 2.0), g, tok.measure_text);
            }
            Overlay::Tip { p, text } => {
                let text = crate::i18n::tr(&app.ui.language, &text).to_owned();
                let s = xf.to_screen(p) + vec2(14.0, 14.0);
                let g = painter.layout(text, egui::FontId::proportional(11.0), tok.measure_text, 260.0);
                let r = Rect::from_min_size(s, g.size() + vec2(10.0, 6.0));
                painter.rect_filled(r, 3.0, tok.measure_bg);
                painter.galley(r.min + vec2(5.0, 3.0), g, tok.measure_text);
            }
            Overlay::LiquidPins { object, page, pins, resize } => draw_liquid_pins(painter, xf.rect(object), xf.rect(page), pins, resize),
        }
    }
}

/// A path overlay (canvas coordinates; the Pencil's stroke while drawing, the Smooth and Erase
/// drags) as screen polylines, its curves flattened to about a quarter of a screen pixel.
fn draw_path_overlay(painter: &egui::Painter, xf: &Xf, path: &designcraft_geom::BezPath, color: Color32, dashed_line: bool) {
    use designcraft_geom::PathEl;
    let stroke = Stroke::new(1.0, color);
    let mut runs: Vec<Vec<Pos2>> = Vec::new();
    designcraft_geom::kurbo::flatten(path, 0.25 / xf.zoom.max(1e-9), |el| match el {
        PathEl::MoveTo(p) => runs.push(vec![xf.to_screen(p)]),
        PathEl::LineTo(p) => {
            if let Some(run) = runs.last_mut() {
                run.push(xf.to_screen(p));
            }
        }
        PathEl::ClosePath => {
            if let Some(run) = runs.last_mut()
                && let Some(&first) = run.first()
            {
                run.push(first);
            }
        }
        // Flattening yields only moves, lines and closes.
        PathEl::QuadTo(..) | PathEl::CurveTo(..) => {}
    });
    for run in runs.into_iter().filter(|r| r.len() >= 2) {
        if dashed_line {
            painter.extend(egui::Shape::dashed_line(&run, stroke, 3.0, 3.0));
        } else {
            painter.add(egui::Shape::line(run, stroke));
        }
    }
}

/// A frame's whole content drawn faintly over the canvas, the part outside the frame included,
/// with its bounds outlined (while the content moves inside the frame).
fn draw_content_ghost(painter: &egui::Painter, xf: &Xf, doc: &Document, layout: &CanvasLayout, id: designcraft_doc::ItemId) {
    let Some(it) = doc.item(id) else { return };
    let Some((a, _)) = item_canvas_xf(doc, layout, id) else { return };
    let outline = Stroke::new(1.0, Color32::from_rgb(196, 111, 43));
    let Content::Graphic(g) = &it.content else {
        for c in it.children() {
            let r = xf.rect((a * it.xf).transform_rect_bbox(c.bounds()));
            painter.rect_stroke(r, 0.0, outline, StrokeKind::Middle);
        }
        return;
    };
    let m = a * it.xf * g.xf;
    let (w, h) = g.size;
    let corners = [Point::new(0.0, 0.0), Point::new(w, 0.0), Point::new(w, h), Point::new(0.0, h)].map(|p| xf.to_screen(m * p));
    if corners.iter().any(|p| !p.x.is_finite() || !p.y.is_finite()) {
        return;
    }
    if let Some(tex) = ghost_texture(painter.ctx(), doc, g) {
        let mut mesh = egui::Mesh::with_texture(tex.id());
        let tint = Color32::from_white_alpha(90);
        for (p, uv) in corners.iter().zip([pos2(0.0, 0.0), pos2(1.0, 0.0), pos2(1.0, 1.0), pos2(0.0, 1.0)]) {
            mesh.vertices.push(egui::epaint::Vertex { pos: *p, uv, color: tint });
        }
        mesh.indices.extend([0, 1, 2, 0, 2, 3]);
        painter.add(egui::Shape::mesh(mesh));
    }
    painter.add(egui::Shape::closed_line(corners.to_vec(), outline));
}

/// A screen-resolution proxy of a placed graphic for the ghost, cached per asset.
fn ghost_texture(ctx: &egui::Context, doc: &Document, g: &designcraft_doc::Graphic) -> Option<egui::TextureHandle> {
    let asset = doc.assets.get(&g.asset)?;
    let key = egui::Id::unique(("content_ghost", std::sync::Arc::as_ptr(&asset.data) as usize, asset.data.len(), asset.page));
    if let Some(t) = ctx.data(|d| d.get_temp::<egui::TextureHandle>(key)) {
        return Some(t);
    }
    // A 72 ppi proxy is enough for a faint preview.
    let pm = designcraft_render::images::mip(&asset.data, asset.page, g.size.0, 1.0)?;
    let (pw, ph) = (pm.width() as usize, pm.height() as usize);
    if pw == 0 || ph == 0 {
        return None;
    }
    let ci = egui::ColorImage::from_rgba_premultiplied([pw, ph], pm.data_as_u8_slice());
    let max = ctx.input(|i| i.max_texture_side).min(4096);
    let t = ctx.load_texture("content_ghost", crate::widgets::fit_texture(ci, max), egui::TextureOptions::LINEAR);
    ctx.data_mut(|d| d.insert_temp(key, t.clone()));
    Some(t)
}

/// An object's liquid controls on an object-based page, as InDesign draws them in orange: a
/// dashed line across the object for width and height with a padlock (closed: locked), and a pin
/// outside each edge (pinned: a line to the page edge).
fn draw_liquid_pins(painter: &egui::Painter, o: Rect, page: Rect, pins: [bool; 4], resize: [bool; 2]) {
    let col = Color32::from_rgb(178, 88, 18);
    let st = Stroke::new(1.5, col);
    let c = o.center();
    let mids = [pos2(c.x, o.min.y), pos2(o.max.x, c.y), pos2(c.x, o.max.y), pos2(o.min.x, c.y)];
    // The width and height lines between the edges' middles.
    for (a, b) in [(mids[3], mids[1]), (mids[0], mids[2])] {
        dashed(painter, a, b, Stroke::new(1.0, col), 3.0, 3.0);
    }
    for m in mids {
        painter.circle_filled(m, 3.5, col);
    }
    let (po, lo) = (designcraft_tools::PIN_OFFSET_PX as f32, designcraft_tools::LOCK_OFFSET_PX as f32);
    let outward = [vec2(0.0, -1.0), vec2(1.0, 0.0), vec2(0.0, 1.0), vec2(-1.0, 0.0)];
    let edge = [pos2(c.x, page.min.y), pos2(page.max.x, c.y), pos2(c.x, page.max.y), pos2(page.min.x, c.y)];
    for i in 0..4 {
        if pins[i] {
            painter.line_segment([mids[i], edge[i]], st);
            painter.circle_filled(edge[i], 4.0, col);
        } else {
            painter.circle_stroke(mids[i] + outward[i] * po, 4.0, st);
        }
    }
    // Padlocks: the width lock right of the centre, the height lock above it.
    for (at, locked) in [(c + vec2(lo, 0.0), !resize[0]), (c + vec2(0.0, -lo), !resize[1])] {
        let body = Rect::from_center_size(at + vec2(0.0, 2.0), vec2(8.0, 6.0));
        painter.rect_filled(body.expand(1.0), 1.0, Color32::WHITE);
        painter.rect_filled(body, 1.0, col);
        // The shackle: over the body when locked, swung open when not.
        let base = if locked { body.center().x } else { body.center().x + 3.0 };
        let pts: Vec<_> = (0..=8)
            .map(|k| {
                let a = std::f32::consts::PI * k as f32 / 8.0;
                pos2(base - 2.6 * a.cos(), body.min.y - 3.0 * a.sin())
            })
            .collect();
        painter.add(egui::Shape::line(pts, Stroke::new(1.3, col)));
    }
}

/// Paint ruler bands and ticks, the zero-point crosshair, guide previews, and ticks restarted at each page origin.
#[allow(clippy::too_many_arguments)]
fn draw_rulers(app: &DesignApp, ui: &egui::Ui, full: Rect, rect: Rect, xf: &Xf, doc: &Document, layout: &CanvasLayout, t: &Tokens) {
    let painter = ui.painter_at(full);
    let (top, left) = ruler_rects(full, rect);
    painter.rect_filled(top, 0.0, t.ruler);
    painter.rect_filled(left, 0.0, t.ruler);
    painter.rect_filled(Rect::from_min_max(full.min, rect.min), 0.0, t.ruler);
    painter.line_segment([pos2(rect.min.x, rect.min.y - 0.25), pos2(full.max.x, rect.min.y - 0.25)], Stroke::new(0.5, t.ruler_tick));
    painter.line_segment([pos2(rect.min.x - 0.25, rect.min.y), pos2(rect.min.x - 0.25, full.max.y)], Stroke::new(0.5, t.ruler_tick));
    // Zero-point crosshair box.
    let corner = Rect::from_min_max(full.min, rect.min);
    let zc = corner.center();
    painter.line_segment([zc - vec2(4.0, 0.0), zc + vec2(4.0, 0.0)], Stroke::new(1.0, t.ruler_tick));
    painter.line_segment([zc - vec2(0.0, 4.0), zc + vec2(0.0, 4.0)], Stroke::new(1.0, t.ruler_tick));
    // While the zero point is dragged out of the corner, the rulers measure from where it would
    // land (kept on the pasteboard, as the engine keeps it) and crosshair lines show the spot.
    // Cmd-dragged, the lines are the two guides it makes; locked, a plain drag does nothing.
    let dragged = ui.data(|d| d.get_temp::<Pos2>(egui::Id::unique((ZERO_POINT_DRAG, app.pane)))).filter(|p| !corner.contains(*p));
    let guides = ui.input(|i| i.modifiers.command);
    let moving = dragged.filter(|_| !guides && !doc.settings.zero_point_locked).and_then(|p| zero_point_at(app, xf, p));
    let cross = if guides { dragged } else { moving.map(|(_, c)| xf.to_screen(c)) };
    if let Some(p) = cross {
        let m = Stroke::new(1.0, t.text_dim);
        dashed(&painter, pos2(p.x, rect.min.y), pos2(p.x, full.max.y), m, 3.0, 3.0);
        dashed(&painter, pos2(rect.min.x, p.y), pos2(full.max.x, p.y), m, 3.0, 3.0);
    }
    let zp = moving.map_or_else(|| doc.zero_point(), |(zp, _)| zp);
    let pieces = ruler_pieces(app, doc, layout, zp);
    let Some(&(_, _, origin)) = pieces.first() else { return };
    let font = egui::FontId::proportional(9.0);
    let tick = Stroke::new(1.0, t.ruler_tick);
    // Horizontal: one run of ticks per piece (a page origin restarts at every page).
    let unit = doc.settings.horizontal_units;
    let (major, sub) = unit.ruler_ticks(xf.zoom);
    let (v0, v1) = (xf.to_canvas(top.min).x, xf.to_canvas(pos2(top.max.x, 0.0)).x);
    for &(from, to, o) in &pieces {
        // The piece and the visible ruler, measured from the piece's origin.
        let (s0, s1) = (from - o.x, to - o.x);
        let (c0, c1) = ((v0 - o.x).max(s0), (v1 - o.x).min(s1));
        if !(c0.is_finite() && c1.is_finite() && c0 <= c1) {
            continue;
        }
        let mut k = (c0 / major).floor() as i64;
        while (k as f64) * major <= c1 {
            let x0 = k as f64 * major;
            for s in 0..sub {
                let x = x0 + major * s as f64 / sub as f64;
                if x < s0 || x >= s1 {
                    continue;
                }
                let sx = xf.to_screen(Point::new(o.x + x, 0.0)).x;
                let len = if s == 0 {
                    RULER
                } else if sub % 2 == 0 && s == sub / 2 {
                    7.0
                } else {
                    4.0
                };
                painter.line_segment([pos2(sx, top.max.y - len), pos2(sx, top.max.y)], tick);
                if s == 0 {
                    let v = unit.from_pt(x);
                    painter.text(pos2(sx + 2.0, top.min.y + 1.0), egui::Align2::LEFT_TOP, fmt_tick(v), font.clone(), t.ruler_text);
                }
            }
            k += 1;
        }
    }
    // Vertical.
    let unit = doc.settings.vertical_units;
    let (major, sub) = unit.ruler_ticks(xf.zoom);
    let c0 = xf.to_canvas(pos2(0.0, left.min.y)).y - origin.y;
    let c1 = xf.to_canvas(pos2(0.0, left.max.y)).y - origin.y;
    let mut k = (c0 / major).floor() as i64;
    while (k as f64) * major <= c1 {
        let y0 = k as f64 * major;
        for s in 0..sub {
            let y = y0 + major * s as f64 / sub as f64;
            let sy = xf.to_screen(Point::new(0.0, origin.y + y)).y;
            let len = if s == 0 {
                RULER
            } else if sub % 2 == 0 && s == sub / 2 {
                7.0
            } else {
                4.0
            };
            painter.line_segment([pos2(left.max.x - len, sy), pos2(left.max.x, sy)], tick);
            if s == 0 {
                // Stacked digits like InDesign's vertical ruler.
                let label = fmt_tick(unit.from_pt(y));
                for (j, ch) in label.chars().enumerate() {
                    painter.text(pos2(left.min.x + 3.0, sy + 2.0 + j as f32 * 8.5), egui::Align2::LEFT_TOP, ch, font.clone(), t.ruler_text);
                }
            }
        }
        k += 1;
    }
    // Pointer position markers.
    if let Some(p) = ui.ctx().pointer_hover_pos().filter(|p| rect.contains(*p) && !pointer_elsewhere(app, ui.ctx())) {
        let m = Stroke::new(1.0, t.text_dim);
        dashed(&painter, pos2(p.x, top.min.y), pos2(p.x, top.max.y), m, 1.5, 1.5);
        dashed(&painter, pos2(left.min.x, p.y), pos2(left.max.x, p.y), m, 1.5, 1.5);
    }
}

/// The horizontal ruler of the spread in view in pieces `(from x, to x, origin)`, canvas
/// coordinates: the ruler origin (one per page for a page origin) moved by zero point `zp`.
fn ruler_pieces(app: &DesignApp, doc: &Document, layout: &CanvasLayout, zp: [f64; 2]) -> Vec<(f64, f64, Point)> {
    let Some(slot) = current_slot(app, layout).and_then(|i| layout.slots.get(i)) else { return Vec::new() };
    ruler_pieces_of(doc, slot, zp)
}

/// [`ruler_pieces`] for the spread in `slot`.
fn ruler_pieces_of(doc: &Document, slot: &SpreadSlot, zp: [f64; 2]) -> Vec<(f64, f64, Point)> {
    let x = |v: f64, y: f64| if v.is_finite() { slot.to_canvas(Point::new(v, y)).x } else { v };
    doc.ruler_pieces(slot.spread)
        .into_iter()
        .map(|(from, to, b)| (x(from, b.y), x(to, b.y), slot.to_canvas(Point::new(b.x + zp[0], b.y + zp[1]))))
        .collect()
}

/// The horizontal (top) and vertical (left) rulers around the view `rect` in `full`.
fn ruler_rects(full: Rect, rect: Rect) -> (Rect, Rect) {
    (
        Rect::from_min_max(pos2(rect.min.x, full.min.y), pos2(full.max.x, rect.min.y)),
        Rect::from_min_max(pos2(full.min.x, rect.min.y), pos2(rect.min.x, full.max.y)),
    )
}

/// Right-clicking a ruler opens its menu, as InDesign's: units for that ruler, Hide Rulers and
/// (the horizontal ruler) Delete All Guides on Spread. Right-clicking the corner where they meet
/// opens Lock Zero Point and the units for both rulers at once.
fn ruler_units_menus(app: &mut DesignApp, resp: &egui::Response, full: Rect, rect: Rect, doc: &Document) {
    let (top, left) = ruler_rects(full, rect);
    let corner = Rect::from_min_max(full.min, rect.min);
    let at = resp.interact_pointer_pos();
    let (h, v) = (doc.settings.horizontal_units, doc.settings.vertical_units);
    let locked = doc.settings.zero_point_locked;
    // (where, which units it sets, the unit checked)
    let menus = [(top, RulerMenu::Horizontal, Some(h)), (left, RulerMenu::Vertical, Some(v)), (corner, RulerMenu::Both, (h == v).then_some(h))];
    for (area, which, cur) in menus {
        let open = if resp.secondary_clicked() {
            Some(egui::SetOpenCommand::Bool(at.is_some_and(|p| area.contains(p))))
        } else if resp.clicked() {
            Some(egui::SetOpenCommand::Bool(false))
        } else {
            None
        };
        egui::Popup::context_menu(resp)
            .id(resp.id.with(("ruler_units", which as u8)))
            .open_memory(open)
            .show(|ui| ruler_units_menu(app, ui, which, cur, locked));
    }
}

#[derive(Clone, Copy, PartialEq)]
enum RulerMenu {
    Horizontal,
    Vertical,
    Both,
}

/// The units a ruler's menu offers, in InDesign's order.
const RULER_UNITS: [designcraft_geom::Unit; 9] = {
    use designcraft_geom::Unit::*;
    [Points, Picas, Inches, InchesDecimal, Millimeters, Centimeters, Ciceros, Agates, Pixels]
};

/// A ruler's menu: the units (the current one checked), then Hide Rulers and, on the horizontal
/// ruler, Delete All Guides on Spread. The corner's menu (`Both`) starts with Lock Zero Point
/// (`locked` = the zero point is locked). Picking a unit sets `horizontalUnits`,
/// `verticalUnits` or both through `document.preferences`.
fn ruler_units_menu(app: &mut DesignApp, ui: &mut egui::Ui, which: RulerMenu, cur: Option<designcraft_geom::Unit>, locked: bool) {
    let lang = app.ui.language.clone();
    let tr = |s: &str| crate::i18n::tr(&lang, s).to_string();
    if which == RulerMenu::Both {
        let label = tr(if locked { "Unlock Zero Point" } else { "Lock Zero Point" });
        if ui.button(crate::rtl::widget(ui, format!("   {label}"))).clicked() {
            let _ = app.run("view.lockZeroPoint", json!({"on": !locked}));
            ui.close();
        }
        ui.separator();
    }
    // A document in Q or Ha keeps its unit on the list.
    let extra = cur.filter(|u| !RULER_UNITS.contains(u));
    for u in RULER_UNITS.into_iter().chain(extra) {
        let label = tr(u.label());
        let text = if Some(u) == cur { format!("✓ {label}") } else { format!("   {label}") };
        if ui.button(crate::rtl::widget(ui, text)).clicked() {
            let p = match which {
                RulerMenu::Horizontal => json!({"horizontalUnits": u}),
                RulerMenu::Vertical => json!({"verticalUnits": u}),
                RulerMenu::Both => json!({"horizontalUnits": u, "verticalUnits": u}),
            };
            let _ = app.run("document.preferences", p);
            ui.close();
        }
    }
    ui.separator();
    let hide = format!("   {}", tr("Hide Rulers"));
    if ui
        .add(egui::Button::new(crate::rtl::widget(ui, hide)).shortcut_text(crate::menus::shortcut_of(app, "view.rulers").unwrap_or_default()))
        .clicked()
    {
        let _ = app.run("view.rulers", json!({}));
        ui.close();
    }
    if which == RulerMenu::Horizontal && ui.button(crate::rtl::widget(ui, format!("   {}", tr("Delete All Guides on Spread")))).clicked() {
        crate::menus::activate(app, "app.deleteAllGuides", &Value::Null);
        ui.close();
    }
}

/// A canvas context menu asked for by a right-click; `open` until the popup is open after a frame.
#[derive(Clone, Copy, Debug)]
struct MenuRequest {
    kind: crate::menus::ContextMenu,
    open: bool,
}

/// The popup id of the canvas context menu in split-view pane `pane`.
fn context_menu_id(pane: u8) -> egui::Id {
    egui::Id::unique(("canvas_context_menu", pane))
}

/// Was a canvas context menu open (in either pane of this window) when this frame began?
/// Pane 0 shows its menu, which a click or Escape may close, before pane 1 handles input; that
/// click or key still belongs to the menu. Popups are per window (viewport), and so is input.
pub fn context_menu_open(ctx: &egui::Context) -> bool {
    let pass = ctx.cumulative_pass_nr();
    let key = egui::Id::unique("canvas_context_menu_open").with(ctx.viewport_id());
    if let Some((p, open)) = ctx.data(|d| d.get_temp::<(u64, bool)>(key))
        && p == pass
    {
        return open;
    }
    let open = (0..2).any(|pane| egui::Popup::is_id_open(ctx, context_menu_id(pane)));
    ctx.data_mut(|d| d.insert_temp(key, (pass, open)));
    open
}

/// Shows the context menu a right-click asked for. A right-click while the menu is open closes it
/// in that frame (it was a click outside the menu), so the request stays until a frame ends with
/// the menu open: it reopens in the next frame where the new click was.
fn context_menu(app: &mut DesignApp, ui: &egui::Ui, resp: &egui::Response) {
    let id = context_menu_id(app.pane);
    let Some(req) = ui.data(|d| d.get_temp::<MenuRequest>(id)) else { return };
    egui::Popup::menu(resp)
        .id(id)
        .open_memory(req.open.then_some(egui::SetOpenCommand::Bool(true)))
        .at_pointer_fixed()
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .show(|ui| crate::menus::context_menu(app, ui, req.kind));
    if req.open {
        if egui::Popup::is_id_open(ui.ctx(), id) {
            ui.data_mut(|d| d.insert_temp(id, MenuRequest { open: false, ..req }));
        } else {
            ui.ctx().request_repaint();
        }
    }
}

/// Prepares a right-click at canvas point `p`: picks the menu and the selection it acts on.
/// - Typing: a click in a text frame places the caret there as a left click would, except
///   inside the selected text, which stays selected; elsewhere the text is left as it is.
/// - On an object: it is selected first unless it already is, as the Selection tool would.
/// - On nothing: the selection stays.
fn context_target(app: &mut DesignApp, p: Point, zoom: f64) -> crate::menus::ContextMenu {
    use crate::menus::ContextMenu;
    let hit = item_at(app, p, zoom);
    if app.session.wants_text() {
        let text_frame = hit.filter(|(id, _)| app.session.active().and_then(|st| st.doc.item(*id)).is_some_and(Item::is_text_frame));
        if let Some((id, sp)) = text_frame {
            // A press in the selected text keeps it for dragging; end that press without a move.
            if app.run("text.placeCaret", json!({"frame": id.0, "point": [sp.x, sp.y]})).is_ok_and(|r| r["dragging"] == json!(true)) {
                let _ = app.run("text.release", json!({"frame": id.0, "point": [sp.x, sp.y], "moved": true}));
            }
        }
        return ContextMenu::Text;
    }
    let Some((id, _)) = hit else { return ContextMenu::Canvas };
    let direct = app.session.tool_id() == "directSelection";
    let Some((target, selected)) = app.session.active().map(|st| {
        let target = if direct { id } else { st.doc.top_level_of(id).unwrap_or(id) };
        (target, st.selection.contains(target))
    }) else {
        return ContextMenu::Canvas;
    };
    if !selected {
        let _ = app.run("selection.set", json!({"ids": [target.0], "content": direct}));
    }
    ContextMenu::Object
}

/// The frontmost item under canvas point `p` and that point in its spread, hit as the tools do.
fn item_at(app: &DesignApp, p: Point, zoom: f64) -> Option<(designcraft_doc::ItemId, Point)> {
    let st = app.session.active()?;
    let (sr, sp) = CanvasLayout::new(&st.doc, st.editing_parents).spread_at(p)?;
    let tol = 4.0 / zoom.max(1e-6);
    let id = match sr {
        SpreadRef::Doc(si) => st.doc.hit_item(si, sp, tol)?,
        _ => st.doc.spread(sr)?.items.iter().rev().find(|it| !it.locked && designcraft_doc::edit_hit(it, sp, tol))?.id,
    };
    Some((id, sp))
}

pub(crate) fn fmt_tick(v: f64) -> String {
    let r = v.round();
    if (v - r).abs() < 1e-6 { format!("{}", r as i64) } else { format!("{v:.1}") }
}

/// Move the view's content by a screen-space distance `d` (rotated views move in screen axes).
fn pan(app: &mut DesignApp, xf: &Xf, d: egui::Vec2) {
    let d = xf.unrotate_delta(d);
    if let Some(v) = app.view_mut() {
        v.origin = Point::new(v.origin.x - d.x as f64 / v.zoom, v.origin.y - d.y as f64 / v.zoom);
    }
}

/// A scrollbar thumb on a track `track` pixels long, for `visible` (the view) within `content`
/// (the pasteboard), both spans along the bar's axis in screen pixels: the thumb's start and
/// length on the track, and the content pixels one thumb pixel moves (0 when nothing scrolls).
fn thumb_span(track: f32, content: (f32, f32), visible: (f32, f32)) -> (f32, f32, f32) {
    let track = track.max(0.0);
    let (lo, hi) = (content.0.min(visible.0), content.1.max(visible.1));
    let (total, shown) = (hi - lo, visible.1 - visible.0);
    let scrollable = total - shown;
    if !total.is_finite() || !shown.is_finite() || total <= 0.0 || scrollable <= 0.0 {
        return (0.0, track, 0.0);
    }
    let len = (track * shown / total).max(MIN_THUMB.min(track));
    let free = track - len;
    if free <= 0.0 {
        return (0.0, track, 0.0);
    }
    ((visible.0 - lo) / scrollable * free, len, scrollable / free)
}

/// The track and thumb of the vertical (or horizontal) scrollbar of the view `screen` inside
/// `outer`, and the content pixels per thumb pixel.
fn scrollbar_rects(outer: Rect, screen: Rect, content: Rect, vertical: bool) -> (Rect, Rect, f32) {
    if vertical {
        let track = Rect::from_min_max(pos2(outer.max.x - SCROLLBAR, screen.min.y), pos2(outer.max.x, screen.max.y));
        let (start, len, k) = thumb_span(track.height(), (content.min.y, content.max.y), (screen.min.y, screen.max.y));
        (track, Rect::from_min_size(pos2(track.min.x, track.min.y + start), vec2(SCROLLBAR, len)), k)
    } else {
        let track = Rect::from_min_max(pos2(screen.min.x, outer.max.y - SCROLLBAR), pos2(screen.max.x, outer.max.y));
        let (start, len, k) = thumb_span(track.width(), (content.min.x, content.max.x), (screen.min.x, screen.max.x));
        (track, Rect::from_min_size(pos2(track.min.x + start, track.min.y), vec2(len, SCROLLBAR)), k)
    }
}

/// How far a scrollbar interaction pans the view along its axis: a thumb drag follows the pointer,
/// a click on the track beside the thumb pages by one view.
fn scrollbar_move(resp: &egui::Response, grabbed: bool, vertical: bool, thumb: Rect, page: f32, k: f32) -> f32 {
    if grabbed {
        let d = resp.drag_delta();
        return -(if vertical { d.y } else { d.x }) * k;
    }
    if resp.clicked()
        && let Some(p) = resp.interact_pointer_pos()
    {
        let (p, a, b) = if vertical { (p.y, thumb.min.y, thumb.max.y) } else { (p.x, thumb.min.x, thumb.max.x) };
        if p < a {
            return page;
        } else if p > b {
            return -page;
        }
    }
    0.0
}

/// The view's scrollbars, on the right and bottom edges of `outer`: they span the pasteboard;
/// dragging a thumb pans, clicking the track pages by one view.
fn scrollbars(app: &mut DesignApp, ui: &mut egui::Ui, outer: Rect, screen: Rect, rect: Rect, t: &Tokens) {
    let content = |app: &DesignApp| {
        let st = app.session.active()?;
        let v = app.view()?;
        let xf = Xf::new(rect, v);
        Some((xf, xf.rect(CanvasLayout::new(&st.doc, st.editing_parents).pasteboard)))
    };
    let Some((xf, cb)) = content(app) else { return };
    let press = ui.input(|i| i.pointer.press_origin());
    let mut hot = [false; 2];
    for (n, vertical) in [true, false].into_iter().enumerate() {
        let (track, thumb, k) = scrollbar_rects(outer, screen, cb, vertical);
        let id = egui::Id::unique(("canvas_scrollbar", app.pane, vertical));
        let resp = ui.interact(track, id, Sense::click_and_drag());
        let grab_id = id.with("grab");
        if resp.drag_started() {
            let on_thumb = press.is_some_and(|p| thumb.contains(p));
            ui.data_mut(|d| d.insert_temp(grab_id, on_thumb));
        }
        let grabbed = resp.dragged() && ui.data(|d| d.get_temp::<bool>(grab_id)).unwrap_or(false);
        if !resp.dragged() {
            ui.data_mut(|d| d.remove::<bool>(grab_id));
        }
        let page = if vertical { screen.size().y } else { screen.size().x };
        let moved = scrollbar_move(&resp, grabbed, vertical, thumb, page, k);
        if moved != 0.0 && moved.is_finite() {
            pan(app, &xf, if vertical { vec2(0.0, moved) } else { vec2(moved, 0.0) });
        }
        if let Some(slot) = hot.get_mut(n) {
            *slot = grabbed || resp.hovered();
        }
    }
    // Painted from the view as it is after this frame's input.
    let Some((_, cb)) = content(app) else { return };
    let painter = ui.painter_at(outer);
    let right = Rect::from_min_max(pos2(outer.max.x - SCROLLBAR, outer.min.y), outer.max);
    let bottom = Rect::from_min_max(pos2(outer.min.x, outer.max.y - SCROLLBAR), outer.max);
    painter.rect_filled(right, 0.0, t.panel_darker);
    painter.rect_filled(bottom, 0.0, t.panel_darker);
    for (n, vertical) in [true, false].into_iter().enumerate() {
        let (_, thumb, _) = scrollbar_rects(outer, screen, cb, vertical);
        let alpha = if hot.get(n).copied().unwrap_or(false) { 0.6 } else { 0.35 };
        painter.rect_filled(thumb.shrink(2.0), SCROLLBAR / 2.0 - 2.0, t.text.gamma_multiply(alpha));
    }
}

/// Give an open menu priority, then handle wheel/zoom/pan, spring Zoom, guide drags and primary pointer routing.
fn handle_input(app: &mut DesignApp, ui: &mut egui::Ui, resp: &egui::Response, rect: Rect) {
    // While a context menu is open (in either pane), the click that closes it and the keys go to the menu.
    let menu_open = context_menu_open(ui.ctx());
    let Some(v) = app.view().copied() else { return };
    let xf = Xf::new(rect, &v);
    let wants_text = app.session.wants_text();
    let space = navigation_space(app, ui.ctx());
    // Scroll / zoom.
    let alt = wheel_alt(ui);
    if resp.hovered() {
        let (scroll, zoom_delta, m, hover) = ui.input(|i| (i.smooth_scroll_delta, i.zoom_delta(), i.modifiers, i.pointer.hover_pos()));
        if let Some(hp) = hover {
            if zoom_delta != 1.0 {
                zoom_at(app, hp, zoom_delta as f64);
            } else if m.command && scroll.y != 0.0 {
                zoom_at(app, hp, (1.0 + scroll.y as f64 * 0.003).clamp(0.5, 2.0));
            } else if alt && scroll != egui::Vec2::ZERO {
                // Alt + wheel zooms by as much as Ctrl/Cmd + wheel, which egui turns into `zoom_delta`.
                let speed = ui.ctx().options(|o| o.input_options.scroll_zoom_speed);
                zoom_at(app, hp, (speed * (scroll.x + scroll.y)).exp() as f64);
            } else if scroll != egui::Vec2::ZERO
                && let Some(v) = app.view_mut()
            {
                let scroll = xf.unrotate_delta(scroll);
                v.origin = Point::new(v.origin.x - scroll.x as f64 / v.zoom, v.origin.y - scroll.y as f64 / v.zoom);
            }
        }
    }
    let zoom = spring_zoom_mode(app, ui.ctx(), space);
    if spring_zoom_input(app, ui, resp, &xf, zoom) {
        return;
    }
    // Space-drag = hand (unless a tool drag is under way: then Space is a modifier, e.g. Live
    // Distribute while resizing). Middle-drag pans too, with any tool; tools only
    // see the primary button.
    // A column guide drag keeps the pointer until the button is up, whatever Space or a menu does
    // meanwhile.
    if ui.data(|d| d.get_temp::<ColumnDrag>(egui::Id::unique((COLUMN_DRAG, app.pane)))).is_some() && column_guide_drag(app, ui, resp, rect, &xf) {
        return;
    }
    let tool_drag: bool = ui.data(|d| d.get_temp(egui::Id::unique(("canvas_pointer_down", app.pane)))).unwrap_or(false);
    let hand = space && !ui.input(|i| i.modifiers.command);
    if !tool_drag && ((hand && resp.dragged()) || resp.dragged_by(egui::PointerButton::Middle)) {
        let d = xf.unrotate_delta(resp.drag_delta());
        if let Some(v) = app.view_mut() {
            v.origin = Point::new(v.origin.x - d.x as f64 / v.zoom, v.origin.y - d.y as f64 / v.zoom);
        }
        return;
    }
    // Editing text (Space types a space): Alt held at the press gives the Hand tool until the
    // button is released, and the caret stays. Alt pressed later in a text drag copies the text.
    let alt_hand_id = egui::Id::unique(("canvas_alt_hand", app.pane));
    let mut alt_hand: bool = ui.data(|d| d.get_temp(alt_hand_id)).unwrap_or(false);
    let (press, alt, primary_down) = ui.input(|i| (i.pointer.primary_pressed(), i.modifiers.alt, i.pointer.primary_down()));
    if press && alt && wants_text && !tool_drag && !menu_open && resp.hovered() {
        alt_hand = true;
    }
    if alt_hand {
        if resp.dragged() {
            pan(app, &xf, resp.drag_delta());
        }
        ui.data_mut(|d| d.insert_temp(alt_hand_id, primary_down));
        return;
    }
    // The zero point and ruler guides need the rulers (hidden while the view is rotated).
    if !space && !menu_open && xf.rot == 0 && zero_point_drag(app, ui, resp, rect, &xf) {
        return;
    }
    // Hidden or locked guides can't stay selected.
    if (!app.ui.guides || app.ui.guides_locked) && app.session.active().is_some_and(|st| !st.selection.guides.is_empty()) {
        let _ = app.run("guide.deselect", json!({}));
    }
    if !space && !menu_open && xf.rot == 0 && guide_drag(app, ui, resp, rect, &xf) {
        return;
    }
    if !space && !menu_open && xf.rot == 0 && column_guide_drag(app, ui, resp, rect, &xf) {
        return;
    }
    let m = ui.input(|i| mods(i, space));
    let vi = app.view_info();
    let pos = |p: Pos2| xf.to_canvas(p);
    let mut events: Vec<PointerEvent> = Vec::new();
    let down_id = egui::Id::unique(("canvas_pointer_down", app.pane));
    let mut down: bool = ui.data(|d| d.get_temp(down_id)).unwrap_or(false);
    let clicks_id = egui::Id::unique(("canvas_clicks", app.pane));
    let (pressed, released, origin, latest, now) =
        ui.input(|i| (i.pointer.primary_pressed(), i.pointer.primary_released(), i.pointer.press_origin(), i.pointer.latest_pos(), i.time));
    // A complete click can arrive in one pass: release clears egui's held origin.
    // Recover the actual primary press position, preserving the origin for held gestures.
    let origin = origin.or_else(|| {
        ui.input(|i| {
            i.events
                .iter()
                .rev()
                .find_map(|event| match event {
                    egui::Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed: true, .. } => Some(*pos),
                    _ => None,
                })
                .filter(|pos| pos.is_finite())
        })
    });
    let in_view = |o: Pos2| if xf.rot == 0 { rect.contains(o) } else { resp.rect.contains(o) };
    if pressed
        && !space
        && !menu_open
        && let Some(o) = origin.filter(|o| in_view(*o) && resp.hovered())
    {
        // Count presses in a row here: egui reports double and triple clicks only on release and
        // stops at three, but a press must know (word, line, paragraph, story).
        let (delay, dist) = ui.ctx().options(|o| (o.input_options.max_double_click_delay, o.input_options.max_click_dist));
        let n = press_count(ui.data(|d| d.get_temp(clicks_id)), o, now, delay, dist);
        ui.data_mut(|d| d.insert_temp(clicks_id, Clicks { at: f64::NEG_INFINITY, pos: o, n }));
        events.push(PointerEvent { kind: PointerKind::Down, pos: pos(o), mods: m, clicks: n });
        down = true;
    }
    if down
        && resp.dragged()
        && resp.drag_delta() != egui::Vec2::ZERO
        && let Some(p) = latest
    {
        events.push(PointerEvent { kind: PointerKind::Drag, pos: pos(p), mods: m, clicks: 1 });
    }
    // Power Zoom: holding the Hand tool still for half a second turns the press into one.
    let hold_id = egui::Id::unique(("canvas_hold", app.pane));
    if down && !released && app.session.tool_id() == "hand" && !m.alt {
        let (now, origin_pos) = ui.input(|i| (i.time, i.pointer.press_origin()));
        let (start, fired): (f64, bool) = ui.data(|d| d.get_temp(hold_id)).unwrap_or((now, false));
        let still = origin_pos.zip(latest).is_some_and(|(o, l)| (o - l).length() < 3.0);
        if !fired && still && now - start >= 0.5 {
            let p = pos(latest.unwrap_or(rect.center()));
            events.push(PointerEvent { kind: PointerKind::Up, pos: p, mods: m, clicks: 1 });
            events.push(PointerEvent { kind: PointerKind::Down, pos: p, mods: Mods { alt: true, ..m }, clicks: 1 });
            ui.data_mut(|d| d.insert_temp(hold_id, (start, true)));
        } else {
            ui.data_mut(|d| d.insert_temp(hold_id, (start, fired)));
            if !fired {
                ui.ctx().request_repaint_after(std::time::Duration::from_millis(100));
            }
        }
    } else {
        ui.data_mut(|d| d.remove::<(f64, bool)>(hold_id));
    }
    if down && released {
        if let Some(c) = ui.data(|d| d.get_temp::<Clicks>(clicks_id)) {
            let dist = ui.ctx().options(|o| o.input_options.max_click_dist);
            ui.data_mut(|d| d.insert_temp(clicks_id, release_clicks(c, latest.unwrap_or(c.pos), now, dist)));
        }
        let p = latest.unwrap_or(rect.center());
        events.push(PointerEvent { kind: PointerKind::Up, pos: pos(p), mods: m, clicks: 1 });
        down = false;
    } else if !down && let Some(p) = resp.hover_pos() {
        events.push(PointerEvent { kind: PointerKind::Move, pos: pos(p), mods: m, clicks: 1 });
    }
    ui.data_mut(|d| d.insert_temp(down_id, down));
    for e in events {
        if let Err(err) = app.session.pointer(&e, vi) {
            app.status(err.to_string());
        }
    }
    app.after_engine();
    // Right-click: tools only see the primary button, so it starts no gesture; it opens the menu.
    if resp.secondary_clicked()
        && !down
        && !space
        && app.ui.screen_mode != ScreenMode::Presentation
        && let Some(o) = resp.interact_pointer_pos().filter(|o| in_view(*o))
    {
        let kind = context_target(app, xf.to_canvas(o), xf.zoom);
        ui.data_mut(|d| d.insert_temp(context_menu_id(app.pane), MenuRequest { kind, open: true }));
        ui.ctx().request_repaint();
    }
    if resp.clicked() || resp.drag_started() {
        resp.request_focus();
    }
    // Arrow keys, Tab and Escape belong to the canvas (caret, nudge, tab character, deselect).
    // Without the lock egui moves focus to a neighbouring widget, and the next Space or Return
    // clicks it while typing still reaches the text.
    let lock = egui::EventFilter { tab: true, horizontal_arrows: true, vertical_arrows: true, escape: true };
    ui.memory_mut(|m| m.set_focus_lock_filter(resp.id, lock));
    // Keyboard for the active tool. A field focused when the frame began keeps this frame's keys.
    let field_id = field_focused_id(ui.ctx());
    let field = ui.ctx().text_edit_focused() || ui.data(|d| d.get_temp::<bool>(field_id)).unwrap_or(false);
    if menu_open || modal_open(app) || (field && !resp.has_focus()) || !keyboard_pane(app) {
        return;
    }
    // Keys of a frame that began composing are the IME's too (a Backspace that empties the marked
    // text must not delete the character before it).
    let composing_at_start = app.session.tool_composing();
    let evs = ui.input(|i| i.events.clone());
    // Native only: egui-winit sends a ⌘+V chord as a paste without its key press, so the modifiers
    // held at the paste tell Paste in Place from Paste. The web also gets the key press, and the
    // shortcut table has already run the command. A chord's modifiers may be released in the same
    // frame, hence the modifiers held at each event.
    #[cfg(not(target_arch = "wasm32"))]
    let mut held =
        ui.data(|d| d.get_temp::<(egui::Modifiers, egui::Modifiers)>(modifiers_id())).map_or_else(|| ui.input(|i| i.modifiers), |(start, _)| start);
    for e in evs {
        #[cfg(not(target_arch = "wasm32"))]
        let chord = {
            if let egui::Event::ModifiersChanged(m) = &e {
                held = *m;
            }
            if matches!(e, egui::Event::Paste(_)) { crate::menus::paste_chord_command(app, held) } else { None }
        };
        let composing = composing_at_start || app.session.tool_composing();
        match e {
            egui::Event::Ime(ime) if wants_text => ime_event(app, ui.ctx(), ime, vi),
            // Keys the IME passes through come as text only while nothing is marked.
            egui::Event::Text(t) if wants_text && !composing => {
                let _ = app.run("text.insert", json!({"text": t}));
            }
            // The IME owns the keyboard until it commits: no typing, clipboard or editing keys.
            egui::Event::Text(_) | egui::Event::Paste(_) | egui::Event::Copy | egui::Event::Cut | egui::Event::Key { .. } if composing => {}
            egui::Event::Paste(t) if wants_text => {
                // Formatted when the system clipboard still holds what was copied here;
                // Shift (⇧⌘V) pastes without formatting.
                let plain = ui.input(|i| i.modifiers.shift);
                let id = if plain { "edit.pasteWithoutFormatting" } else { "edit.paste" };
                let _ = app.run(id, json!({"text": t}));
            }
            egui::Event::Copy | egui::Event::Cut if wants_text => {
                // `run` puts the copied text on the system clipboard.
                let id = if matches!(e, egui::Event::Cut) { "edit.cut" } else { "edit.copy" };
                let _ = app.run(id, json!({}));
            }
            // Paste in Place's chord (⌘⌥⇧V unless rebound) arrives as a paste too.
            #[cfg(not(target_arch = "wasm32"))]
            egui::Event::Paste(_) if chord.is_some() => {
                if let Some(id) = chord
                    && crate::menus::enabled(app, id)
                {
                    crate::menus::activate(app, id, &serde_json::Value::Null);
                }
            }
            // Without a native menu bar the copy and paste keys arrive only as these events.
            egui::Event::Copy | egui::Event::Cut | egui::Event::Paste(_) if !app.native_menu => {
                let (id, params) = match e {
                    egui::Event::Copy => ("edit.copy", serde_json::Value::Null),
                    egui::Event::Cut => ("edit.cut", serde_json::Value::Null),
                    egui::Event::Paste(t) => ("edit.paste", json!({"text": t})),
                    _ => continue,
                };
                if crate::menus::enabled(app, id) {
                    crate::menus::activate(app, id, &params);
                }
            }
            egui::Event::Key { key, pressed: true, modifiers, .. } => tool_key(app, key, modifiers, vi),
            _ => {}
        }
    }
    finish_pending_ime_delete(app, ui.ctx());
    let _ = DVec2::ZERO;
}

/// A key for the active tool (caret movement, nudging, Backspace); Escape it leaves alone
/// deselects.
fn tool_key(app: &mut DesignApp, key: egui::Key, modifiers: egui::Modifiers, vi: designcraft_engine::ViewInfo) {
    let k = match key {
        egui::Key::ArrowLeft => ToolKey::Left,
        egui::Key::ArrowRight => ToolKey::Right,
        egui::Key::ArrowUp => ToolKey::Up,
        egui::Key::ArrowDown => ToolKey::Down,
        egui::Key::Enter => ToolKey::Enter,
        egui::Key::Escape => ToolKey::Escape,
        egui::Key::Backspace => ToolKey::Backspace,
        egui::Key::Delete => ToolKey::Delete,
        egui::Key::Tab => ToolKey::Tab,
        egui::Key::Home => ToolKey::Home,
        egui::Key::End => ToolKey::End,
        _ => return,
    };
    let m = Mods { shift: modifiers.shift, alt: modifiers.alt, cmd: modifiers.command, ctrl: modifiers.ctrl, space: false };
    let handled = app.session.tool_key(k, m, vi).unwrap_or(false);
    if !handled && k == ToolKey::Escape {
        let _ = app.run("edit.deselectAll", json!({}));
    }
    app.after_engine();
}

/// The keys a floating panel's own OS window left over once the shortcuts took theirs: the
/// clipboard and the active tool's keys, as the canvas handles them while no text is edited.
/// `modifiers` are those held in that window; egui-winit sends ⌘C, ⌘X and ⌘V as `Copy`, `Cut`
/// and `Paste` without their key press.
pub(crate) fn forwarded_keys(app: &mut DesignApp, events: Vec<egui::Event>, modifiers: egui::Modifiers) {
    if !keyboard_pane(app) || modal_open(app) || app.session.wants_text() || app.session.tool_composing() {
        return;
    }
    let vi = app.view_info();
    for e in events {
        #[cfg(not(target_arch = "wasm32"))]
        let chord = if matches!(e, egui::Event::Paste(_)) { crate::menus::paste_chord_command(app, modifiers) } else { None };
        // The web keeps floating panels in the page, so nothing is forwarded there.
        #[cfg(target_arch = "wasm32")]
        let chord: Option<&str> = {
            let _ = modifiers;
            None
        };
        let (id, params) = match (e, chord) {
            (egui::Event::Paste(_), Some(id)) => (id, serde_json::Value::Null),
            (egui::Event::Copy, _) if !app.native_menu => ("edit.copy", serde_json::Value::Null),
            (egui::Event::Cut, _) if !app.native_menu => ("edit.cut", serde_json::Value::Null),
            (egui::Event::Paste(t), _) if !app.native_menu => ("edit.paste", json!({"text": t})),
            (egui::Event::Key { key, pressed: true, modifiers, .. }, _) => {
                tool_key(app, key, modifiers, vi);
                continue;
            }
            _ => continue,
        };
        if crate::menus::enabled(app, id) {
            crate::menus::activate(app, id, &params);
        }
    }
}

/// A dialog or Quick Apply has the keyboard. The canvas keeps egui's focus behind a modal, and it
/// runs before the modal draws, so it must not also act on the Esc or Return that closes it.
fn modal_open(app: &DesignApp) -> bool {
    app.ui.dialog.is_some() || app.ui.palette.is_some()
}

/// Does the current pane take the keyboard? Both panes (split, or the second window) see the same
/// events, so typing, tool keys, the clipboard and the input method serve only the focused pane.
fn keyboard_pane(app: &DesignApp) -> bool {
    app.pane == app.focus_pane
}

/// Has the composition these events belong to already ended outside the IME in this batch?
///
/// A pointer press and a document switch end the composition and type its marked text where the
/// caret then is, but the system IME still sends the Commit (or a delete) for the copy it holds.
/// Applying those at the new caret would type the text twice, put it in another story or document,
/// or delete text there, so they are dropped and the IME is told to drop its copy.
fn ime_composition_ended(app: &DesignApp) -> bool {
    app.ime_marked.is_some() && !app.session.tool_composing()
}

/// Did the composition move to another owner (another story, cell or document state) since it
/// began? A late commit then belongs to the old one, not the caret that is there now.
fn ime_composition_moved(app: &DesignApp) -> bool {
    let Some(recorded) = app.ime_target else { return false };
    app.session.ime_text_context().is_some_and(|context| ime_target(&context) != recorded)
}

/// Drop events for a composition that already ended or moved, keeping what was typed.
fn drop_stale_ime_events(app: &mut DesignApp) {
    app.ime_marked = None;
    app.ime_target = None;
    app.pending_ime_delete = None;
    app.ime_discard = true;
    // The host must interrupt the system IME's copy in this pass too, as for a click away.
    app.ime_interrupt = true;
}

/// One input method event for the text caret: marked text previews in the story, a commit types.
fn ime_event(app: &mut DesignApp, ctx: &egui::Context, e: egui::ImeEvent, vi: designcraft_engine::ViewInfo) {
    let r = match e {
        egui::ImeEvent::Preedit { text, active_range_chars } => {
            if text.is_empty() {
                app.ime_marked = None;
                app.ime_target = None;
            } else {
                // The composition's owner at its first marked text: a commit may only land there.
                if app.ime_marked.is_none() {
                    app.ime_target = app.session.ime_text_context().map(|c| ime_target(&c));
                }
                app.ime_marked = Some(text.clone());
            }
            app.session.tool_preedit(&text, active_range_chars, vi).map_err(|e| e.to_string())
        }
        // A composition that already ended or moved in this batch owns nothing any more.
        egui::ImeEvent::Commit(_) | egui::ImeEvent::DeleteSurrounding { .. } | egui::ImeEvent::DeleteSurroundingBytes { .. }
            if app.ime_interrupt || ime_composition_ended(app) || ime_composition_moved(app) =>
        {
            // Dropping a stale delete clears the marked target. Keep the pass's interrupt latch
            // authoritative so the commit following that delete cannot become a fresh insertion.
            drop_stale_ime_events(app);
            Ok(())
        }
        // A bare line break confirms the composition (Enter with some IMEs): the marked text, not
        // a new paragraph.
        egui::ImeEvent::Commit(t) if t == "\n" || t == "\r" => {
            finish_pending_ime_delete(app, ctx);
            keep_marked_text(app);
            Ok(())
        }
        egui::ImeEvent::Commit(t) => ime_commit(app, ctx, &t, vi),
        // The web text agent's corrections and mobile Backspace: characters around the caret go.
        // These are held for the Commit in the same batch so both mutations are one undo step.
        egui::ImeEvent::DeleteSurrounding { before_chars, after_chars } => {
            app.pending_ime_delete = Some((before_chars, after_chars, false));
            Ok(())
        }
        egui::ImeEvent::DeleteSurroundingBytes { before_bytes, after_bytes } => {
            app.pending_ime_delete = Some((before_bytes, after_bytes, true));
            Ok(())
        }
        // `Enabled` and `Disabled` carry nothing: the IME is on while the observation sets `ime`.
        _ => Ok(()),
    };
    if let Err(e) = r {
        app.status(e);
    }
    app.after_engine();
}

/// The bounded committed snapshot and stable target just before an IME mutation.
struct ImeBefore {
    target: ImeTarget,
    stamp: ImeMutationStamp,
    snapshot: IMESurroundingText,
    document_uid: u64,
    document_epoch: u64,
    story: designcraft_doc::StoryId,
    cell: Option<designcraft_doc::CellAddr>,
    anchor: usize,
    focus: usize,
}

/// Build a stable owner from the document uid, story and cell identity, plus the document epoch.
fn ime_target(context: &ImeTextContext<'_>) -> ImeTarget {
    let owner = egui::Id::unique(("designcraft-ime", context.document_uid, context.story.0, context.cell.map(|c| (c.table, c.row, c.col))));
    ImeTarget::new(owner, context.document_epoch)
}

/// Capture the bounded committed surrounding text, mutation stamp, target and selection identity before an IME edit.
fn capture_ime_before(app: &DesignApp) -> Option<ImeBefore> {
    let context = app.session.ime_text_context()?;
    let snapshot = IMESurroundingText::from_text(context.text, context.focus, context.anchor, None);
    let stamp = ImeMutationStamp::from_surrounding_text(snapshot.clone());
    Some(ImeBefore {
        target: ime_target(&context),
        stamp,
        snapshot,
        document_uid: context.document_uid,
        document_epoch: context.document_epoch,
        story: context.story,
        cell: context.cell,
        anchor: context.anchor,
        focus: context.focus,
    })
}

/// Fold a pending delete-surrounding plus commit into one engine operation, clear marked/target state and record the change.
fn ime_commit(app: &mut DesignApp, ctx: &egui::Context, text: &str, vi: designcraft_engine::ViewInfo) -> Result<(), String> {
    let pending = app.pending_ime_delete.take();
    let before = capture_ime_before(app);
    let r = if let Some((before_bytes, after_bytes, bytes)) = pending {
        app.session.ime_commit_after_delete(text, vi, before_bytes, after_bytes, bytes)
    } else {
        app.session.ime_commit(text, vi)
    };
    app.ime_marked = None;
    app.ime_target = None;
    let r = r.map_err(|e| e.to_string());
    if let Ok(mutation) = &r {
        note_ime_change(app, ctx, before, mutation.as_ref(), pending);
    }
    r.map(|_| ())
}

/// Apply a pending IME delete unless its composition ended or moved; record the mutation and drop stale marked text.
fn finish_pending_ime_delete(app: &mut DesignApp, ctx: &egui::Context) {
    let Some((before_bytes, after_bytes, bytes)) = app.pending_ime_delete.take() else { return };
    if ime_composition_ended(app) || ime_composition_moved(app) {
        // The composition ended earlier in this batch: its delete belongs to the copy the system
        // IME still holds, not to the caret that is there now.
        drop_stale_ime_events(app);
        return;
    }
    let before = capture_ime_before(app);
    match app.session.ime_delete_surrounding(before_bytes, after_bytes, bytes) {
        Ok(mutation) => {
            // The delete drops the composition (its marked text is not typed): the IME's copy goes.
            if !app.session.tool_composing() && app.ime_marked.take().is_some() {
                app.ime_discard = true;
            }
            if !app.session.tool_composing() {
                app.ime_target = None;
            }
            note_ime_change(app, ctx, before, mutation.as_ref(), Some((before_bytes, after_bytes, bytes)));
        }
        Err(e) => app.status(e.to_string()),
    }
    app.after_engine();
}

/// Record the origin of a successful IME mutation.
///
/// A receipt is earned only when the engine reported the exact mutation it applied and the bounded
/// committed snapshot agrees with it. Smart quotes, autocorrect, tracked changes and any other
/// application transformation therefore report `Other`, as do failed, stale and no-op mutations.
fn note_ime_change(
    app: &mut DesignApp,
    ctx: &egui::Context,
    before: Option<ImeBefore>,
    mutation: Option<&ImeMutation>,
    delete: Option<(usize, usize, bool)>,
) {
    let Some(before) = before else { return };
    let Some(after) = app.session.ime_text_context() else {
        let viewport = ctx.viewport_id();
        app.ime_tracker.note_application_change(viewport, ctx.cumulative_frame_nr_for(viewport), before.target);
        return;
    };
    if ime_target(&after) != before.target {
        return;
    }
    let after_snapshot = IMESurroundingText::from_text(after.text, after.focus, after.anchor, None);
    if &after_snapshot == before.stamp.surrounding_text() {
        return;
    }
    let after_stamp = ImeMutationStamp::from_surrounding_text(after_snapshot.clone());
    let viewport = ctx.viewport_id();
    let frame_nr = ctx.cumulative_frame_nr_for(viewport);
    if mutation.is_some_and(|m| ime_change_is_exact(&before, &after_snapshot, m, delete)) {
        app.ime_tracker.note_input_method_apply(viewport, frame_nr, before.target, before.stamp, after_stamp);
    } else {
        app.ime_tracker.note_application_change(viewport, frame_nr, before.target);
    }
}

/// Does the bounded post-mutation snapshot agree with the mutation the engine applied?
///
/// The expected result is built from the reported ranges and typed text — absolute byte offsets in
/// the whole story, not the truncated outgoing window — so a delete that reaches past the window
/// still matches. Every visible byte of the after window must agree with it, but a mutation the
/// after window does not overlap at all (a delete longer than the window) rests on the engine's
/// report alone, which is exact because the engine computed it from the whole story. A rewrite,
/// tracked changes or a replaced story is not a receipt.
fn ime_change_is_exact(before: &ImeBefore, after: &IMESurroundingText, m: &ImeMutation, delete: Option<(usize, usize, bool)>) -> bool {
    // The same target, selection and text the receipt was captured around.
    if m.document_uid != before.document_uid
        || m.document_epoch != before.document_epoch
        || m.story != before.story
        || m.cell != before.cell
        || m.anchor_before != before.anchor
        || m.focus_before != before.focus
    {
        return false;
    }
    let (Some(before_payload), Some(after_payload)) = (before.snapshot.available(), after.available()) else { return false };
    let before_text = before_payload.text.as_str();
    let after_text = after_payload.text.as_str();
    let selection = m.anchor_before.min(m.focus_before)..m.anchor_before.max(m.focus_before);
    // The reported deletion adjoins the selection, is inside the text, and exists only for a
    // delete-surrounding event. Its exact extent is the engine's: it computes the ranges from the
    // whole story with the requested counts.
    if m.deleted_before.end != selection.start
        || m.deleted_after.start != selection.end
        || m.deleted_before.start > selection.start
        || m.deleted_after.end > m.text_len_before
    {
        return false;
    }
    if delete.is_none() && (!m.deleted_before.is_empty() || !m.deleted_after.is_empty()) {
        return false;
    }
    // The story changed by exactly the removed bytes, the replaced selection and the typed text:
    // tracked changes and an inserted paragraph return (a table anchor) break this identity.
    let replaced = if m.inserted.is_empty() { 0 } else { selection.len() };
    let expected_len = m
        .text_len_before
        .saturating_sub(m.deleted_before.len())
        .saturating_sub(m.deleted_after.len())
        .saturating_sub(replaced)
        .saturating_add(m.inserted.len());
    if m.text_len_after != expected_len {
        return false;
    }
    // A commit collapses the caret after the typed text; a delete keeps the selection, shifted.
    let (want_anchor, want_focus) = if m.inserted.is_empty() {
        (m.anchor_before.saturating_sub(m.deleted_before.len()), m.focus_before.saturating_sub(m.deleted_before.len()))
    } else {
        let at = m.deleted_before.start + m.inserted.len();
        (at, at)
    };
    if m.anchor_after != want_anchor || m.focus_after != want_focus {
        return false;
    }
    // The text around the caret, rebuilt from the reported mutation. A snapshot's window starts
    // before its selection's start by the selection's own offset in that window.
    let before_win = selection.start.saturating_sub(before_payload.cursor.min(before_payload.anchor));
    let after_selection = m.anchor_after.min(m.focus_after)..m.anchor_after.max(m.focus_after);
    let after_win = after_selection.start.saturating_sub(after_payload.cursor.min(after_payload.anchor));
    if after_win.saturating_add(after_payload.cursor) != m.focus_after || after_win.saturating_add(after_payload.anchor) != m.anchor_after {
        return false;
    }
    let before_end = before_win.saturating_add(before_text.len());
    let after_end = after_win.saturating_add(after_text.len());
    if before_end > m.text_len_before || after_end > m.text_len_after {
        return false;
    }
    let kept = if m.inserted.is_empty() { selection.len() } else { 0 };
    let inserted_post = m.deleted_before.start + kept;
    let tail_post = inserted_post + m.inserted.len();
    let compared = (|| -> Option<usize> {
        let mut n = 0usize;
        n += ime_compare_kept(before_text, before_win..before_end, 0..m.deleted_before.start, 0, after_text, after_win..after_end)?;
        if kept > 0 {
            n += ime_compare_kept(before_text, before_win..before_end, selection.clone(), m.deleted_before.start, after_text, after_win..after_end)?;
        }
        n += ime_compare_window(after_text, after_win..after_end, &(inserted_post..tail_post), &m.inserted)?;
        n += ime_compare_kept(
            before_text,
            before_win..before_end,
            m.deleted_after.end..m.text_len_before,
            tail_post,
            after_text,
            after_win..after_end,
        )?;
        Some(n)
    })();
    compared.is_some()
}

/// Compare one unchanged piece of the story: the pre-mutation bytes `pre` (absolute) appear at
/// post-mutation offset `post_start`. Returns the number of bytes compared, or `None` on a
/// mismatch. Bytes outside the bounded before window cannot be checked and are skipped.
fn ime_compare_kept(
    before_text: &str,
    before_win: std::ops::Range<usize>,
    pre: std::ops::Range<usize>,
    post_start: usize,
    after_text: &str,
    after_win: std::ops::Range<usize>,
) -> Option<usize> {
    let from = pre.start.max(before_win.start);
    let to = pre.end.min(before_win.end);
    if from >= to {
        return Some(0);
    }
    let expected = before_text.get(from - before_win.start..to - before_win.start)?;
    let post = post_start + (from - pre.start)..post_start + (to - pre.start);
    ime_compare_window(after_text, after_win, &post, expected)
}

/// Compare `expected`, which begins at absolute post-mutation offset `post.start`, with what the
/// after window shows of it. Returns the number of bytes compared; `None` when they differ.
fn ime_compare_window(after_text: &str, after_win: std::ops::Range<usize>, post: &std::ops::Range<usize>, expected: &str) -> Option<usize> {
    let from = post.start.max(after_win.start);
    let to = post.end.min(after_win.end);
    if from >= to {
        return Some(0);
    }
    let actual = after_text.get(from - after_win.start..to - after_win.start)?;
    let skip = from - post.start;
    let want = expected.get(skip..skip + (to - from))?;
    (actual == want).then_some(to - from)
}

/// End a composition outside the IME (a click away, the canvas losing the keyboard): the marked
/// text stays as typed, and the system IME is told to drop its copy.
pub(crate) fn keep_marked_text(app: &mut DesignApp) {
    if app.ime_marked.take().is_some() {
        app.ime_discard = true;
    }
    app.ime_target = None;
    if app.session.tool_composing() {
        if let Err(e) = app.session.end_composition() {
            app.status(e.to_string());
        }
        app.after_engine();
    }
}

/// What the canvas found for this viewport pass, observed once at the end of the pass.
pub(crate) struct CanvasIme {
    /// The viewport it was found in: the second window's canvas runs inside the root's pass and
    /// must not consume the root's observation.
    viewport: egui::ViewportId,
    /// The caret's screen rectangle, when the canvas owns the text caret this pass.
    caret: Option<Rect>,
}

/// While the text caret is in the canvas, let the system IME compose (egui-winit allows it only in
/// frames that set `ime`; the web build focuses its hidden text input then) and keep the candidate
/// window at the caret, through zoom, scrolling, view rotation and vertical text.
///
/// The caret is only recorded here: the observation waits for the end of the viewport's pass, so
/// a command or a field that takes the keyboard in a later section of the UI is included.
fn ime_canvas_pass(app: &mut DesignApp, ctx: &egui::Context, xf: &Xf) {
    if !keyboard_pane(app) {
        // Both panes see the same viewport and events. The focused pane owns this viewport's one
        // observation; observing for the other would reset the context every split-window frame.
        return;
    }
    // A text field, dialog or Quick Apply has the keyboard: egui's own text editing serves the IME.
    let ours = app.session.wants_text() && !ctx.text_edit_focused() && !modal_open(app);
    let caret = if ours { app.session.tool_ime_caret(app.view_info()) } else { None };
    let caret = caret.map(|(a, b)| Rect::from_two_pos(xf.to_screen(a), xf.to_screen(b)).expand(1.0));
    app.canvas_ime = caret.is_some();
    app.ime_canvas = Some(CanvasIme { viewport: ctx.viewport_id(), caret });
}

/// Observe the IME once for this viewport pass, after every command and focus change it applied.
pub(crate) fn ime_observe(app: &mut DesignApp, ctx: &egui::Context) {
    let viewport = ctx.viewport_id();
    if app.ime_canvas.as_ref().is_none_or(|canvas| canvas.viewport != viewport) {
        return;
    }
    let Some(canvas) = app.ime_canvas.take() else { return };
    // A text field, dialog or Quick Apply that took the keyboard later in the pass owns the IME.
    let ours = canvas.caret.is_some() && !ctx.text_edit_focused() && !modal_open(app);
    let Some(r) = canvas.caret.filter(|_| ours) else {
        // The IME goes away with its marked text: keep that text as typed, like a click away.
        keep_marked_text(app);
        app.ime_tracker.observe(viewport, None);
        return;
    };
    // The composition ended outside the IME (a click, a command): the IME drops what it has marked.
    let interrupt = app.ime_interrupt || (app.ime_marked.is_some() && !app.session.tool_composing());
    app.ime_interrupt = false;
    if interrupt {
        app.ime_marked = None;
        app.ime_target = None;
        app.ime_discard = true;
    }
    app.canvas_ime = true;
    // One observation per viewport pass, after this pass's events and commands. The committed
    // text excludes any active pre-edit, and the tracker copies only the bounded IME window.
    let update = match app.session.ime_text_context() {
        Some(context) => {
            let observation = ImeObservation::new(ctx.cumulative_frame_nr_for(viewport), ctx.cumulative_pass_nr_for(viewport), ime_target(&context))
                .purpose(egui::IMEPurpose::Normal)
                .committed(Some(CommittedText::new(context.text, context.focus, context.anchor)));
            app.ime_tracker.observe(viewport, Some(observation))
        }
        None => app.ime_tracker.observe(viewport, None),
    };
    let Some(update) = update else { return };
    ctx.output_mut(|o| {
        o.ime = Some(egui::output::IMEOutput {
            owner: update.owner,
            context_generation: update.context_generation,
            purpose: update.purpose,
            rect: r,
            cursor_rect: r,
            should_interrupt_composition: interrupt,
            reset_request: None,
            change_cause: update.change_cause,
            surrounding_text: update.surrounding_text,
        });
    });
}

/// View → Show Hidden Characters: ¶ paragraph ends, » tabs, · spaces, ¬ forced line breaks, # end of story.
fn draw_hidden_characters(app: &DesignApp, painter: &egui::Painter, xf: &Xf, doc: &Document, layout: &CanvasLayout) {
    for story in doc.stories.values() {
        let cs = app.session.cache.get(doc, story.id, None);
        for ft in &cs.frames {
            let (Some((a, _)), Some(it)) = (item_canvas_xf(doc, layout, ft.frame), doc.item(ft.frame)) else { continue };
            let m = a * doc.text_xf(it);
            let col = layer_color(doc, it);
            for l in &ft.lines {
                let size = (l.ascent * 0.75 * xf.zoom).clamp(6.0, 40.0) as f32;
                let font = egui::FontId::proportional(size);
                for (x, mark, centered) in hidden_marks(&story.text, l) {
                    let p = xf.to_screen(m * Point::new(x, l.baseline));
                    let align = if centered { egui::Align2::CENTER_BOTTOM } else { egui::Align2::LEFT_BOTTOM };
                    painter.text(p, align, mark, font.clone(), col);
                }
            }
        }
    }
}

/// The hidden-character marks on line `l` of a story with text `text`: frame-space x, mark, and
/// whether the mark is centered on x (spaces) or starts at it.
fn hidden_marks(text: &str, l: &compose::Line) -> Vec<(f64, &'static str, bool)> {
    let mut out = Vec::new();
    for g in &l.glyphs {
        if g.len == 0 {
            continue;
        }
        let Some(ch) = text.get(g.byte..).and_then(|t| t.chars().next()) else { continue };
        let mark = match ch {
            ' ' => "·",
            '\t' => "»",
            '\u{2028}' => "¬",
            '\u{A0}' => "°",
            _ => continue,
        };
        if ch == ' ' {
            out.push((g.x + g.adv / 2.0, mark, true));
        } else {
            out.push((g.x, mark, false));
        }
    }
    if l.last_in_para {
        // The spaces trimmed off the line's end sit past `end_x`; the paragraph mark follows them.
        let end = l.glyphs.iter().filter(|g| g.len > 0 && !g.visible && g.x >= l.end_x - 1e-9).map(|g| g.x + g.adv).fold(l.end_x, f64::max);
        let mark = if l.range.end >= text.len() { "#" } else { "¶" };
        out.push((end + 1.0, mark, false));
    }
    out
}

/// View › Flattener Preview: red over every object that involves transparency (opacity, blend
/// modes, effects, knockout) — what flattening or export rasterising affects.
fn draw_flattener_preview(painter: &egui::Painter, xf: &Xf, doc: &Document, layout: &CanvasLayout) {
    fn transparent(it: &Item) -> bool {
        it.opacity < 1.0 || it.blend != Default::default() || it.effects.any() || it.knockout || it.children().iter().any(|c| transparent(c))
    }
    let red = Color32::from_rgba_unmultiplied(230, 30, 30, 110);
    for slot in &layout.slots {
        let Some(sp) = doc.spread(slot.spread) else { continue };
        for it in sp.items.iter().filter(|it| !it.hidden && transparent(it)) {
            let r = it.bounds().inflate(designcraft_render::effect_outset(it), designcraft_render::effect_outset(it));
            painter.rect_filled(xf.rect(slot.xf.transform_rect_bbox(r)), 0.0, red);
        }
    }
}

/// Edit › Spelling › Dynamic Spelling: red squiggles under misspelled words (checked once per
/// story version).
fn draw_dynamic_spelling(app: &DesignApp, ctx: &egui::Context, painter: &egui::Painter, xf: &Xf, doc: &Document, layout: &CanvasLayout) {
    for story in doc.stories.values() {
        let key = egui::Id::unique(("dyn_spell", std::sync::Arc::as_ptr(story) as usize, doc.user_words.len()));
        let bad: std::sync::Arc<Vec<std::ops::Range<usize>>> = match ctx.data(|d| d.get_temp(key)) {
            Some(v) => v,
            None => {
                let v = std::sync::Arc::new(designcraft_engine::cmd::spelling::misspellings(doc, story, &doc.user_words));
                ctx.data_mut(|d| d.insert_temp(key, v.clone()));
                v
            }
        };
        if bad.is_empty() {
            continue;
        }
        let cs = app.session.cache.get(doc, story.id, None);
        let red = Stroke::new(1.0, Color32::from_rgb(230, 30, 30));
        for ft in &cs.frames {
            let (Some((a, _)), Some(it)) = (item_canvas_xf(doc, layout, ft.frame), doc.item(ft.frame)) else { continue };
            let m = a * doc.text_xf(it);
            for l in &ft.lines {
                for r in bad.iter().filter(|r| r.start < l.range.end && r.end > l.range.start) {
                    let gs: Vec<_> = l.glyphs.iter().filter(|g| g.len > 0 && g.byte >= r.start && g.byte < r.end).collect();
                    let (Some(x0), Some(x1)) = (gs.iter().map(|g| g.x).reduce(f64::min), gs.iter().map(|g| g.x + g.adv).reduce(f64::max)) else {
                        continue;
                    };
                    let y = l.baseline + l.descent * 0.5;
                    let (p0, p1) = (xf.to_screen(m * Point::new(x0, y)), xf.to_screen(m * Point::new(x1, y)));
                    // A zigzag 2 px high, 4 px per wave.
                    let n = ((p1.x - p0.x) / 2.0).max(1.0) as usize;
                    let pts: Vec<Pos2> =
                        (0..=n).map(|k| pos2(p0.x + (p1.x - p0.x) * k as f32 / n as f32, p0.y + if k % 2 == 0 { 0.0 } else { 2.0 })).collect();
                    painter.add(egui::Shape::line(pts, red));
                }
            }
        }
    }
}

/// Plain text of the current text selection (story markers resolved to readable characters).
pub fn selected_text(app: &DesignApp) -> Option<String> {
    let st = app.session.active()?;
    let t = st.selection.text?;
    let story = st.doc.text_story(t.story, t.cell)?;
    let s = story.slice(t.range());
    if s.is_empty() {
        return None;
    }
    Some(s.replace(designcraft_doc::story::FORCED_LINE_BREAK, "\n"))
}

/// One device pixel, the width InDesign uses for guides and frame edges.
fn hair(painter: &egui::Painter) -> f32 {
    1.0 / painter.ctx().pixels_per_point()
}

const ZERO_POINT_DRAG: &str = "canvas_zero_point_drag";

/// The ruler corner: drag from it and the zero point lands where the pointer is released; a click
/// that stays in the corner leaves it; a double-click resets it. Locked, the corner ignores both.
/// Cmd-dragged (Ctrl elsewhere), it makes a horizontal and a vertical guide instead.
/// Holds the pointer position while dragging; returns true while it owns the pointer.
fn zero_point_drag(app: &mut DesignApp, ui: &mut egui::Ui, resp: &egui::Response, rect: Rect, xf: &Xf) -> bool {
    let id = egui::Id::unique((ZERO_POINT_DRAG, app.pane));
    let corner = Rect::from_min_max(resp.rect.min, rect.min);
    let (pressed, released, down, origin, latest, dbl, command) = ui.input(|i| {
        (
            i.pointer.primary_pressed(),
            i.pointer.primary_released(),
            i.pointer.primary_down(),
            i.pointer.press_origin(),
            i.pointer.latest_pos(),
            i.pointer.button_double_clicked(egui::PointerButton::Primary),
            i.modifiers.command,
        )
    });
    let mut at: Option<Pos2> = ui.data(|d| d.get_temp(id));
    if at.is_none() {
        if !pressed || !resp.hovered() || app.session.active().is_none() || corner.width() <= 0.0 || corner.height() <= 0.0 {
            return false;
        }
        at = origin.filter(|o| corner.contains(*o));
    }
    let Some(mut p) = at else { return false };
    if let Some(l) = latest.filter(|l| l.x.is_finite() && l.y.is_finite()) {
        p = l;
    }
    if !released && down {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
        ui.data_mut(|d| d.insert_temp(id, p));
        return true;
    }
    ui.data_mut(|d| d.remove::<Pos2>(id));
    if !released {
        // The release happened where we couldn't see it (outside the window).
        return false;
    }
    let locked = app.session.active().is_none_or(|st| st.doc.settings.zero_point_locked);
    if command {
        if !corner.contains(p) && rect.contains(p) {
            corner_guides(app, xf, p);
        }
    } else if !locked {
        if dbl {
            let _ = app.run("view.zeroPoint", json!({"reset": true}));
        } else if !corner.contains(p)
            && let Some((zp, _)) = zero_point_at(app, xf, p)
        {
            let _ = app.run("view.zeroPoint", json!({ "at": zp }));
        }
    }
    true
}

/// A screen point as a zero point (its offset from the ruler origin of the spread in view, kept
/// on the pasteboard as the engine keeps it), and where that zero point is on the canvas.
fn zero_point_at(app: &DesignApp, xf: &Xf, p: Pos2) -> Option<([f64; 2], Point)> {
    let st = app.session.active()?;
    let layout = CanvasLayout::new(&st.doc, st.editing_parents);
    let slot = layout.slots.get(current_slot(app, &layout)?)?;
    let sp = slot.to_spread(xf.to_canvas(p));
    let base = st.doc.ruler_base(slot.spread, sp.x)?;
    let zp = st.doc.clamp_zero_point([sp.x - base.x, sp.y - base.y])?;
    Some((zp, slot.to_canvas(Point::new(base.x + zp[0], base.y + zp[1]))))
}

/// Cmd-drag from the ruler corner: a horizontal and a vertical spread guide through the drop
/// point, on the spread under it.
fn corner_guides(app: &mut DesignApp, xf: &Xf, p: Pos2) {
    if !app.ui.guides || app.ui.guides_locked {
        return;
    }
    let Some((r, sp)) = app.session.active().and_then(|st| CanvasLayout::new(&st.doc, st.editing_parents).spread_at(xf.to_canvas(p))) else {
        return;
    };
    if !(sp.x.is_finite() && sp.y.is_finite()) {
        return;
    }
    let _ = app.run("guide.add", json!({"spread": r, "orientation": "both", "point": [sp.x, sp.y], "spreadGuide": true}));
}

const GUIDE_DRAG: &str = "canvas_guide_drag";

/// A ruler guide being dragged: out of a ruler (`from` None) or an existing one.
#[derive(Clone, Debug)]
struct GuideDrag {
    orientation: designcraft_doc::Orientation,
    from: Option<(designcraft_doc::SpreadRef, usize, usize)>,
    /// Pointer position (screen).
    at: Option<Pos2>,
    /// Modifiers held at the pointer's last move: Shift snaps to the ruler's ticks, Command
    /// suspends snapping.
    mods: egui::Modifiers,
}

const GUIDE_PRESS: &str = "canvas_guide_press";

/// A press on an existing guide: where it was (screen) and whether Shift was down. Released
/// where it was pressed, it is a click, which selects the guide.
#[derive(Clone, Copy, Debug)]
struct GuidePress {
    origin: Pos2,
    shift: bool,
}

/// How far (screen pixels) the pointer may move between press and release for a click.
const GUIDE_CLICK_SLOP: f32 = 3.0;

/// Where a dragged guide is, as the rulers measure it (#1282): `X: …` for a vertical guide, `Y: …`
/// for a horizontal one, from the zero point (and the page origin, when rulers restart per page)
/// in the document's ruler units. `None` with no document or rulers.
fn guide_readout(app: &DesignApp, xf: &Xf, g: &GuideDrag) -> Option<String> {
    let (label, value, unit) = guide_measure(app, xf, g)?;
    Some(format!("{label}: {}", designcraft_geom::units::format_measure(value, unit)))
}

/// Where a pointer readout of `size` goes for a pointer at `p`: below right of it, on the other
/// side where that would leave the canvas `area` (near its right or bottom edge), and kept inside
/// it (with the label's 3-point frame) however small the canvas is.
fn readout_at(p: Pos2, size: egui::Vec2, area: Rect) -> Pos2 {
    const GAP: f32 = 12.0;
    const PAD: f32 = 3.0;
    let x = if p.x + GAP + size.x + PAD > area.max.x { p.x - GAP - size.x } else { p.x + GAP };
    let y = if p.y + GAP + size.y + PAD > area.max.y { p.y - GAP - size.y } else { p.y + GAP };
    let clamp = |v: f32, lo: f32, hi: f32| if hi < lo { lo } else { v.clamp(lo, hi) };
    pos2(clamp(x, area.min.x + PAD, area.max.x - PAD - size.x), clamp(y, area.min.y + PAD, area.max.y - PAD - size.y))
}

/// [`guide_readout`]'s parts: the axis label, the value (points) and its unit.
fn guide_measure(app: &DesignApp, xf: &Xf, g: &GuideDrag) -> Option<(&'static str, f64, designcraft_geom::Unit)> {
    let p = g.at?;
    let st = app.session.active()?;
    let layout = CanvasLayout::new(&st.doc, st.editing_parents);
    // Measured where the guide is dropped (`guide_drag`): its snapped spot, on the spread under
    // the pointer, as the rulers measure that spread, not the one in the middle of the view.
    let (c, slot) = match guide_spot(app, xf, g) {
        Some(spot) => (layout.to_canvas(spot.spread, spot.sp), layout.slot(spot.spread)?),
        None => (xf.to_canvas(p), current_slot(app, &layout).and_then(|i| layout.slots.get(i))?),
    };
    // An existing guide moves only on its own spread (`guide_drag`): over another, dropping it
    // leaves it where it was, so there is no place to show.
    if g.from.is_some_and(|(r, _, _)| r != slot.spread) {
        return None;
    }
    let pieces = ruler_pieces_of(&st.doc, slot, st.doc.zero_point());
    let first = pieces.first()?;
    let (label, value, unit) = match g.orientation {
        designcraft_doc::Orientation::Vertical => {
            // The horizontal ruler restarts at each page: the piece under the guide measures it.
            let o = pieces.iter().find(|(from, to, _)| *from <= c.x && c.x < *to).unwrap_or(first).2;
            ("X", c.x - o.x, st.doc.settings.horizontal_units)
        }
        designcraft_doc::Orientation::Horizontal => ("Y", c.y - first.2.y, st.doc.settings.vertical_units),
    };
    value.is_finite().then_some((label, value, unit))
}

/// Where a dragged ruler guide is (#1221).
struct GuideSpot {
    /// The spread under the pointer.
    spread: SpreadRef,
    /// The pointer in that spread's coordinates.
    pointer: Point,
    /// [`Self::pointer`] with the guide's axis snapped.
    sp: Point,
    /// [`Self::sp`] on screen, where the preview line goes.
    screen: Pos2,
    /// Alignment marks (canvas coordinates) when an object or page edge or center caught it.
    marks: Vec<Overlay>,
}

/// Where the guide of drag `g` is: its axis (x for a vertical guide, y for a horizontal one)
/// snapped as a moved object's is, to ruler guides, margins, columns, bleed, the document grid and
/// smart-guide targets (not the baseline grid, which would catch a horizontal guide almost
/// anywhere). Shift snaps to the ruler's ticks instead and Command suspends snapping. The guides
/// being dragged are not targets. An existing guide over another spread, where it
/// can't be dropped, stays at the pointer. The preview, the readout and the drop all read it.
/// `None` with no spread under the pointer.
fn guide_spot(app: &DesignApp, xf: &Xf, g: &GuideDrag) -> Option<GuideSpot> {
    let p = g.at?;
    let st = app.session.active()?;
    let layout = CanvasLayout::new(&st.doc, st.editing_parents);
    let (r, pointer) = layout.spread_at(xf.to_canvas(p))?;
    let spot = |sp: Point, marks: Vec<Overlay>| GuideSpot { spread: r, pointer, sp, screen: xf.to_screen(layout.to_canvas(r, sp)), marks };
    if g.mods.command || g.from.is_some_and(|(from, _, _)| from != r) {
        return Some(GuideSpot { spread: r, pointer, sp: pointer, screen: p, marks: Vec::new() });
    }
    let vertical = matches!(g.orientation, designcraft_doc::Orientation::Vertical);
    let place = |v: f64| if vertical { Point::new(v, pointer.y) } else { Point::new(pointer.x, v) };
    if g.mods.shift {
        let slot = layout.slot(r)?;
        let at = ruler_tick_of(&st.doc, slot, g.orientation, xf.zoom, pointer).map_or(pointer, place);
        return Some(spot(at, Vec::new()));
    }
    let spread = st.doc.spread(r)?;
    // Not targets: the dragged guide, and the others of a selected group, which move with it.
    let exclude: Vec<GuideSel> = match g.from {
        Some((sr, pi, gi)) if drags_group(app, GuideSel { spread: sr, page: pi, index: gi }) => st.selection.guides.to_vec(),
        Some((sr, pi, gi)) => vec![GuideSel { spread: sr, page: pi, index: gi }],
        None => Vec::new(),
    };
    // How far the guide reaches across its axis: its page, or the whole spread.
    let page = match g.from {
        Some((_, pi, gi)) => spread.pages.get(pi).filter(|pg| pg.guides.get(gi).is_some_and(|guide| !guide.spread)),
        None => spread.pages.iter().find(|pg| pg.bounds().contains(pointer)),
    };
    let reach = page.map_or_else(|| spread.bounds(), |pg| pg.bounds());
    let (at, rect, x_edges, y_edges) = if vertical {
        (pointer.x, DRect::new(pointer.x, reach.y0, pointer.x, reach.y1), [true; 3], [false; 3])
    } else {
        (pointer.y, DRect::new(reach.x0, pointer.y, reach.x1, pointer.y), [false; 3], [true; 3])
    };
    let cx = designcraft_tools::ToolContext {
        doc: &st.doc,
        selection: &st.selection,
        cache: &app.session.cache,
        layout: &layout,
        zoom: xf.zoom.max(1e-6),
        layer: st.active_layer,
        snap: app.ui.snap_view(),
        unit: app.ui.units,
        stroke_in_front: false,
    };
    let s = designcraft_tools::snap::snap(
        &cx,
        designcraft_tools::SnapRequest {
            spread: r,
            gesture: designcraft_tools::Gesture::Guide,
            rect,
            x_edges,
            y_edges,
            exclude: &[],
            exclude_guides: &exclude,
            copying: false,
            lengths: [None, None],
            angle: None,
            radius: 0.0,
            pointer,
        },
    );
    let snapped = at + if vertical { s.delta.x } else { s.delta.y };
    Some(if snapped.is_finite() { spot(place(snapped), s.guides) } else { spot(pointer, Vec::new()) })
}

/// The nearest ruler tick to spread point `sp` along a guide of `orientation`'s axis, in spread
/// coordinates: the horizontal ruler's (the piece under the point) where that axis runs across the
/// screen, else the vertical ruler's. `None` when the ruler has no ticks there.
fn ruler_tick_of(doc: &Document, slot: &SpreadSlot, orientation: designcraft_doc::Orientation, zoom: f64, sp: Point) -> Option<f64> {
    let vertical = matches!(orientation, designcraft_doc::Orientation::Vertical);
    // A quarter-turned spread runs its x axis down the screen.
    let across = vertical == slot.rotation.is_multiple_of(2);
    let c = slot.to_canvas(sp);
    let pieces = ruler_pieces_of(doc, slot, doc.zero_point());
    let step = |unit: designcraft_geom::Unit| {
        let (major, sub) = unit.ruler_ticks(zoom);
        let step = major / f64::from(sub.max(1));
        (step.is_finite() && step > 0.0).then_some(step)
    };
    let tick = |v: f64, o: f64, step: f64| o + ((v - o) / step).round() * step;
    let c = if across {
        let &(_, _, o) = pieces.iter().find(|(from, to, _)| *from <= c.x && c.x < *to).or(pieces.first())?;
        Point::new(tick(c.x, o.x, step(doc.settings.horizontal_units)?), c.y)
    } else {
        let o = pieces.first()?.2;
        Point::new(c.x, tick(c.y, o.y, step(doc.settings.vertical_units)?))
    };
    let s = slot.to_spread(c);
    Some(if vertical { s.x } else { s.y }).filter(|v| v.is_finite())
}

/// The guide under a screen point (within 3 px), as (spread, page, index, orientation).
fn guide_at(app: &DesignApp, xf: &Xf, p: Pos2) -> Option<(designcraft_doc::SpreadRef, usize, usize, designcraft_doc::Orientation)> {
    let st = app.session.active()?;
    let layout = CanvasLayout::new(&st.doc, st.editing_parents);
    let cp = xf.to_canvas(p);
    let tol = 3.0 / xf.zoom;
    for slot in &layout.slots {
        let sp = st.doc.spread(slot.spread)?;
        let local = slot.to_spread(cp);
        for (pi, pg) in sp.pages.iter().enumerate() {
            let pr = if pg.guides.iter().any(|g| g.spread) { slot.xf.inverse().transform_rect_bbox(layout.pasteboard) } else { pg.bounds() };
            for (gi, g) in pg.guides.iter().enumerate() {
                if !g.editable_in(&st.doc) {
                    continue;
                }
                let r = if g.spread { pr } else { pg.bounds() };
                let hit = match g.orientation {
                    designcraft_doc::Orientation::Horizontal => (local.y - g.position).abs() <= tol && local.x >= r.x0 && local.x <= r.x1,
                    designcraft_doc::Orientation::Vertical => (local.x - g.position).abs() <= tol && local.y >= r.y0 - tol && local.y <= r.y1 + tol,
                };
                if hit && !g.locked {
                    return Some((slot.spread, pi, gi, g.orientation));
                }
            }
        }
    }
    None
}

/// A primary press at `o` on a ruler starts a new guide; on a guide it selects and grabs it.
fn begin_guide_drag(app: &mut DesignApp, ui: &mut egui::Ui, resp: &egui::Response, rect: Rect, xf: &Xf, o: Pos2) -> Option<GuideDrag> {
    let full = resp.rect;
    if full.contains(o) && !rect.contains(o) {
        // In a ruler (not the corner).
        let orientation = if o.y < rect.min.y && o.x >= rect.min.x {
            Some(designcraft_doc::Orientation::Horizontal)
        } else if o.x < rect.min.x && o.y >= rect.min.y {
            Some(designcraft_doc::Orientation::Vertical)
        } else {
            None
        };
        if let Some(orientation) = orientation {
            return Some(GuideDrag { orientation, from: None, at: Some(o), mods: egui::Modifiers::NONE });
        }
    } else if rect.contains(o)
        && matches!(app.session.tool_id(), "selection" | "directSelection")
        && let Some((r, pi, gi, orientation)) = guide_at(app, xf, o)
    {
        let shift = ui.input(|i| i.modifiers.shift);
        // A press on a guide selects it (Shift: on release, as a click adds or removes it).
        let selected = app.session.active().is_some_and(|st| st.selection.guides.contains(&GuideSel { spread: r, page: pi, index: gi }));
        if !shift && !selected {
            let _ = app.run("guide.select", json!({"spread": r, "page": pi, "index": gi}));
        }
        ui.data_mut(|d| d.insert_temp(egui::Id::unique(GUIDE_PRESS), GuidePress { origin: o, shift }));
        return Some(GuideDrag { orientation, from: Some((r, pi, gi)), at: Some(o), mods: egui::Modifiers::NONE });
    }
    None
}

/// Ruler guides: drag one out of a ruler, drag an existing one (Selection tools) to move it, drop
/// it back on a ruler to delete it. A press near an unlocked guide takes it even over an object:
/// the guide is selected and the object is not. Returns true from the press to the release, so
/// the tools never see that press.
fn guide_drag(app: &mut DesignApp, ui: &mut egui::Ui, resp: &egui::Response, rect: Rect, xf: &Xf) -> bool {
    let id = egui::Id::unique(GUIDE_DRAG);
    let (pressed, released, origin, latest) =
        ui.input(|i| (i.pointer.primary_pressed(), i.pointer.primary_released(), i.pointer.press_origin(), i.pointer.latest_pos()));
    let mut drag: Option<GuideDrag> = ui.data(|d| d.get_temp(id));
    if drag.is_none() && pressed && resp.hovered() && app.ui.guides && !app.ui.guides_locked {
        let Some(o) = origin else { return false };
        drag = begin_guide_drag(app, ui, resp, rect, xf, o);
    }
    let Some(mut g) = drag else { return false };
    if let Some(p) = latest {
        g.at = Some(p);
    }
    g.mods = ui.input(|i| i.modifiers);
    if !released {
        ui.ctx().set_cursor_icon(match g.orientation {
            designcraft_doc::Orientation::Horizontal => egui::CursorIcon::ResizeVertical,
            designcraft_doc::Orientation::Vertical => egui::CursorIcon::ResizeHorizontal,
        });
        ui.data_mut(|d| d.insert_temp(id, g));
        return true;
    }
    ui.data_mut(|d| d.remove::<GuideDrag>(id));
    let press: Option<GuidePress> = ui.data_mut(|d| {
        let press = d.get_temp(egui::Id::unique(GUIDE_PRESS));
        d.remove::<GuidePress>(egui::Id::unique(GUIDE_PRESS));
        press
    });
    let Some(p) = g.at else { return true };
    if let (Some((r, pi, gi)), Some(press)) = (g.from, press)
        && (p - press.origin).length() <= GUIDE_CLICK_SLOP
    {
        // A click: the guide is selected (at the press); Shift adds it or removes it.
        if press.shift {
            let _ = app.run("guide.select", json!({"spread": r, "page": pi, "index": gi, "toggle": true}));
        }
        return true;
    }
    // Dragging one of several selected guides moves (or deletes) them all.
    let group = g.from.is_some_and(|(spread, page, index)| drags_group(app, GuideSel { spread, page, index }));
    let on_canvas = rect.contains(p);
    // Placed where the preview showed it (snapped); the page under the pointer owns a new guide.
    let target = guide_spot(app, xf, &g).and_then(|spot| {
        let spread = app.session.active()?.doc.spread(spot.spread)?;
        let on_page = spread.pages.iter().position(|pg| pg.bounds().contains(spot.pointer));
        Some((spot.spread, spot.sp, on_page))
    });
    let pos_of = |sp: Point| match g.orientation {
        designcraft_doc::Orientation::Horizontal => sp.y,
        designcraft_doc::Orientation::Vertical => sp.x,
    };
    let orient = match g.orientation {
        designcraft_doc::Orientation::Horizontal => "horizontal",
        designcraft_doc::Orientation::Vertical => "vertical",
    };
    match (g.from, on_canvas, target) {
        // Dropped back on a ruler: delete.
        (Some(_), false, _) if group => {
            let _ = app.run("guide.delete", json!({}));
        }
        (Some((r, pi, gi)), false, _) => {
            let _ = app.run("guide.delete", json!({"spread": r, "page": pi, "index": gi}));
        }
        (Some((r, _, _)), true, Some((tr, _, _))) if tr == r && group => {
            if let Some((dx, dy)) = press.and_then(|press| guide_drag_delta(app, xf, &g, press.origin)) {
                let _ = app.run("guide.moveBy", json!({"dx": dx, "dy": dy}));
            }
        }
        (Some((r, pi, gi)), true, Some((tr, sp, _))) if tr == r => {
            let _ = app.run("guide.move", json!({"spread": r, "page": pi, "index": gi, "position": pos_of(sp)}));
        }
        (None, true, Some((r, sp, on_page))) => {
            // The Page tool draws liquid guides from the rulers.
            let liquid = app.session.tool_id() == "page" && on_page.is_some();
            let mut params =
                json!({"spread": r, "orientation": orient, "position": pos_of(sp), "at": sp.x, "spreadGuide": on_page.is_none(), "liquid": liquid});
            if let Some(pi) = on_page {
                params["page"] = json!(pi);
            }
            let _ = app.run("guide.add", params);
        }
        _ => {}
    }
    true
}

/// Whether dragging guide `from` takes the other selected guides along: it is one of several
/// selected.
fn drags_group(app: &DesignApp, from: GuideSel) -> bool {
    app.session.active().is_some_and(|st| st.selection.guides.len() > 1 && st.selection.guides.contains(&from))
}

/// How far a drag of the existing guide `g.from`, pressed at `origin` (screen), moves guides, in
/// spread coordinates: the dragged guide goes to its [`guide_spot`], the other selected guides as
/// far along their own axis as it moved. `None` while the pointer is not over the guide's own
/// spread, where a drop leaves it in place.
fn guide_drag_delta(app: &DesignApp, xf: &Xf, g: &GuideDrag, origin: Pos2) -> Option<(f64, f64)> {
    let (r, pi, gi) = g.from?;
    let spot = guide_spot(app, xf, g)?;
    if spot.spread != r {
        return None;
    }
    let sp = spot.sp;
    let st = app.session.active()?;
    let layout = CanvasLayout::new(&st.doc, st.editing_parents);
    let start = layout.slot(r)?.to_spread(xf.to_canvas(origin));
    let at = st.doc.spread(r)?.pages.get(pi)?.guides.get(gi)?.position;
    let (dx, dy) = match g.orientation {
        designcraft_doc::Orientation::Vertical => (sp.x - at, sp.y - start.y),
        designcraft_doc::Orientation::Horizontal => (sp.x - start.x, sp.y - at),
    };
    (dx.is_finite() && dy.is_finite()).then_some((dx, dy))
}

/// Guides being dragged on the canvas and how far they have moved (spread coordinates).
struct MovingGuides {
    guides: std::collections::HashSet<GuideSel>,
    dx: f64,
    dy: f64,
}

/// What a drag of an existing guide moves, for the preview: the dragged guide, or every selected
/// guide when it is one of several selected. `None` before the drag leaves the click distance,
/// over a ruler (a drop there deletes) and off the guide's spread.
fn moving_guides(app: &DesignApp, xf: &Xf, rect: Rect, g: &GuideDrag, press: GuidePress) -> Option<MovingGuides> {
    let (spread, page, index) = g.from?;
    let p = g.at?;
    if !rect.contains(p) || (p - press.origin).length() <= GUIDE_CLICK_SLOP {
        return None;
    }
    let (dx, dy) = guide_drag_delta(app, xf, g, press.origin)?;
    let from = GuideSel { spread, page, index };
    let guides = if drags_group(app, from) { app.session.active()?.selection.guides.iter().copied().collect() } else { [from].into_iter().collect() };
    Some(MovingGuides { guides, dx, dy })
}

/// An object is under screen point `p` where the Selection tool would pick it (a document
/// spread's, or a parent spread's while editing parents): it takes the press before the guides
/// behind it.
fn over_item(app: &DesignApp, xf: &Xf, p: Pos2) -> bool {
    let Some(st) = app.session.active() else { return false };
    let layout = CanvasLayout::new(&st.doc, st.editing_parents);
    let cx = designcraft_tools::ToolContext {
        doc: &st.doc,
        selection: &st.selection,
        cache: &app.session.cache,
        layout: &layout,
        zoom: xf.zoom.max(1e-6),
        layer: st.active_layer,
        snap: app.ui.snap_view(),
        unit: app.ui.units,
        stroke_in_front: false,
    };
    cx.hit(xf.to_canvas(p)).is_some()
}

const COLUMN_DRAG: &str = "canvas_column_drag";

/// A column guide being dragged with a Selection tool: its gutter follows the pointer.
#[derive(Clone, Debug)]
struct ColumnDrag {
    /// The document the drag's interaction belongs to.
    doc: u64,
    spread: SpreadRef,
    page: usize,
    gutter: usize,
    /// The pointer's spread x minus the gutter's left edge at the press: the gutter keeps that
    /// distance from the pointer.
    grab: f64,
    /// The left edge last previewed.
    last: Option<f64>,
}

/// The interior column guide under screen point `p` (within 3 px) that a press there would drag,
/// as (spread, page, gutter, the gutter's left edge in spread x): a Selection tool, guides shown
/// (not in Preview or Presentation), column guides unlocked, and no object or ruler guide that
/// takes the press there.
fn column_guide_at(app: &DesignApp, xf: &Xf, p: Pos2) -> Option<(SpreadRef, usize, usize, f64)> {
    let preview = matches!(app.ui.screen_mode, ScreenMode::Preview | ScreenMode::Presentation);
    if !app.ui.guides || preview || !matches!(app.session.tool_id(), "selection" | "directSelection") {
        return None;
    }
    let st = app.session.active()?;
    if st.doc.settings.column_guides_locked {
        return None;
    }
    let layout = CanvasLayout::new(&st.doc, st.editing_parents);
    let (r, sp) = layout.spread_at(xf.to_canvas(p))?;
    let tol = 3.0 / xf.zoom.max(1e-9);
    let hit = st.doc.spread(r)?.pages.iter().enumerate().find_map(|(pi, pg)| {
        let m = pg.margin_rect();
        if sp.y < m.y0 - tol || sp.y > m.y1 + tol {
            return None;
        }
        pg.column_rects().windows(2).enumerate().find_map(|(gi, w)| match w {
            // Columns that cannot move (gutters wider than the margin box) offer no guide.
            [a, b]
                if ((sp.x - a.x1).abs() <= tol || (sp.x - b.x0).abs() <= tol)
                    && pg.columns.with_gutter_moved(m.width(), gi, a.x1 - m.x0).is_some() =>
            {
                Some((r, pi, gi, a.x1))
            }
            _ => None,
        })
    })?;
    // A ruler guide there takes the press first (see `guide_drag`).
    let ruler_guide = !app.ui.guides_locked && guide_at(app, xf, p).is_some();
    (!ruler_guide && !over_item(app, xf, p)).then_some(hit)
}

/// Whether spread `r`'s x axis runs across the screen (the spread view is not turned a quarter).
fn spread_x_across(app: &DesignApp, r: SpreadRef) -> bool {
    app.session.active().is_none_or(|st| {
        let [a, b, ..] = CanvasLayout::new(&st.doc, st.editing_parents).xf(r).as_coeffs();
        a.abs() >= b.abs()
    })
}

/// Canvas x `x` moved to the nearest tick mark of the horizontal ruler (its finest increment).
fn ruler_tick_x(app: &DesignApp, doc: &Document, layout: &CanvasLayout, zoom: f64, x: f64) -> Option<f64> {
    let pieces = ruler_pieces(app, doc, layout, doc.zero_point());
    let &(_, _, o) = pieces.iter().find(|(from, to, _)| *from <= x && x < *to).or(pieces.first())?;
    let (major, sub) = doc.settings.horizontal_units.ruler_ticks(zoom);
    let step = major / f64::from(sub.max(1));
    (step.is_finite() && step > 0.0).then(|| o.x + ((x - o.x) / step).round() * step).filter(|v| v.is_finite())
}

/// Where a dragged gutter's left edge goes for spread x `want`: Shift snaps it to the ruler's tick
/// marks; otherwise its edges snap to the document grid, guides and objects as those snaps are
/// set. Measured against the document as it was when the drag began.
fn snap_gutter(app: &DesignApp, g: &ColumnDrag, want: f64, shift: bool, zoom: f64) -> f64 {
    let Some(st) = app.session.active() else { return want };
    let doc = st.interaction.as_ref().map_or_else(|| st.doc.clone(), |i| i.doc.clone());
    let layout = CanvasLayout::new(&doc, st.editing_parents);
    let Some(slot) = layout.slots.iter().find(|s| s.spread == g.spread) else { return want };
    let Some(pg) = doc.spread(g.spread).and_then(|sp| sp.pages.get(g.page)) else { return want };
    let m = pg.margin_rect();
    if shift {
        if !spread_x_across(app, g.spread) {
            return want;
        }
        let c = slot.to_canvas(Point::new(want, m.y0));
        return ruler_tick_x(app, &doc, &layout, zoom, c.x).map_or(want, |x| slot.to_spread(Point::new(x, c.y)).x);
    }
    let cols = pg.column_rects();
    let width = match (cols.get(g.gutter), cols.get(g.gutter + 1)) {
        (Some(a), Some(b)) => (b.x0 - a.x1).max(0.0),
        _ => 0.0,
    };
    let cx = designcraft_tools::ToolContext {
        doc: &doc,
        selection: &st.selection,
        cache: &app.session.cache,
        layout: &layout,
        zoom: zoom.max(1e-6),
        layer: st.active_layer,
        // A line has no size or spacing to match.
        snap: designcraft_tools::SnapView { smart_dimensions: false, smart_spacing: false, ..app.ui.snap_view() },
        unit: app.ui.units,
        stroke_in_front: false,
    };
    let rect = DRect::new(want, m.y0, want + width, m.y1);
    let s = designcraft_tools::snap::snap(
        &cx,
        designcraft_tools::SnapRequest {
            spread: g.spread,
            gesture: designcraft_tools::Gesture::Move,
            rect,
            x_edges: [true, false, true],
            y_edges: [false; 3],
            exclude: &[],
            exclude_guides: &[],
            copying: false,
            lengths: [None, None],
            angle: None,
            radius: 0.0,
            pointer: Point::new(want, m.center().y),
        },
    );
    let x = want + s.delta.x;
    if x.is_finite() { x } else { want }
}

/// Column guides: with a Selection tool, drag an interior column guide (not over an object) to
/// move its gutter, both guides together; Escape puts it back. One undo step per drag. Returns
/// true while it owns the pointer.
fn column_guide_drag(app: &mut DesignApp, ui: &mut egui::Ui, resp: &egui::Response, rect: Rect, xf: &Xf) -> bool {
    let id = egui::Id::unique((COLUMN_DRAG, app.pane));
    let (pressed, released, origin, latest, shift, escape) = ui.input(|i| {
        (
            i.pointer.primary_pressed(),
            // A release this frame, or one missed while something else had the pointer.
            i.pointer.primary_released() || !i.pointer.primary_down(),
            i.pointer.press_origin(),
            i.pointer.latest_pos(),
            i.modifiers.shift,
            i.key_pressed(egui::Key::Escape),
        )
    });
    let mut drag: Option<ColumnDrag> = ui.data(|d| d.get_temp(id));
    if drag.is_none()
        && pressed
        && resp.hovered()
        && let Some(o) = origin.filter(|o| rect.contains(*o))
        && let Some((spread, page, gutter, left)) = column_guide_at(app, xf, o)
        // Another gesture or a dialog's preview owns the document until it ends.
        && let Some((doc, slot_x)) = app
            .session
            .active()
            .filter(|st| st.interaction.is_none())
            .map(|st| (st.uid, CanvasLayout::new(&st.doc, st.editing_parents).to_spread(spread, xf.to_canvas(o)).x))
        && app.session.begin_interaction("Move Column Guide").is_ok()
    {
        drag = Some(ColumnDrag { doc, spread, page, gutter, grab: slot_x - left, last: None });
    }
    let Some(mut g) = drag else { return false };
    // Another document became active: the drag ends in its own, as it began.
    if app.session.active().is_none_or(|st| st.uid != g.doc) {
        app.session.cancel_interaction_of(g.doc);
        ui.data_mut(|d| d.remove::<ColumnDrag>(id));
        return false;
    }
    if escape {
        app.session.cancel_interaction();
        ui.data_mut(|d| d.remove::<ColumnDrag>(id));
        return true;
    }
    if let Some(p) = latest
        && let Some(x) = app.session.active().map(|st| CanvasLayout::new(&st.doc, st.editing_parents).to_spread(g.spread, xf.to_canvas(p)).x)
        && x.is_finite()
    {
        let want = snap_gutter(app, &g, x - g.grab, shift, xf.zoom);
        if g.last != Some(want) && (g.last.is_some() || (p - origin.unwrap_or(p)).length() > 0.0) {
            let params = json!({"spread": g.spread, "page": g.page, "gutter": g.gutter, "position": want});
            if let Err(e) = app.session.preview("layout.moveColumnGuide", params) {
                app.status(e.to_string());
            }
            g.last = Some(want);
        }
    }
    if released {
        ui.data_mut(|d| d.remove::<ColumnDrag>(id));
        if let Err(e) = app.session.commit_interaction() {
            app.status(e.to_string());
        }
        app.after_engine();
        return true;
    }
    ui.data_mut(|d| d.insert_temp(id, g));
    true
}

/// Presses in a row on the canvas: the last press's count, and where and when it was released
/// (`at` is −∞ while the button is down).
#[derive(Clone, Copy, Debug)]
struct Clicks {
    at: f64,
    pos: Pos2,
    n: u8,
}

/// The count of a press at `pos` at time `now`: one more than the last press when it comes within
/// the double-click delay of the last release and near where that release was, else 1.
fn press_count(last: Option<Clicks>, pos: Pos2, now: f64, delay: f64, dist: f32) -> u8 {
    match last {
        Some(c) if now - c.at <= delay && (pos - c.pos).length() <= dist => c.n.saturating_add(1),
        _ => 1,
    }
}

/// The record a release at `pos` leaves: a release away from its press ended a drag, which is
/// not a click, so the next press counts from one.
fn release_clicks(press: Clicks, pos: Pos2, now: f64, dist: f32) -> Clicks {
    let n = if (pos - press.pos).length() <= dist { press.n } else { 0 };
    Clicks { at: now, pos, n }
}

#[cfg(test)]
#[path = "parent_canvas_tests.rs"]
mod parent_canvas_tests;

#[cfg(test)]
#[path = "guide_selection_tests.rs"]
mod guide_selection_tests;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod pencil_preview_tests {
    use egui::{Pos2, pos2, vec2};

    /// Lines painted in `color` (egui paths) and the points they pass through.
    fn lines_in(h: &egui_kittest::Harness<'static, crate::test_window::Window>, color: egui::Color32) -> Vec<Vec<Pos2>> {
        fn walk(s: &egui::Shape, color: egui::Color32, out: &mut Vec<Vec<Pos2>>) {
            match s {
                egui::Shape::Path(p) if p.stroke.color == egui::epaint::ColorMode::Solid(color) && p.points.len() >= 2 => out.push(p.points.clone()),
                egui::Shape::Vec(v) => v.iter().for_each(|s| walk(s, color, out)),
                _ => {}
            }
        }
        let mut out = vec![];
        h.output().shapes.iter().for_each(|s| walk(&s.shape, color, &mut out));
        out
    }

    #[test]
    fn the_pencil_stroke_shows_while_drawing() {
        let mut app = crate::DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", serde_json::json!({})).unwrap();
        app.run("tool.select", serde_json::json!({"tool": "pencil"})).unwrap();
        let mut h = crate::test_window::open(app, vec2(1400.0, 900.0));
        let doc = &h.state().app.session.doc().unwrap().doc;
        let layer = doc.layers[0].color;
        let color = egui::Color32::from_rgb(layer[0], layer[1], layer[2]);
        let items = |h: &egui_kittest::Harness<'static, crate::test_window::Window>| h.state().app.session.doc().unwrap().doc.spreads[0].items.len();
        let before = items(&h);
        // Press on the page and drag, without letting go.
        let start = pos2(520.0, 380.0);
        h.hover_at(start);
        h.drag_at(start);
        h.run_steps(2);
        let path: Vec<Pos2> = (1..=12).map(|i| start + vec2(i as f32 * 12.0, (i as f32 * 0.6).sin() * 30.0)).collect();
        for p in &path {
            h.hover_at(*p);
            h.run_steps(1);
        }
        let drawn = lines_in(&h, color);
        let stroke = drawn.iter().max_by_key(|l| l.len()).expect("the stroke is painted while the button is down");
        let near = |a: Pos2, b: Pos2| (a - b).length() < 2.0;
        assert!(near(stroke[0], start), "it starts where the press was: {:?}", stroke[0]);
        assert!(path.iter().all(|p| stroke.iter().any(|q| near(*p, *q))), "it follows the pointer: {stroke:?}");
        assert_eq!(items(&h), before, "nothing is made before the release");
        // Letting go makes the path and ends the preview.
        h.drop_at(*path.last().unwrap());
        h.run_steps(3);
        assert_eq!(items(&h), before + 1);
        let app = &mut h.state_mut().app;
        let view = app.view_info();
        assert!(app.session.overlays(view).is_empty(), "the preview is gone");
    }
}

#[cfg(test)]
mod scrollbar_tests {
    use egui::{Event, Modifiers, PointerButton};
    use egui_kittest::Harness;

    use super::*;
    use crate::test_window::{Window, open, wheel};

    fn window() -> Harness<'static, Window> {
        let mut h = open(DesignApp::new(designcraft_engine::Session::new(), crate::Services::default()), vec2(1440.0, 900.0));
        // Zoomed in so both axes scroll.
        set_zoom(&mut h.state_mut().app, 2.0);
        h.run_steps(3);
        h
    }

    fn origin(h: &Harness<'static, Window>) -> Point {
        h.state().app.view().map(|v| v.origin).unwrap_or_default()
    }

    /// The scrollbar's track and thumb as the canvas lays them out for the current view, and
    /// the content pixels per thumb pixel.
    fn bar(h: &Harness<'static, Window>, vertical: bool) -> (Rect, Rect, f32) {
        let app = &h.state().app;
        let rect = app.canvas_rect.unwrap();
        let st = app.session.active().unwrap();
        let xf = Xf::new(rect, app.view().unwrap());
        let cb = xf.rect(CanvasLayout::new(&st.doc, st.editing_parents).pasteboard);
        let outer = Rect::from_min_max(rect.min, rect.max + vec2(SCROLLBAR, SCROLLBAR));
        scrollbar_rects(outer, rect, cb, vertical)
    }

    /// A primary-button drag from `a` to `b` in a few moves, with `mods` held throughout.
    fn drag(h: &mut Harness<'static, Window>, a: Pos2, b: Pos2, mods: Modifiers) {
        h.event(Event::ModifiersChanged(mods));
        h.hover_at(a);
        h.step();
        h.event(Event::PointerButton { pos: a, button: PointerButton::Primary, pressed: true, modifiers: mods });
        h.step();
        for n in 1..=4 {
            h.hover_at(a + (b - a) * n as f32 / 4.0);
            h.step();
        }
        h.event(Event::PointerButton { pos: b, button: PointerButton::Primary, pressed: false, modifiers: mods });
        h.event(Event::ModifiersChanged(Modifiers::NONE));
        h.run_steps(3);
    }

    /// Scrollbars sit immediately outside the view, have non-full-size thumbs and use a positive content scale.
    #[test]
    fn document_views_have_scrollbars_beside_the_view() {
        let h = window();
        let rect = h.state().app.canvas_rect.unwrap();
        for vertical in [true, false] {
            let r = h.ctx.read_response(egui::Id::unique(("canvas_scrollbar", 0u8, vertical))).expect("scrollbar").rect;
            let (track, thumb, k) = bar(&h, vertical);
            assert_eq!(r, track);
            assert!(k > 0.0, "the zoomed-in view scrolls");
            if vertical {
                assert_eq!((r.min.x, r.width(), r.min.y, r.max.y), (rect.max.x, SCROLLBAR, rect.min.y, rect.max.y));
                assert!(thumb.height() < track.height());
            } else {
                assert_eq!((r.min.y, r.height(), r.min.x, r.max.x), (rect.max.y, SCROLLBAR, rect.min.x, rect.max.x));
                assert!(thumb.width() < track.width());
            }
        }
    }

    #[test]
    fn dragging_a_thumb_pans_and_clicking_the_track_pages() {
        let mut h = window();
        let zoom = h.state().app.view().unwrap().zoom;
        // Vertical thumb: down 40 px moves the view down by 40 thumb pixels' worth of content.
        let (_, thumb, k) = bar(&h, true);
        let before = origin(&h);
        drag(&mut h, thumb.center(), thumb.center() + vec2(0.0, 40.0), Modifiers::NONE);
        let after = origin(&h);
        let want = 40.0 * k as f64 / zoom;
        assert!((after.y - before.y - want).abs() < want * 0.2, "{before:?} → {after:?}, want +{want}");
        assert_eq!(after.x, before.x);
        // Horizontal thumb: left 30 px.
        let (_, thumb, k) = bar(&h, false);
        drag(&mut h, thumb.center(), thumb.center() - vec2(30.0, 0.0), Modifiers::NONE);
        let next = origin(&h);
        let want = 30.0 * k as f64 / zoom;
        assert!((after.x - next.x - want).abs() < want * 0.2, "{after:?} → {next:?}, want -{want}");
        assert_eq!(next.y, after.y);
        // A click on the vertical track above the thumb pages up by one view.
        let (track, thumb, _) = bar(&h, true);
        assert!(thumb.min.y - track.min.y > 4.0);
        crate::test_window::click_at(&mut h, pos2(track.center().x, track.min.y + 2.0));
        let paged = origin(&h);
        let view = h.state().app.canvas_rect.unwrap().height() as f64 / zoom;
        assert!((next.y - paged.y - view).abs() < 0.01, "{next:?} → {paged:?}, want -{view}");
    }

    /// Wheel scrolling moves both thumbs and keeps the painted scrollbar rect in sync.
    #[test]
    fn scrolling_the_view_moves_the_thumbs() {
        let mut h = window();
        let (_, v0, _) = bar(&h, true);
        let (_, h0, _) = bar(&h, false);
        let c = h.state().app.canvas_rect.unwrap().center();
        wheel(&mut h, c, vec2(-50.0, -80.0));
        let (_, v1, _) = bar(&h, true);
        let (_, h1, _) = bar(&h, false);
        assert!(v1.min.y > v0.min.y && h1.min.x > h0.min.x, "{v0:?} → {v1:?}, {h0:?} → {h1:?}");
        let r = h.ctx.read_response(egui::Id::unique(("canvas_scrollbar", 0u8, true))).expect("scrollbar");
        assert_eq!(r.rect, bar(&h, true).0);
    }

    #[test]
    fn alt_drag_while_editing_text_pans_and_keeps_the_caret() {
        let mut h = window();
        let app = &mut h.state_mut().app;
        let rect = app.canvas_rect.unwrap();
        let xf = Xf::new(rect, app.view().unwrap());
        // A text frame filling the visible part of the page, with the caret in it.
        let (a, b) = (xf.to_canvas(rect.min + vec2(20.0, 20.0)), xf.to_canvas(rect.max - vec2(20.0, 20.0)));
        let r = app.run("frame.create", json!({"rect": [a.x, a.y, b.x, b.y], "content": "text", "text": "Hello brave world"})).unwrap();
        let sid = r["story"].as_u64().unwrap();
        app.run("tool.select", json!({"tool": "type"})).unwrap();
        app.run("text.select", json!({"story": sid, "anchor": 3, "focus": 3})).unwrap();
        assert!(app.session.wants_text());
        h.run_steps(2);
        let caret = h.state().app.session.doc().unwrap().selection.text;
        let before = origin(&h);
        let zoom = h.state().app.view().unwrap().zoom;
        let from = rect.center();
        drag(&mut h, from, from + vec2(60.0, 40.0), Modifiers::ALT);
        let after = origin(&h);
        assert!(after.x < before.x && after.y < before.y, "{before:?} → {after:?}");
        assert!((before.x - after.x - 60.0 / zoom).abs() < 15.0 / zoom, "{before:?} → {after:?}");
        assert_eq!(h.state().app.session.doc().unwrap().selection.text, caret);
        assert!(h.state_mut().app.session.wants_text(), "still editing text");
        // Without Alt the same drag selects text and leaves the view alone.
        drag(&mut h, from, from + vec2(60.0, 40.0), Modifiers::NONE);
        assert_eq!(origin(&h), after);
    }
}
