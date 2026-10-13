//! Modal windows (dialogs, About) that the user moves by dragging their title row.
//!
//! The modal stays anchored at the centre of the window and is shifted by an offset kept in
//! egui's temporary memory. A modal that was not shown in the previous pass starts centred.
//! egui keeps the whole modal inside the window, so its title row stays reachable.

use egui::{Align2, Context, CursorIcon, Id, Modal, Rect, Sense, Ui, Vec2, vec2};

#[derive(Clone, Copy, Default)]
struct Placement {
    offset: Vec2,
    /// The pass the modal was last shown in: a gap means it was closed and opened again.
    pass: u64,
}

/// A modal centred in the window and shifted by the user's drag.
///
/// `area` is the modal's area id. `instance` names what it shows (for example the dialog kind),
/// so a different dialog in the same host starts centred.
pub(crate) fn modal(ctx: &Context, area: Id, instance: Id) -> Modal {
    let pass = ctx.cumulative_pass_nr();
    let size = ctx.memory(|m| m.area_rect(area)).map(|r| r.size());
    let content = ctx.content_rect();
    let offset = ctx.data_mut(|d| {
        let p = d.get_temp_mut_or_default::<Placement>(instance);
        if p.pass.saturating_add(1) < pass {
            *p = Placement::default();
        }
        p.pass = pass;
        if let Some(size) = size {
            p.offset = clamp(p.offset, size, content);
        }
        p.offset
    });
    Modal::new(area).area(Modal::default_area(area).anchor(Align2::CENTER_CENTER, offset))
}

/// Makes `rect`, the modal's title row, drag the modal opened with [`modal`] for `instance`.
///
/// Widgets registered after this call sit on top of the drag area and keep their own input.
pub(crate) fn title_drag(ui: &Ui, instance: Id, rect: Rect) {
    let resp = ui.interact(rect, instance.with("title_drag"), Sense::drag());
    if resp.dragged() {
        let delta = resp.drag_delta();
        ui.ctx().data_mut(|d| d.get_temp_mut_or_default::<Placement>(instance).offset += delta);
        ui.ctx().set_cursor_icon(CursorIcon::Grabbing);
    } else if resp.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::Grab);
    }
}

/// Limits `offset` to the range in which a modal of `size` fits inside `content`, the range egui
/// constrains it to, so dragging back from beyond an edge moves it at once.
fn clamp(offset: Vec2, size: Vec2, content: Rect) -> Vec2 {
    let room = ((content.size() - size) * 0.5).max(Vec2::ZERO);
    vec2(offset.x.min(room.x).max(-room.x), offset.y.min(room.y).max(-room.y))
}
