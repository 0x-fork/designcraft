//! Window chrome: application bar (with menus), Control panel, document tabs, status bar, start screen.

use designcraft_doc::{Align, Content};
use designcraft_geom::Unit;
use egui::{Color32, Sense, Stroke, vec2};
use serde_json::json;

use crate::panels;
use crate::theme::{Tokens, semibold};
use crate::widgets::{NumField, caption, measure};
use crate::{DesignApp, icons};

/// Height of the application bar (InDesign 2026), then a 7 pt lower band and a 1 pt border. The
/// band covers the panel's 1 pt separator line, so the frame's bottom margin is 1 pt short of it.
const APP_BAR: f32 = 36.0;
const APP_BAR_BAND: f32 = 7.0;
const SEPARATOR: f32 = 1.0;
/// Height of the application bar on macOS, where it is the window's title bar (#333). AppKit draws
/// the traffic lights itself, centred in the standard 28 pt title-bar band at the top of the
/// window; moving them would take AppKit calls (`unsafe`, which the workspace forbids). So the bar
/// is that band: the lights sit vertically centred in it, with the bar's controls beside them.
pub const MAC_APP_BAR: f32 = 28.0;
/// The traffic lights as AppKit lays them out: button diameter, centre-to-centre pitch and the
/// first button's left edge (8–10 pt depending on the macOS version; the larger value keeps the
/// gap after them from shrinking).
const MAC_LIGHT: f32 = 12.0;
const MAC_LIGHT_PITCH: f32 = 20.0;
const MAC_LIGHTS_LEFT: f32 = 10.0;
/// Where the bar's content starts to the right of the traffic lights: the end of the three lights
/// plus the same gap the lights keep from the top of the window.
pub const MAC_CONTENT_LEFT: f32 = MAC_LIGHTS_LEFT + 2.0 * MAC_LIGHT_PITCH + MAC_LIGHT + (MAC_APP_BAR - MAC_LIGHT) / 2.0;

fn search_id() -> egui::Id {
    egui::Id::unique("app_bar_search")
}

/// Whether the application bar's search field had the keyboard last frame. Escape takes egui's
/// focus away before any widget runs, so that press must not reach the global shortcuts.
pub fn search_had_focus(ctx: &egui::Context) -> bool {
    ctx.memory(|m| m.had_focus_last_frame(search_id()))
}

/// The search field's state between frames: the query, the highlighted match, whether the matches
/// are shown, and the widget that had the keyboard before the field took it.
#[derive(Clone, Default)]
struct SearchState {
    query: String,
    highlight: usize,
    open: bool,
    previous_focus: Option<egui::Id>,
}

/// The application bar's search field: finds menu commands by their labels in the interface
/// language. ↑/↓ move through the matches; Return or a click runs one as its menu item would.
/// Running one or pressing Escape clears the field and gives the keyboard back to the widget that
/// had it. While the field has the keyboard, no key reaches the shortcuts or the document.
fn search_field(app: &mut DesignApp, ui: &mut egui::Ui) {
    let t = Tokens::get(ui.ctx());
    let id = search_id();
    let state_id = id.with("state");
    let mut st: SearchState = ui.data(|d| d.get_temp(state_id)).unwrap_or_default();
    let (r, _) = ui.allocate_exact_size(vec2(125.0, 18.0), Sense::hover());
    ui.painter().rect(r, 1.0, t.input, Stroke::new(1.0, t.field_border), egui::StrokeKind::Inside);
    icons::paint(ui.painter(), egui::Rect::from_min_size(r.min + vec2(3.0, 2.0), vec2(14.0, 14.0)), "search", t.icon);
    let focused = ui.memory(|m| m.has_focus(id));
    let had_focus = search_had_focus(ui.ctx());
    if !focused && !had_focus {
        st.previous_focus = ui.memory(|m| m.focused());
    }
    let none = egui::Modifiers::NONE;
    let (down, up, enter, escape) = if focused || had_focus {
        ui.input_mut(|i| {
            (
                i.consume_key(none, egui::Key::ArrowDown),
                i.consume_key(none, egui::Key::ArrowUp),
                i.consume_key(none, egui::Key::Enter),
                i.consume_key(none, egui::Key::Escape),
            )
        })
    } else {
        (false, false, false, false)
    };
    let lang = app.ui.language.clone();
    let hint = crate::rtl::widget(ui, egui::RichText::new(crate::i18n::tr(&lang, "Search")).color(t.text_dim));
    let field = egui::Rect::from_min_max(r.min + vec2(19.0, 0.0), r.max - vec2(3.0, 0.0));
    let resp = ui
        .put(
            field,
            egui::TextEdit::singleline(&mut st.query)
                .id(id)
                .frame(egui::Frame::NONE)
                .margin(egui::Margin::ZERO)
                .font(egui::FontId::proportional(11.0))
                .text_color(t.text)
                .hint_text(hint)
                .desired_width(field.width())
                .align(egui::Align2::LEFT_CENTER)
                // Return runs the highlighted match instead of leaving the field.
                .return_key(None),
        )
        .on_hover_text(crate::i18n::tr(&lang, "Search commands"));
    if resp.changed() {
        st.highlight = 0;
        st.open = true;
    }
    if resp.gained_focus() {
        st.open = true;
    }
    let items = if st.open { crate::menus::command_search(app, &st.query) } else { Vec::new() };
    let last = items.len().saturating_sub(1);
    if down {
        st.highlight = st.highlight.saturating_add(1);
    }
    if up {
        st.highlight = st.highlight.saturating_sub(1);
    }
    st.highlight = st.highlight.min(last);
    let mut run = None;
    if enter && let Some(it) = items.get(st.highlight).filter(|it| crate::menus::menu_enabled(app, &it.id)) {
        run = Some(it.id.clone());
    }
    let mut over_matches = false;
    if st.open && !st.query.trim().is_empty() {
        let shown = egui::Area::new(id.with("matches"))
            .order(egui::Order::Foreground)
            .pivot(egui::Align2::RIGHT_TOP)
            .fixed_pos(r.right_bottom() + vec2(0.0, 2.0))
            .show(ui.ctx(), |ui| {
                egui::Frame::menu(ui.style()).show(ui, |ui| {
                    let size = vec2(300.0, 22.0);
                    ui.set_width(size.x);
                    if items.is_empty() {
                        crate::rtl::label(ui, egui::RichText::new(crate::i18n::tr(&lang, "No matching commands")).color(t.text_dim));
                    }
                    for (k, it) in items.iter().enumerate() {
                        let mut b = egui::Button::new(crate::rtl::widget(ui, it.label.as_str())).frame(false).min_size(size);
                        if let Some(sc) = &it.shortcut {
                            b = b.shortcut_text(crate::menus::shortcut_text(sc));
                        }
                        if k == st.highlight {
                            ui.painter().rect_filled(egui::Rect::from_min_size(ui.cursor().min, size), 2.0, t.hover);
                        }
                        let enabled = crate::menus::menu_enabled(app, &it.id);
                        // The button aligns its label by the layout it's in: left, like the menus.
                        let row = ui.allocate_ui_with_layout(
                            size,
                            egui::Layout::left_to_right(egui::Align::Center).with_main_align(egui::Align::Min),
                            |ui| ui.add_enabled(enabled, b),
                        );
                        if row.inner.clicked() {
                            run = Some(it.id.clone());
                        }
                    }
                });
            });
        over_matches = shown.response.contains_pointer();
    }
    if escape || run.is_some() {
        st.query.clear();
        st.open = false;
        let previous = st.previous_focus.take();
        ui.memory_mut(|m| match previous {
            Some(p) => m.request_focus(p),
            None => m.surrender_focus(id),
        });
    } else if !resp.has_focus() && !over_matches {
        // A click elsewhere or Tab took the keyboard: the matches close, the query stays.
        st.open = false;
    }
    ui.data_mut(|d| d.insert_temp(state_id, st));
    if let Some(cmd) = run {
        crate::menus::activate(app, &cmd, &serde_json::Value::Null);
    }
}

/// Width of the app bar's right-hand slot: what is left of the row, minus a pixel of slack so
/// that rounding at fractional scales (115%) never makes the row a hair wider than the scroll
/// area and shows a needless scroll bar. Never narrower than the controls themselves.
fn tools_slot_width(available: f32, tools_w: f32) -> f32 {
    (available - 1.0).max(tools_w)
}

/// Lay out the application bar, native/custom title handling, menu and tool scrolling, drag region, caption controls and search.
pub fn app_bar(app: &mut DesignApp, ui: &mut egui::Ui) {
    let t = Tokens::get(ui.ctx());
    let mac = app.integrated_titlebar;
    let left = if mac { MAC_CONTENT_LEFT as i8 } else { 8 };
    let bar = if mac { MAC_APP_BAR } else { APP_BAR };
    // The tallest control keeps a margin above and below it in the shorter macOS bar.
    let home = if mac { 22.0 } else { 24.0 };
    let resp = egui::Panel::top("app_bar")
        .exact_size(bar + APP_BAR_BAND)
        .frame(egui::Frame::NONE.fill(t.app_bar).inner_margin(egui::Margin {
            left,
            right: if app.custom_titlebar { 0 } else { 10 },
            top: 0,
            bottom: (APP_BAR_BAND - SEPARATOR) as i8,
        }))
        .show(ui, |ui| {
            let full = ui.max_rect();
            if mac {
                // The whole title-bar band, margins included; the controls drawn below take
                // their own clicks.
                let band = egui::Rect::from_min_max(
                    egui::pos2(full.min.x - f32::from(left), full.min.y),
                    egui::pos2(full.max.x + 10.0, full.min.y + MAC_APP_BAR),
                );
                let _ = craft_window::title_bar::empty_area(ui, band);
            }
            let mut menus_end = full.min.x;
            let mut tools_start = full.max.x;
            // Windows caption buttons stay at the right edge; the rest of the bar scrolls.
            let caption_w = if app.custom_titlebar { CAPTION_W } else { 0.0 };
            // Undecorated window: the bar moves it. The drag area is the whole bar but the caption
            // buttons, lower band included, registered before the menus and tools so they stay on
            // top of it: any gap drags, and the lower band is always there, however narrow the
            // window (the menus and tools scroll).
            if app.custom_titlebar {
                let drag = egui::Rect::from_min_max(
                    egui::pos2(full.min.x - left as f32, full.min.y),
                    egui::pos2(full.max.x - caption_w, full.min.y + bar + APP_BAR_BAND),
                );
                let response = ui.interact(drag, egui::Id::unique("window_title_drag"), Sense::click_and_drag());
                // Only from an empty part of the bar: the lower band, or the gap between the menus
                // and the tools (as they were last frame). A menu title pressed and dragged doesn't
                // move the window, as with native title bars.
                let (gap0, gap1) = ui
                    .data(|d| (d.get_temp::<f32>(egui::Id::unique("app_bar_menus_end")), d.get_temp::<f32>(egui::Id::unique("app_bar_tools_start"))));
                let empty = |p: egui::Pos2| p.y >= full.min.y + bar || matches!((gap0, gap1), (Some(a), Some(b)) if p.x >= a && p.x <= b);
                let from_empty = ui.input(|i| i.pointer.press_origin().or(i.pointer.interact_pos())).is_some_and(empty);
                if response.double_clicked() && from_empty {
                    let on = !ui.input(|i| i.viewport().maximized.unwrap_or(false));
                    let _ = app.run("window.maximize", json!({"on": on}));
                } else if response.drag_started_by(egui::PointerButton::Primary) && from_empty {
                    let _ = app.run("window.drag", json!({}));
                }
                app.flush_window_commands(ui.ctx());
            }
            if app.custom_titlebar {
                let caption = egui::Rect::from_min_max(egui::pos2(full.max.x - caption_w, full.min.y), full.max);
                ui.scope_builder(egui::UiBuilder::new().max_rect(caption).layout(egui::Layout::right_to_left(egui::Align::Center)), |ui| {
                    caption_buttons(app, ui);
                });
            }
            let scrolled = egui::Rect::from_min_max(full.min, egui::pos2(full.max.x - caption_w, full.max.y));
            ui.scope_builder(egui::UiBuilder::new().max_rect(scrolled), |ui| {
                // The right-hand controls' width, measured last frame: in a window too narrow for them
                // and the menus they follow the menus, and the bar scrolls sideways.
                let tools_w_id = ui.scope_id().with("tools_width");
                let tools_w: f32 = ui.data(|d| d.get_temp(tools_w_id)).unwrap_or(0.0);
                crate::widgets::overflow_scrolling(ui);
                // (Drag-scrolling is touch-only by default, so a mouse drag on the bar moves the window.)
                egui::ScrollArea::horizontal().id_salt("app_bar_scroll").auto_shrink([false, false]).show(ui, |ui| {
                    ui.horizontal_centered(|ui| {
                        if icons::button(ui, "home", home, app.session.active().is_none(), crate::i18n::tr(&app.ui.language, "Home")).clicked() {
                            app.session_home();
                        }
                        ui.add_space(6.0);
                        if !app.native_menu {
                            ui.style_mut().visuals.widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
                            ui.style_mut().visuals.widgets.inactive.bg_stroke = Stroke::NONE;
                            ui.style_mut().visuals.widgets.hovered.bg_stroke = Stroke::NONE;
                            ui.style_mut().spacing.button_padding = vec2(7.0, 3.0);
                            crate::menus::menu_bar(app, ui);
                        }
                        menus_end = ui.min_rect().max.x;
                        // Where the menus end on screen (moves as a narrow bar scrolls).
                        ui.data_mut(|d| d.insert_temp(egui::Id::unique("app_bar_menus_end"), menus_end));
                        let size = vec2(tools_slot_width(ui.available_width(), tools_w), ui.available_height());
                        let tools = ui.allocate_ui_with_layout(size, egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.add_space(4.0);
                            search_field(app, ui);
                            ui.add_space(8.0);
                            let current = app.ui.workspace.clone();
                            let shown = crate::i18n::workspace_name(&app.ui.language, &current);
                            crate::menus::menu_button(
                                ui,
                                crate::rtl::widget(ui, egui::RichText::new(format!("{shown} ▾")).font(semibold(11.5)).color(t.text)),
                                |ui| {
                                    let customs: Vec<String> = app.ui.custom_workspaces.iter().map(|w| w.name.clone()).collect();
                                    for w in crate::menus::WORKSPACES.iter().map(|w| w.to_string()).chain(customs.iter().cloned()) {
                                        if ui
                                            .selectable_label(w == current, crate::rtl::widget(ui, crate::i18n::workspace_name(&app.ui.language, &w)))
                                            .clicked()
                                        {
                                            let _ = app.run("window.workspace", json!({"name": w}));
                                            ui.close();
                                        }
                                    }
                                    ui.separator();
                                    if ui
                                        .button(crate::rtl::widget(
                                            ui,
                                            format!(
                                                "{} {}",
                                                crate::i18n::tr(&app.ui.language, "Reset"),
                                                crate::i18n::workspace_name(&app.ui.language, &current)
                                            ),
                                        ))
                                        .clicked()
                                    {
                                        let _ = app.run("window.resetWorkspace", json!({}));
                                        ui.close();
                                    }
                                    if ui.button(crate::rtl::widget(ui, crate::i18n::tr(&app.ui.language, "New Workspace…"))).clicked() {
                                        let _ = app.run("window.newWorkspace", json!({}));
                                        ui.close();
                                    }
                                    if !customs.is_empty() {
                                        crate::menus::menu_button(
                                            ui,
                                            crate::rtl::widget(ui, crate::i18n::tr(&app.ui.language, "Delete Workspace")),
                                            |ui| {
                                                for w in &customs {
                                                    if ui.button(w).clicked() {
                                                        let _ = app.run("window.deleteWorkspace", json!({"name": w}));
                                                        ui.close();
                                                    }
                                                }
                                            },
                                        );
                                    }
                                },
                            );
                            ui.add_space(6.0);
                            if icons::button(ui, "share", 22.0, false, crate::i18n::tr(&app.ui.language, "Share")).clicked() {
                                app.status(crate::i18n::tr(&app.ui.language, "Export a PDF, IDML or package to share — no cloud account needed."));
                            }
                            ui.add_space(8.0);
                            // Always one click away: the ArtCraft community Discord.
                            if crate::about::discord_button(ui, "Discord", vec2(78.0, 22.0)) {
                                let _ = app.run("help.discord", json!({}));
                            }
                            tools_start = ui.min_rect().min.x;
                            ui.data_mut(|d| d.insert_temp(egui::Id::unique("app_bar_tools_start"), tools_start));
                            ui.min_rect().width()
                        });
                        if (tools.inner - tools_w).abs() > 0.5 {
                            ui.data_mut(|d| d.insert_temp(tools_w_id, tools.inner));
                            ui.ctx().request_discard("app bar: the right-hand controls changed width");
                        }
                    });
                });
            });
            // Centred title.
            let title = match app.session.active() {
                Some(d) => format!("DesignCraft — {}", crate::rtl::isolate(&d.title())),
                None => "DesignCraft".to_string(),
            };
            // Centred between the menus and the right-hand controls; left out when it won't fit.
            let galley = crate::rtl::plain(ui.ctx(), &title, egui::FontId::proportional(11.5), t.text);
            let w = galley.size().x;
            let cx = full.center().x;
            if cx - w / 2.0 > menus_end + 12.0 && cx + w / 2.0 < tools_start - 12.0 {
                ui.painter().galley(egui::pos2(cx - w / 2.0, full.center().y - galley.size().y / 2.0), galley, t.text);
            }
        });
    let r = resp.response.rect;
    ui.painter().rect_filled(egui::Rect::from_min_max(egui::pos2(r.min.x, r.max.y - APP_BAR_BAND), r.max), 0.0, t.pasteboard);
    ui.painter().line_segment([egui::pos2(r.min.x, r.max.y), egui::pos2(r.max.x, r.max.y)], Stroke::new(1.0, t.border));
}

const CAPTION_BUTTON_W: f32 = 46.0;
/// Minimize, maximize and close, and the space after them.
const CAPTION_W: f32 = 3.0 * CAPTION_BUTTON_W + 8.0;

/// Windows caption buttons. The macOS traffic lights remain native.
fn caption_buttons(app: &mut DesignApp, ui: &mut egui::Ui) {
    let t = Tokens::get(ui.ctx());
    let maximized = ui.input(|i| i.viewport().maximized.unwrap_or(false));
    let spacing = ui.spacing().item_spacing.x;
    ui.spacing_mut().item_spacing.x = 0.0;
    for (command, label) in
        [("window.close", "Close"), ("window.maximize", if maximized { "Restore" } else { "Maximize" }), ("window.minimize", "Minimize")]
    {
        let (rect, _) = ui.allocate_exact_size(vec2(CAPTION_BUTTON_W, APP_BAR), Sense::hover());
        let response = ui.interact(rect, egui::Id::unique(command), Sense::click());
        response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, ui.is_enabled(), label));
        let hovered = response.hovered();
        if hovered {
            ui.painter().rect_filled(rect, 0.0, if command == "window.close" { t.caption_close } else { t.hover });
        }
        let color = if hovered && command == "window.close" { t.caption_close_text } else { t.text };
        let stroke = Stroke::new(1.0, color);
        let icon = egui::Rect::from_center_size(rect.center(), vec2(10.0, 10.0));
        match command {
            "window.close" => {
                ui.painter().line_segment([icon.left_top(), icon.right_bottom()], stroke);
                ui.painter().line_segment([icon.right_top(), icon.left_bottom()], stroke);
            }
            "window.maximize" if maximized => {
                let back = icon.translate(vec2(2.0, -2.0));
                ui.painter().line_segment([back.left_top(), back.right_top()], stroke);
                ui.painter().line_segment([back.right_top(), back.right_bottom()], stroke);
                ui.painter().rect_stroke(icon, 0.0, stroke, egui::StrokeKind::Inside);
            }
            "window.maximize" => {
                ui.painter().rect_stroke(icon, 0.0, stroke, egui::StrokeKind::Inside);
            }
            _ => {
                ui.painter().line_segment([icon.left_center(), icon.right_center()], stroke);
            }
        }
        if response.clicked() {
            let params = if command == "window.maximize" { json!({"on": !maximized}) } else { json!({}) };
            let _ = app.run(command, params);
            app.flush_window_commands(ui.ctx());
        }
        response.on_hover_text(crate::i18n::tr(&app.ui.language, label));
    }
    ui.spacing_mut().item_spacing.x = spacing;
    ui.add_space(8.0);
}

/// Undecorated Windows windows need resize grips as well as caption controls.
pub(crate) fn resize_borders(app: &mut DesignApp, ui: &mut egui::Ui) {
    if ui.input(|i| i.viewport().maximized.unwrap_or(false) || i.viewport().fullscreen.unwrap_or(false)) {
        return;
    }
    let r = ui.ctx().content_rect();
    let edge = 4.0;
    let corner = 10.0;
    let rect = |x0, y0, x1, y1| egui::Rect::from_min_max(egui::pos2(x0, y0), egui::pos2(x1, y1));
    for (name, region, cursor) in [
        ("northWest", rect(r.left(), r.top(), r.left() + corner, r.top() + corner), egui::CursorIcon::ResizeNwSe),
        ("northEast", rect(r.right() - corner, r.top(), r.right(), r.top() + corner), egui::CursorIcon::ResizeNeSw),
        ("southWest", rect(r.left(), r.bottom() - corner, r.left() + corner, r.bottom()), egui::CursorIcon::ResizeNeSw),
        ("southEast", rect(r.right() - corner, r.bottom() - corner, r.right(), r.bottom()), egui::CursorIcon::ResizeNwSe),
        ("north", rect(r.left() + corner, r.top(), r.right() - corner, r.top() + edge), egui::CursorIcon::ResizeVertical),
        ("south", rect(r.left() + corner, r.bottom() - edge, r.right() - corner, r.bottom()), egui::CursorIcon::ResizeVertical),
        ("west", rect(r.left(), r.top() + corner, r.left() + edge, r.bottom() - corner), egui::CursorIcon::ResizeHorizontal),
        ("east", rect(r.right() - edge, r.top() + corner, r.right(), r.bottom() - corner), egui::CursorIcon::ResizeHorizontal),
    ] {
        let response = ui.interact(region, egui::Id::unique(("window_resize", name)), Sense::drag()).on_hover_cursor(cursor);
        if response.drag_started_by(egui::PointerButton::Primary) {
            let _ = app.run("window.resize", json!({"edge": name}));
            app.flush_window_commands(ui.ctx());
        }
    }
}

impl DesignApp {
    fn session_home(&mut self) {
        self.ui.status = crate::i18n::tr(&self.ui.language, "Home").into();
    }
}

fn vsep(ui: &mut egui::Ui) {
    let t = Tokens::get(ui.ctx());
    let (r, _) = ui.allocate_exact_size(vec2(9.0, 46.0), Sense::hover());
    ui.painter().line_segment([r.center_top() + vec2(0.0, 4.0), r.center_bottom() - vec2(0.0, 4.0)], Stroke::new(1.0, t.divider));
}

/// Two-row context-sensitive Control panel.
pub fn control_bar(app: &mut DesignApp, ui: &mut egui::Ui) {
    let t = Tokens::get(ui.ctx());
    egui::Panel::top("control_bar")
        .exact_size(59.0)
        .frame(
            egui::Frame::NONE.fill(t.panel).inner_margin(egui::Margin { left: 8, right: 8, top: 5, bottom: 3 }).stroke(Stroke::new(1.0, t.divider)),
        )
        .show(ui, |ui| {
            // Wider than the window: the bar scrolls sideways.
            crate::widgets::overflow_scrolling(ui);
            egui::ScrollArea::horizontal().id_salt("control_bar_scroll").auto_shrink([false, true]).show(ui, |ui| {
                ui.horizontal_top(|ui| {
                    let text_mode =
                        matches!(app.session.tool_id(), "type" | "verticalType") || app.session.active().is_some_and(|d| d.selection.text.is_some());
                    if app.session.tool_id() == "page" {
                        control_page(app, ui);
                    } else if text_mode {
                        control_text(app, ui);
                    } else {
                        crate::object_control::show(app, ui);
                    }
                });
            });
        });
}

/// Page-size preset menu and the portrait / landscape toggles of the Control panel.
fn page_preset_and_orientation(app: &mut DesignApp, ui: &mut egui::Ui, lang: &str, size: (f64, f64), n: usize, anchor: u8) {
    let landscape = size.0 > size.1;
    ui.vertical(|ui| {
        let cur = panels::properties::preset_name(size.0, size.1);
        egui::ComboBox::from_id_salt("ctl_page_preset").selected_text(crate::rtl::widget(ui, crate::i18n::tr(lang, cur))).width(130.0).show_ui(
            ui,
            |ui| {
                for (name, w, h) in panels::properties::PAGE_PRESETS {
                    if ui.selectable_label(cur == *name, crate::rtl::widget(ui, crate::i18n::tr(lang, name))).clicked() {
                        let (w, h) = if landscape { (h, w) } else { (w, h) };
                        let _ = app.run("layout.pageSize", json!({"pages": [n], "width": w, "height": h, "anchor": anchor}));
                    }
                }
            },
        );
        ui.horizontal(|ui| {
            for (icon, tip, want) in [("orient-portrait", "Portrait", false), ("orient-landscape", "Landscape", true)] {
                if crate::widgets::icon_toggle(ui, icon, landscape == want, crate::i18n::tr(lang, tip)).clicked() && landscape != want {
                    let _ = app.run("layout.pageSize", json!({"pages": [n], "width": size.1, "height": size.0, "anchor": anchor}));
                }
            }
        });
    });
}

/// The Control panel with the Page tool, as InDesign's: reference point, X/Y, W/H, page-size
/// preset, orientation, Liquid Page Rule and Show Parent Page Overlay, for
/// the page the Page tool selected (else the page in view). W/H keep the reference point in place.
fn control_page(app: &mut DesignApp, ui: &mut egui::Ui) {
    let lang = app.ui.language.clone();
    let Some(st) = app.session.active() else { return };
    let units = st.doc.settings.horizontal_units;
    let Some(page) = st.selection.page.or_else(|| crate::canvas::current_page(app)) else { return };
    let Some(st) = app.session.active() else { return };
    let d = st.doc.clone();
    let Some((si, pi)) = d.page_loc(page) else { return };
    let pg = d.spreads[si].pages[pi].clone();
    let n = page + 1;
    // Reference point: W/H edits keep it in place.
    let (r, _) = ui.allocate_exact_size(vec2(30.0, 44.0), Sense::hover());
    if let Some(p) = panels::properties::ref_point_proxy(ui, egui::Rect::from_center_size(r.center(), vec2(27.0, 27.0)), app.ui.ref_point) {
        app.ui.ref_point = p;
    }
    let anchor = app.ui.ref_point.min(8);
    let (fx, fy) = ((anchor % 3) as f64 / 2.0, (anchor / 3) as f64 / 2.0);
    // The reference point's position: from the spine across, from the top down.
    let (x, y) = (pg.x - d.spreads[si].spine_x() + fx * pg.width, fy * pg.height);
    egui::Grid::new("ctl_page_xywh").num_columns(4).spacing(vec2(4.0, 4.0)).show(ui, |ui| {
        caption(ui, crate::i18n::tr(&lang, "X:"));
        // Pages don't move within their spread yet.
        ui.add_enabled_ui(false, |ui| measure(ui, "cpx", Some(x), units, 64.0));
        caption(ui, crate::i18n::tr(&lang, "W:"));
        if let Some(v) = measure(ui, "cpw", Some(pg.width), units, 64.0) {
            let _ = app.run("layout.pageSize", json!({"pages": [n], "width": v, "anchor": anchor}));
        }
        ui.end_row();
        caption(ui, crate::i18n::tr(&lang, "Y:"));
        ui.add_enabled_ui(false, |ui| measure(ui, "cpy", Some(y), units, 64.0));
        caption(ui, crate::i18n::tr(&lang, "H:"));
        if let Some(v) = measure(ui, "cph", Some(pg.height), units, 64.0) {
            let _ = app.run("layout.pageSize", json!({"pages": [n], "height": v, "anchor": anchor}));
        }
        ui.end_row();
    });
    vsep(ui);
    // Page-size preset and orientation.
    page_preset_and_orientation(app, ui, &lang, (pg.width, pg.height), n, anchor);
    vsep(ui);
    ui.vertical(|ui| {
        caption(ui, crate::i18n::tr(&lang, "Liquid Page Rule:"));
        let label = panels::interactive::liquid_rule_label(pg.liquid);
        egui::ComboBox::from_id_salt("ctl_page_rule").selected_text(crate::rtl::widget(ui, crate::i18n::tr(&lang, label))).width(150.0).show_ui(
            ui,
            |ui| {
                for (r, l) in panels::interactive::LIQUID_RULES {
                    if ui.selectable_label(pg.liquid == r, crate::rtl::widget(ui, crate::i18n::tr(&lang, l))).clicked() && pg.liquid != r {
                        let _ = app.run("liquid.pageRule", json!({"rule": r, "pages": [n]}));
                    }
                }
            },
        );
    });
    vsep(ui);
    ui.vertical(|ui| {
        let mut overlay = pg.show_parent_overlay;
        if ui.checkbox(&mut overlay, crate::rtl::widget(ui, crate::i18n::tr(&lang, "Show Parent Page Overlay"))).changed() {
            let _ = app.run("layout.pageOptions", json!({"pages": [n], "showParentOverlay": overlay}));
        }
    });
}

/// The Control panel's font sizes (and leadings), as InDesign lists them.
const SIZES: &[f64] = &[6.0, 8.0, 9.0, 10.0, 11.0, 12.0, 14.0, 18.0, 24.0, 30.0, 36.0, 48.0, 60.0, 72.0];
/// The Control panel's tracking values (1/1000 em).
const TRACKING: &[f64] = &[-100.0, -75.0, -50.0, -25.0, -10.0, -5.0, 0.0, 5.0, 10.0, 25.0, 50.0, 75.0, 100.0, 200.0];

/// Whether the text Control panel shows Paragraph Formatting Controls (¶) rather than Character
/// Formatting Controls (A).
const PARA_MODE: &str = "ctl_text_para_mode";

/// An icon and a field after it, as the Control panel lays its fields out.
pub(crate) fn icon_field(ui: &mut egui::Ui, icon: &str, tip: &str, field: NumField) -> Option<f64> {
    let t = Tokens::get(ui.ctx());
    let (r, resp) = ui.allocate_exact_size(vec2(18.0, 18.0), Sense::hover());
    icons::paint(ui.painter(), r, icon, t.icon);
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Label, true, tip));
    let label = resp.id;
    resp.on_hover_text(tip);
    field.spinner().labelled_by(label).show(ui)
}

/// A column of two rows (the Control panel's two-row groups).
fn rows(ui: &mut egui::Ui, mut row: impl FnMut(&mut egui::Ui, usize)) {
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 3.0;
        ui.horizontal(|ui| row(ui, 0));
        ui.horizontal(|ui| row(ui, 1));
    });
}

/// A small toggle button showing `text` (the case and position buttons).
fn text_toggle(ui: &mut egui::Ui, text: &str, strike: bool, on: bool, tip: &str) -> bool {
    let t = Tokens::get(ui.ctx());
    let (r, resp) = ui.allocate_exact_size(vec2(24.0, 20.0), Sense::click());
    if on {
        ui.painter().rect_filled(r, 2.0, t.tool_active);
    } else if resp.hovered() {
        ui.painter().rect_filled(r, 2.0, t.hover);
    }
    let col = if on { t.text_strong } else { t.icon };
    if text == "small-caps" {
        crate::icons::paint(ui.painter(), r, "small-caps", col);
    } else {
        ui.painter().text(r.center(), egui::Align2::CENTER_CENTER, text, egui::FontId::proportional(13.0), col);
    }
    if strike {
        ui.painter().line_segment([r.center() + vec2(-6.0, 0.5), r.center() + vec2(6.0, 0.5)], Stroke::new(1.2, col));
    }
    resp.widget_info(|| egui::WidgetInfo::selected(egui::Role::Button, true, on, tip));
    resp.on_hover_text(tip).clicked()
}

/// The text Control panel, as InDesign's: A shows the Character Formatting Controls (font, size
/// and leading, the case and position buttons, kerning and tracking, the scales, baseline shift
/// and skew, the character style and language); ¶ the Paragraph Formatting Controls (alignment,
/// the indents, space before and after, the drop cap, the paragraph style, Hyphenate and Align to
/// Baseline Grid).
fn control_text(app: &mut DesignApp, ui: &mut egui::Ui) {
    let t = Tokens::get(ui.ctx());
    let lang = app.ui.language.clone();
    let tr = |s: &str| crate::i18n::tr(&lang, s).to_string();
    let attrs = panels::text_attrs(app);
    let chars = attrs.as_ref().map(|a| a["chars"].clone()).unwrap_or_default();
    let para = attrs.as_ref().map(|a| a["para"].clone()).unwrap_or_default();
    let mode_id = egui::Id::unique(PARA_MODE);
    let para_mode = ui.data(|d| d.get_temp::<bool>(mode_id)).unwrap_or(false);
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 3.0;
        for (para, text, tip) in [(false, "A", "Character Formatting Controls"), (true, "¶", "Paragraph Formatting Controls")] {
            let on = para_mode == para;
            let (r, resp) = ui.allocate_exact_size(vec2(24.0, 22.0), Sense::click());
            if on {
                ui.painter().rect_filled(r, 2.0, t.tool_active);
            } else if resp.hovered() {
                ui.painter().rect_filled(r, 2.0, t.hover);
            }
            ui.painter().text(r.center(), egui::Align2::CENTER_CENTER, text, semibold(15.0), if on { t.text_strong } else { t.text_dim });
            let tip = tr(tip);
            resp.widget_info(|| egui::WidgetInfo::selected(egui::Role::Button, true, on, &tip));
            if resp.on_hover_text(&tip).clicked() {
                ui.data_mut(|d| d.insert_temp(mode_id, para));
            }
        }
    });
    vsep(ui);
    if para_mode {
        control_paragraph(app, ui, &attrs, &para, &tr);
    } else {
        control_character(app, ui, &attrs, &chars, &para, &tr);
    }
}

/// Draw the character-mode controls and issue their corresponding text commands.
fn control_character(
    app: &mut DesignApp,
    ui: &mut egui::Ui,
    attrs: &Option<serde_json::Value>,
    chars: &serde_json::Value,
    para: &serde_json::Value,
    tr: &dyn Fn(&str) -> String,
) {
    ui.vertical(|ui| {
        let fam = chars["fontFamily"].as_str().unwrap_or("").to_string();
        let sty = chars["fontStyle"].as_str().unwrap_or("").to_string();
        panels::font_family_picker(app, ui, &fam, 170.0);
        panels::font_style_picker(app, ui, &fam, &sty, 170.0);
    });
    vsep(ui);
    // Size and leading: a spinner on the left and a list of common values on the right;
    // automatic leading in parentheses.
    rows(ui, |ui, row| {
        if row == 0 {
            if let Some(v) = icon_field(
                ui,
                "font-size",
                &tr("Font Size"),
                NumField::number("csize", chars["size"].as_f64(), " pt", 2).width(84.0).presets(SIZES).range(0.1, 1296.0),
            ) {
                let _ = app.run("type.char", json!({"attrs": {"size": v}}));
            }
        } else {
            let auto = chars["leading"]["kind"].as_str() != Some("points");
            let lead = if auto {
                chars["size"].as_f64().map(|s| s * para["autoLeading"].as_f64().unwrap_or(1.2))
            } else {
                chars["leading"]["value"].as_f64()
            };
            let lead = lead.map(|v| (v * 100.0).round() / 100.0);
            let key = if auto { "cleadauto" } else { "clead" };
            if let Some(v) = icon_field(
                ui,
                "leading",
                &tr("Leading"),
                NumField::number(key, lead, " pt", 2).width(84.0).presets(SIZES).range(0.0, 5000.0).parens(auto),
            ) {
                let _ = app.run("type.char", json!({"attrs": {"leading": {"kind": "points", "value": v}}}));
            }
        }
    });
    vsep(ui);
    // All Caps, Superscript, Underline / Small Caps, Subscript, Strikethrough.
    let caps = chars["capitalization"].as_str().unwrap_or("normal").to_string();
    let pos = chars["position"].as_str().unwrap_or("normal").to_string();
    let (ul, st) = (chars["underline"].as_bool().unwrap_or(false), chars["strikethrough"].as_bool().unwrap_or(false));
    let mut set = None;
    rows(ui, |ui, row| {
        if row == 0 {
            if text_toggle(ui, "TT", false, caps == "allCaps", &tr("All Caps")) {
                set = Some(json!({"capitalization": if caps == "allCaps" { "normal" } else { "allCaps" }}));
            }
            if text_toggle(ui, "T¹", false, pos == "superscript", &tr("Superscript")) {
                set = Some(json!({"position": if pos == "superscript" { "normal" } else { "superscript" }}));
            }
            if text_toggle(ui, "T̲", false, ul, &tr("Underline")) {
                set = Some(json!({"underline": !ul}));
            }
        } else {
            if text_toggle(ui, "small-caps", false, caps == "smallCaps", &tr("Small Caps")) {
                set = Some(json!({"capitalization": if caps == "smallCaps" { "normal" } else { "smallCaps" }}));
            }
            if text_toggle(ui, "T₁", false, pos == "subscript", &tr("Subscript")) {
                set = Some(json!({"position": if pos == "subscript" { "normal" } else { "subscript" }}));
            }
            if text_toggle(ui, "T", true, st, &tr("Strikethrough")) {
                set = Some(json!({"strikethrough": !st}));
            }
        }
    });
    if let Some(a) = set {
        let _ = app.run("type.char", json!({"attrs": a}));
    }
    vsep(ui);
    rows(ui, |ui, row| {
        if row == 0 {
            let t = Tokens::get(ui.ctx());
            let (r, resp) = ui.allocate_exact_size(vec2(18.0, 18.0), Sense::hover());
            icons::paint(ui.painter(), r, "kerning", t.icon);
            resp.on_hover_text(tr("Kerning"));
            crate::panels::properties::kerning_control(app, ui, "ckern", &chars["kerning"], 92.0);
        } else {
            if let Some(v) = icon_field(
                ui,
                "tracking",
                &tr("Tracking"),
                NumField::number("ctrack", chars["tracking"].as_f64(), "", 0).width(92.0).presets(TRACKING).step(5.0).range(-1000.0, 10000.0),
            ) {
                let _ = app.run("type.char", json!({"attrs": {"tracking": v}}));
            }
        }
    });
    vsep(ui);
    const SCALES: &[f64] = &[25.0, 50.0, 75.0, 80.0, 90.0, 100.0, 110.0, 120.0, 125.0, 150.0, 200.0];
    let pct = |v: &serde_json::Value| v.as_f64().map(|s| (s * 1000.0).round() / 10.0);
    rows(ui, |ui, row| {
        if row == 0 {
            if let Some(v) = icon_field(
                ui,
                "vscale",
                &tr("Vertical Scale"),
                NumField::number("cvs", pct(&chars["vScale"]), "%", 1).width(84.0).presets(SCALES).range(1.0, 1000.0),
            ) {
                let _ = app.run("type.char", json!({"attrs": {"vScale": v / 100.0}}));
            }
        } else {
            if let Some(v) = icon_field(
                ui,
                "hscale",
                &tr("Horizontal Scale"),
                NumField::number("chs", pct(&chars["hScale"]), "%", 1).width(84.0).presets(SCALES).range(1.0, 1000.0),
            ) {
                let _ = app.run("type.char", json!({"attrs": {"hScale": v / 100.0}}));
            }
        }
    });
    vsep(ui);
    rows(ui, |ui, row| {
        if row == 0 {
            if let Some(v) = icon_field(
                ui,
                "baseline-shift",
                &tr("Baseline Shift"),
                NumField::number("cbs", chars["baselineShift"].as_f64(), " pt", 2).width(76.0).range(-5000.0, 5000.0),
            ) {
                let _ = app.run("type.char", json!({"attrs": {"baselineShift": v}}));
            }
        } else {
            if let Some(v) = icon_field(
                ui,
                "skew",
                &tr("Skew (False Italic)"),
                NumField::number("csk", chars["skew"].as_f64(), "°", 1).width(76.0).range(-85.0, 85.0),
            ) {
                let _ = app.run("type.char", json!({"attrs": {"skew": v}}));
            }
        }
    });
    vsep(ui);
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 3.0;
        // The character style, then the language.
        let cs = attrs.as_ref().and_then(|a| a["characterStyle"].as_str()).unwrap_or(designcraft_doc::NO_CHAR_STYLE).to_string();
        let names: Vec<String> = app.session.active().map(|d| d.doc.styles.character.iter().map(|c| c.name.clone()).collect()).unwrap_or_default();
        let mut pick = None;
        egui::ComboBox::from_id_salt("ctl_char_style")
            .selected_text(crate::rtl::widget(ui, crate::i18n::style_name(&app.ui.language, &cs)))
            .width(150.0)
            .show_ui(ui, |ui| {
                for n in &names {
                    if ui.selectable_label(*n == cs, crate::rtl::widget(ui, crate::i18n::style_name(&app.ui.language, n))).clicked() {
                        pick = Some(n.clone());
                    }
                }
            });
        if let Some(n) = pick {
            let _ = app.run("style.character.apply", json!({"name": n}));
        }
        let lang = chars["language"].as_str().unwrap_or("English: USA").to_string();
        egui::ComboBox::from_id_salt("ctl_language").selected_text(crate::rtl::widget(ui, tr(&lang))).width(150.0).show_ui(ui, |ui| {
            egui::ScrollArea::vertical().max_height(320.0).show(ui, |ui| crate::panels::properties::language_menu(app, ui, &lang));
        });
    });
}

fn control_paragraph(
    app: &mut DesignApp,
    ui: &mut egui::Ui,
    attrs: &Option<serde_json::Value>,
    para: &serde_json::Value,
    tr: &dyn Fn(&str) -> String,
) {
    let cur: Align = serde_json::from_value(para["align"].clone()).unwrap_or_default();
    if let Some(a) = align_buttons(ui, cur, tr) {
        let _ = app.run("type.para", json!({"attrs": {"align": a}}));
    }
    vsep(ui);
    let units = app.session.active().map(|d| d.doc.settings.horizontal_units).unwrap_or(Unit::Picas);
    if let Some((k, v)) = indent_space_fields(ui, para, units, tr) {
        let _ = app.run("type.para", json!({"attrs": {k: v}}));
    }
    if let Some((k, v)) = drop_cap_fields(ui, para, tr) {
        let _ = app.run("type.para", json!({"attrs": {k: v.max(0.0) as u64}}));
    }
    vsep(ui);
    // Bulleted List over Numbered List, as InDesign's (a list with the default hanging indent, or,
    // pressed, none). Pressed when every selected paragraph has it, as Type › Apply Bullets shows.
    use designcraft_doc::ListType;
    let pressed = [ListType::Bullets, ListType::Numbers].map(|k| app.session.selection_has_list(k));
    let mut list = None;
    rows(ui, |ui, row| {
        let (icon, cmd, tip) = if row == 0 { ("bullets", "list.bullets", "Bulleted List") } else { ("numbering", "list.numbers", "Numbered List") };
        let on = pressed.get(row).copied().unwrap_or(false);
        if crate::widgets::icon_toggle(ui, icon, on, &tr(tip)).clicked() {
            list = Some((cmd, !on));
        }
    });
    if let Some((cmd, on)) = list {
        let _ = app.run(cmd, json!({"on": on}));
    }
    vsep(ui);
    let (shade, dialog) = shading_buttons(app, ui, para, tr);
    if let Some(attrs) = shade {
        let _ = app.run("type.para", json!({ "attrs": attrs }));
    }
    if let Some(tab) = dialog {
        let _ = app.run("app.paragraphBordersDialog", json!({ "tab": tab }));
    }
    vsep(ui);
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 3.0;
        let ps = attrs.as_ref().and_then(|a| a["paragraphStyle"].as_str()).unwrap_or("").to_string();
        let ov = attrs.as_ref().and_then(|a| a["paraOverrides"].as_u64()).unwrap_or(0)
            + attrs.as_ref().and_then(|a| a["charOverrides"].as_u64()).unwrap_or(0);
        panels::para_style_picker(app, ui, &ps, ov > 0, 170.0);
        ui.horizontal(|ui| {
            let mut h = para["hyphenate"].as_bool().unwrap_or(true);
            if crate::widgets::checkbox(ui, &mut h, &tr("Hyphenate")).changed() {
                let _ = app.run("type.para", json!({"attrs": {"hyphenate": h}}));
            }
            // First Line Only is aligned too: the button shows it pressed, and a click turns alignment off.
            let grid = matches!(para["gridAlign"].as_str(), Some("allLines" | "firstLineOnly"));
            if crate::widgets::icon_toggle(ui, "grid-baseline", grid, &tr("Align to Baseline Grid")).clicked() {
                let _ = app.run("type.para", json!({"attrs": {"gridAlign": if grid { "none" } else { "allLines" }}}));
            }
        });
    });
}

/// The paragraph alignment buttons, in two rows: the alignment picked this frame, if any.
fn align_buttons(ui: &mut egui::Ui, cur: Align, tr: &dyn Fn(&str) -> String) -> Option<Align> {
    let mut align = None;
    rows(ui, |ui, row| {
        if row == 0 {
            for (a, icon) in [
                (Align::Left, "palign-left"),
                (Align::Center, "palign-center"),
                (Align::Right, "palign-right"),
                (Align::TowardsSpine, "palign-spine-towards"),
            ] {
                if icons::button(ui, icon, 20.0, cur == a, &tr(a.label())).clicked() {
                    align = Some(a);
                }
            }
        } else {
            for (a, icon) in [
                (Align::LeftJustified, "palign-justify-left"),
                (Align::CenterJustified, "palign-justify-center"),
                (Align::RightJustified, "palign-justify-right"),
                (Align::FullyJustified, "palign-justify-all"),
                (Align::AwayFromSpine, "palign-spine-away"),
            ] {
                if icons::button(ui, icon, 20.0, cur == a, &tr(a.label())).clicked() {
                    align = Some(a);
                }
            }
        }
    });
    align
}

/// The indents and space before and after, two per column. Returns the one field changed this
/// frame, with its paragraph attribute and value.
fn indent_space_fields(ui: &mut egui::Ui, para: &serde_json::Value, units: Unit, tr: &dyn Fn(&str) -> String) -> Option<(&'static str, f64)> {
    let mut set = None;
    for ((ia, ta, ka), (ib, tb, kb)) in [
        (("indent-left", "Left Indent", "leftIndent"), ("indent-first", "First Line Left Indent", "firstLineIndent")),
        (("indent-right", "Right Indent", "rightIndent"), ("indent-last", "Last Line Right Indent", "lastLineIndent")),
        (("space-before", "Space Before", "spaceBefore"), ("space-after", "Space After", "spaceAfter")),
    ] {
        rows(ui, |ui, row| {
            if row == 0 {
                if let Some(v) = icon_field(ui, ia, &tr(ta), NumField::measure(ka, para[ka].as_f64(), units).width(84.0).range(0.0, 10_000.0)) {
                    set = Some((ka, v));
                }
            } else {
                let min = crate::panels::properties::para_field_min(kb);
                if let Some(v) = icon_field(ui, ib, &tr(tb), NumField::measure(kb, para[kb].as_f64(), units).width(84.0).range(min, 10_000.0)) {
                    set = Some((kb, v));
                }
            }
        });
        vsep(ui);
    }
    set
}

/// The drop cap's line and character counts. Returns the one changed this frame, with its
/// paragraph attribute and value.
fn drop_cap_fields(ui: &mut egui::Ui, para: &serde_json::Value, tr: &dyn Fn(&str) -> String) -> Option<(&'static str, f64)> {
    let mut drop = None;
    rows(ui, |ui, row| {
        if row == 0 {
            let max = designcraft_compose::MAX_DROP_CAP_LINES as f64;
            if let Some(v) = icon_field(
                ui,
                "drop-lines",
                &tr("Drop Cap Number of Lines"),
                NumField::number("cdl", para["dropCapLines"].as_f64(), "", 0).width(64.0).range(0.0, max),
            ) {
                drop = Some(("dropCapLines", v));
            }
        } else {
            let max = designcraft_compose::MAX_DROP_CAP_CHARS as f64;
            if let Some(v) = icon_field(
                ui,
                "drop-chars",
                &tr("Drop Cap One or More Characters"),
                NumField::number("cdc", para["dropCapChars"].as_f64(), "", 0).width(64.0).range(0.0, max),
            ) {
                drop = Some(("dropCapChars", v));
            }
        }
    });
    drop
}

/// Shading over Border: the on/off checkboxes, the colour swatches and the Paragraph Borders and
/// Shading buttons. Returns the attributes to set and the dialog tab to open, if any.
fn shading_buttons(
    app: &DesignApp,
    ui: &mut egui::Ui,
    para: &serde_json::Value,
    tr: &dyn Fn(&str) -> String,
) -> (Option<serde_json::Value>, Option<&'static str>) {
    // On or off, the Paragraph Borders and Shading dialog on its tab, and the colour, as InDesign's.
    let doc = app.session.active().map(|d| d.doc.clone());
    let mut shade = None;
    let mut dialog = None;
    // Both checkboxes as wide as the wider label, so the icons and colours line up in columns.
    let check_w = ["Shading", "Border"]
        .iter()
        .map(|l| ui.fonts_mut(|f| f.layout_no_wrap(tr(l), egui::FontId::proportional(11.5), Color32::WHITE).size().x))
        .fold(0.0, f32::max)
        + 28.0;
    rows(ui, |ui, row| {
        let (on_key, color_key, label, icon, tab) = if row == 0 {
            ("shadingOn", "shadingColor", "Shading", "para-shading", "shading")
        } else {
            ("borderOn", "borderColor", "Border", "para-border", "border")
        };
        let mut on = para[on_key].as_bool().unwrap_or(false);
        ui.allocate_ui_with_layout(vec2(check_w, crate::widgets::FIELD_H), egui::Layout::left_to_right(egui::Align::Center), |ui| {
            ui.set_min_width(check_w);
            if crate::widgets::checkbox(ui, &mut on, &tr(label)).changed() {
                shade = Some(json!({ on_key: on }));
            }
        });
        if crate::widgets::icon_toggle(ui, icon, false, &tr("Paragraph Borders and Shading…")).clicked() {
            dialog = Some(tab);
        }
        if let Some(doc) = &doc {
            let cur = para[color_key].as_str().unwrap_or("[Black]").to_string();
            if let Some(w) = swatch_field(ui, doc, &cur, 96.0, &tr(if row == 0 { "Paragraph Shading" } else { "Paragraph Border" })) {
                shade = Some(json!({ color_key: w }));
            }
        }
    });
    (shade, dialog)
}

/// A swatch field of the Control panel (Shading and Border colours): the swatch's chip and name,
/// and the document's swatches in its menu. Returns the swatch picked.
pub(crate) fn swatch_field(ui: &mut egui::Ui, doc: &designcraft_doc::Document, cur: &str, width: f32, label: &str) -> Option<String> {
    let t = Tokens::get(ui.ctx());
    let (r, resp) = ui.allocate_exact_size(vec2(width, crate::widgets::FIELD_H), Sense::click());
    let resp = resp.on_hover_text(label);
    resp.widget_info(|| {
        let mut info = egui::WidgetInfo::labeled(egui::Role::ComboBox, true, label);
        info.current_text_value = Some(cur.to_string());
        info
    });
    let p = ui.painter();
    p.rect_filled(r, 0.0, if resp.hovered() { t.hover } else { t.input });
    p.rect_stroke(r, 0.0, Stroke::new(1.0, t.field_border), egui::StrokeKind::Inside);
    let seg = egui::Rect::from_min_max(egui::pos2(r.max.x - crate::widgets::SEG_W, r.min.y), r.max);
    p.line_segment([seg.left_top(), seg.left_bottom()], Stroke::new(1.0, t.field_border));
    icons::paint(p, egui::Rect::from_center_size(seg.center(), vec2(12.0, 12.0)), "chevron-down", t.icon);
    let (c, g) = crate::widgets::swatch_colors(doc, cur, 1.0);
    let chip = egui::Rect::from_min_size(egui::pos2(r.min.x + 4.0, r.center().y - 6.0), vec2(12.0, 12.0));
    crate::widgets::paint_chip(p, chip, c, g);
    let clip = egui::Rect::from_min_max(r.min, egui::pos2(seg.min.x - 2.0, r.max.y));
    p.with_clip_rect(clip).text(egui::pos2(chip.max.x + 5.0, r.center().y), egui::Align2::LEFT_CENTER, cur, egui::FontId::proportional(11.5), t.text);
    let mut picked = None;
    egui::Popup::menu(&resp).show(|ui| {
        picked = crate::panels::swatch_list(ui, doc, cur);
    });
    picked
}

/// The widest a document tab grows: a longer name is shortened so its close button and the
/// tabs after it stay in view.
const DOC_TAB_MAX_W: f32 = 260.0;
/// Close button and padding around a tab's title.
const DOC_TAB_PAD: f32 = 44.0;

/// Draw bounded document tabs with active, dirty, preview and zoom state, plus activation and close actions.
pub fn doc_tabs(app: &mut DesignApp, ui: &mut egui::Ui) {
    let t = Tokens::get(ui.ctx());
    let (bar, _) = ui.allocate_exact_size(vec2(ui.available_width(), 28.0), Sense::hover());
    let max_w = DOC_TAB_MAX_W.min(bar.width());
    let close_label = crate::i18n::tr(&app.ui.language, "Close");
    ui.painter().rect_filled(bar, 0.0, t.tab_strip);
    ui.painter().line_segment([bar.left_bottom(), bar.right_bottom()], Stroke::new(1.0, t.border));
    let mut x = bar.min.x;
    let active = app.session.active_index();
    let zoom = app.view().map(|v| v.zoom).unwrap_or(1.0);
    let mut activate = None;
    let mut close = None;
    let titles: Vec<(String, bool)> = app.session.documents().iter().map(|d| (d.title(), d.is_dirty())).collect();
    for (i, (title, dirty)) in titles.iter().enumerate() {
        let is_active = Some(i) == active;
        // `*` = unsaved; the view-mode suffix only in Preview (InDesign shows its GPU mode there).
        let suffix = if is_active && app.ui.screen_mode == crate::ScreenMode::Preview { " [Preview]" } else { "" };
        let color = if is_active { t.text_strong } else { t.text_dim };
        // The name gives way first; the dirty marker and zoom stay readable.
        let zoom_label = crate::rtl::plain(ui.ctx(), &format!(" @ {:.0}%{suffix}", zoom * 100.0), semibold(11.5), color);
        let name = format!("{}{}", if *dirty { "*" } else { "" }, crate::rtl::isolate(title));
        let name_max = max_w - DOC_TAB_PAD - zoom_label.size().x;
        let galley = crate::rtl::truncated(ui.ctx(), &name, semibold(11.5), color, name_max);
        let elided = galley.elided;
        let w = (galley.size().x + zoom_label.size().x + DOC_TAB_PAD).max(if is_active { 210.0 } else { 150.0 }).min(max_w);
        let r = egui::Rect::from_min_size(egui::pos2(x, bar.min.y), vec2(w, 28.0));
        let resp = ui.interact(r, ui.scope_id().with(("doctab", i)), Sense::click());
        resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, title));
        let resp = if elided {
            resp.on_hover_ui(|ui| {
                let _ = crate::rtl::label(ui, title);
            })
        } else {
            resp
        };
        ui.painter().rect_filled(r, 0.0, if is_active { t.panel } else { t.tab_strip });
        let cr = egui::Rect::from_center_size(egui::pos2(r.min.x + 18.0, r.center().y), vec2(14.0, 14.0));
        let cresp = ui.interact(cr, ui.scope_id().with(("docclose", i)), Sense::click());
        cresp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, format!("{close_label} {title}")));
        ui.painter().text(
            cr.center(),
            egui::Align2::CENTER_CENTER,
            "×",
            egui::FontId::proportional(14.0),
            if cresp.hovered() { t.text_strong } else { t.text_dim },
        );
        let name_w = galley.size().x;
        // Clipped too, for a window narrower than the zoom label.
        let text = ui.painter().with_clip_rect(r);
        text.galley(egui::pos2(r.min.x + 32.0, r.center().y - galley.size().y / 2.0), galley, Color32::WHITE);
        text.galley(egui::pos2(r.min.x + 32.0 + name_w, r.center().y - zoom_label.size().y / 2.0), zoom_label, Color32::WHITE);
        ui.painter().line_segment([r.right_top(), r.right_bottom()], Stroke::new(1.0, t.border));
        if cresp.clicked() {
            close = Some(i);
        } else if resp.clicked() {
            activate = Some(i);
        }
        x += w;
    }
    if let Some(i) = close {
        crate::menus::close_document(app, Some(i));
    } else if let Some(i) = activate {
        let _ = app.run("file.activate", json!({"index": i}));
    }
}

/// Show the Preflight panel. An open Preflight flyout stays: `window.panel` would close it.
fn show_preflight(app: &mut DesignApp) {
    if app.ui.docking_flyout.as_deref() != Some("preflight")
        && let Err(e) = app.run("window.panel", json!({"panel": "preflight"}))
    {
        app.status(e);
    }
}

pub fn status_bar(app: &mut DesignApp, ui: &mut egui::Ui) {
    let t = Tokens::get(ui.ctx());
    let resp = egui::Panel::bottom("status_bar")
        .exact_size(17.0)
        .frame(egui::Frame::NONE.fill(t.panel).inner_margin(egui::Margin::symmetric(8, 0)))
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            ui.horizontal_centered(|ui| {
                let small = egui::FontId::proportional(10.5);
                if app.session.active().is_none() {
                    ui.label(egui::RichText::new(&app.ui.status).font(small).color(t.text_dim));
                    return;
                }
                let z = app.view().map(|v| v.zoom).unwrap_or(1.0);
                crate::menus::menu_button(ui, egui::RichText::new(format!("{:.2}% ▾", z * 100.0)).font(small.clone()), |ui| {
                    for p in [0.05, 0.125, 0.25, 0.5, 0.75, 1.0, 2.0, 4.0, 8.0, 16.0] {
                        if ui.button(format!("{}%", p * 100.0)).clicked() {
                            let _ = app.run("view.zoom", json!({"zoom": p}));
                            ui.close();
                        }
                    }
                });
                ui.add_space(8.0);
                let n = app.session.active().map(|d| d.doc.page_count()).unwrap_or(1);
                let cur = crate::canvas::current_page(app).unwrap_or(0);
                if icons::button(ui, "first", 14.0, false, crate::i18n::tr(&app.ui.language, "First Spread")).clicked() {
                    crate::canvas::go_to_page(app, 0);
                }
                if icons::button(ui, "prev", 14.0, false, crate::i18n::tr(&app.ui.language, "Previous Spread")).clicked() {
                    crate::canvas::go_to_page(app, cur.saturating_sub(1));
                }
                let name = app.session.page_label(cur);
                let (fr, fresp) = ui.allocate_exact_size(vec2(80.0, 14.0), Sense::click());
                ui.painter().rect(fr, 0.0, t.input, Stroke::new(1.0, t.field_border), egui::StrokeKind::Inside);
                ui.painter().text(fr.left_center() + vec2(5.0, 0.0), egui::Align2::LEFT_CENTER, &name, small.clone(), t.text);
                ui.painter().text(fr.right_center() - vec2(5.0, 0.0), egui::Align2::RIGHT_CENTER, "▾", small.clone(), t.text);
                egui::Popup::menu(&fresp).show(|ui| {
                    let names: Vec<String> = (0..n).map(|i| app.session.page_label(i)).collect();
                    egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                        for (i, nm) in names.iter().enumerate() {
                            if ui.button(nm).clicked() {
                                crate::canvas::go_to_page(app, i);
                                ui.close();
                            }
                        }
                    });
                });
                if icons::button(ui, "next", 14.0, false, crate::i18n::tr(&app.ui.language, "Next Spread")).clicked() {
                    crate::canvas::go_to_page(app, (cur + 1).min(n - 1));
                }
                if icons::button(ui, "last", 14.0, false, crate::i18n::tr(&app.ui.language, "Last Spread")).clicked() {
                    crate::canvas::go_to_page(app, n - 1);
                }
                ui.add_space(12.0);
                crate::rtl::label(ui, egui::RichText::new(crate::i18n::tr(&app.ui.language, "[Basic] (working) ▾")).font(small.clone()));
                ui.add_space(8.0);
                // Preflight: overset text is an error.
                let errors = panels::preflight_errors(app);
                let (r, dot) = ui.allocate_exact_size(vec2(9.0, 9.0), Sense::click());
                ui.painter().circle_filled(
                    r.center(),
                    4.0,
                    if errors == 0 { Color32::from_rgb(60, 200, 90) } else { Color32::from_rgb(235, 50, 50) },
                );
                let count = if errors == 0 {
                    format!("{} ▾", crate::i18n::tr(&app.ui.language, "No errors"))
                } else {
                    format!(
                        "{} ▾",
                        crate::i18n::count_label(
                            &app.ui.language,
                            if errors == 1 && !crate::i18n::category_first(&app.ui.language) { "error" } else { "errors" },
                            errors
                        )
                    )
                };
                // As in InDesign, the count opens a menu that leads to the Preflight panel.
                let menu = crate::menus::menu_button(ui, egui::RichText::new(count).font(small.clone()), |ui| {
                    if ui.button(crate::i18n::tr(&app.ui.language, "Preflight Panel…")).clicked() {
                        show_preflight(app);
                        ui.close();
                    }
                });
                // Double-clicking the light or the count shows the panel straight away (the
                // second click has closed the menu the first one opened).
                if dot.double_clicked() || menu.response.double_clicked() {
                    show_preflight(app);
                }
                ui.add_space(12.0);
                ui.label(egui::RichText::new(&app.ui.status).font(small).color(t.text_dim));
            });
        });
    let r = resp.response.rect;
    ui.painter().line_segment([r.left_top(), r.right_top()], Stroke::new(1.0, t.border));
}

pub fn start_screen(app: &mut DesignApp, ui: &mut egui::Ui) {
    let t = Tokens::get(ui.ctx());
    let r = ui.available_rect_before_wrap();
    ui.painter().rect_filled(r, 0.0, t.panel_darker);
    ui.scope_builder(
        egui::UiBuilder::new().max_rect(r.shrink(48.0)).layout(egui::Layout::top_down(if crate::i18n::is_rtl(&app.ui.language) {
            egui::Align::Max
        } else {
            egui::Align::Min
        })),
        |ui| {
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                ui.label(crate::rtl::widget(
                    ui,
                    egui::RichText::new(crate::i18n::tr(&app.ui.language, "Welcome to DesignCraft")).font(semibold(28.0)).color(t.text_strong),
                ));
                ui.label(crate::rtl::widget(
                    ui,
                    egui::RichText::new(crate::i18n::tr(&app.ui.language, "Page layout for print and screen — fast, open and scriptable."))
                        .size(15.0)
                        .color(t.text_dim),
                ));
                ui.add_space(24.0);
                ui.horizontal(|ui| {
                    if ui
                        .add(
                            egui::Button::new(crate::rtl::widget(
                                ui,
                                egui::RichText::new(crate::i18n::tr(&app.ui.language, "  New file  ")).size(14.0).color(Color32::WHITE),
                            ))
                            .fill(t.accent_strong)
                            .corner_radius(16.0)
                            .min_size(vec2(0.0, 32.0)),
                        )
                        .clicked()
                    {
                        let _ = app.run("app.newDocumentDialog", json!({}));
                    }
                    if ui
                        .add(
                            egui::Button::new(crate::rtl::widget(ui, egui::RichText::new(crate::i18n::tr(&app.ui.language, "  Open  ")).size(14.0)))
                                .corner_radius(16.0)
                                .min_size(vec2(0.0, 32.0)),
                        )
                        .clicked()
                    {
                        let _ = app.run("app.openDialog", json!({}));
                    }
                    if ui
                        .add(
                            egui::Button::new(crate::rtl::widget(
                                ui,
                                egui::RichText::new(crate::i18n::tr(&app.ui.language, "  Open sample magazine  ")).size(14.0),
                            ))
                            .corner_radius(16.0)
                            .min_size(vec2(0.0, 32.0)),
                        )
                        .clicked()
                    {
                        let _ = app.run("file.newSample", json!({}));
                    }
                });
                ui.add_space(20.0);
                community_card(app, ui);
                recent_files(app, ui);
                ui.add_space(24.0);
                ui.label(crate::rtl::widget(
                    ui,
                    egui::RichText::new(crate::i18n::tr(&app.ui.language, "Start a new document")).font(semibold(15.0)).color(t.text_strong),
                ));
                ui.add_space(8.0);
                ui.horizontal_wrapped(|ui| {
                    for p in designcraft_doc::build::PRESETS.iter().filter(|p| p.intent == designcraft_doc::Intent::Print) {
                        let (cr, resp) = ui.allocate_exact_size(vec2(120.0, 150.0), Sense::click());
                        let hov = resp.hovered();
                        ui.painter().rect_filled(cr, 6.0, if hov { t.hover } else { t.panel });
                        let k = (80.0 / p.width.max(p.height)) as f32;
                        let pr = egui::Rect::from_center_size(cr.center() - vec2(0.0, 14.0), vec2(p.width as f32 * k, p.height as f32 * k));
                        ui.painter().rect_filled(pr, 0.0, Color32::from_gray(245));
                        ui.painter().text(
                            egui::pos2(cr.center().x, cr.max.y - 26.0),
                            egui::Align2::CENTER_CENTER,
                            p.name,
                            semibold(12.0),
                            t.text_strong,
                        );
                        ui.painter().text(
                            egui::pos2(cr.center().x, cr.max.y - 11.0),
                            egui::Align2::CENTER_CENTER,
                            format!(
                                "{} × {}",
                                designcraft_geom::format_measure(p.width, p.units),
                                designcraft_geom::format_measure(p.height, p.units)
                            ),
                            egui::FontId::proportional(10.0),
                            t.text_dim,
                        );
                        if resp.clicked() {
                            let _ = app.run("file.new", json!({"preset": p.name}));
                        }
                    }
                });
            });
        },
    );
    let _ = Content::Unassigned;
}

/// Recent files shown on the start screen (File › Open Recent has them all).
const START_SCREEN_RECENT: usize = 8;

/// Start screen: the most recent files as cards with the first page's picture (saved in the file)
/// and the name; the full path is the tooltip. A file that is gone is dimmed and says so when
/// clicked. Nothing when the list is empty.
fn recent_files(app: &mut DesignApp, ui: &mut egui::Ui) {
    if app.session.prefs.recent_files.is_empty() {
        return;
    }
    let t = Tokens::get(ui.ctx());
    ui.add_space(24.0);
    ui.label(crate::rtl::widget(
        ui,
        egui::RichText::new(crate::i18n::tr(&app.ui.language, "Recent files")).font(semibold(15.0)).color(t.text_strong),
    ));
    ui.add_space(8.0);
    let rows: Vec<(usize, String)> = app.session.prefs.recent_files.iter().take(START_SCREEN_RECENT).cloned().enumerate().collect();
    let mut open = None;
    ui.horizontal_wrapped(|ui| {
        for (i, path) in rows {
            let stat = recent_file_stat(ui.ctx(), &path);
            let there = stat.exists;
            let modified = there.then_some(stat.modified);
            let (cr, resp) = ui.allocate_exact_size(vec2(140.0, 170.0), Sense::click());
            let resp = resp.on_hover_text(&path);
            ui.painter().rect_filled(cr, 6.0, if resp.hovered() { t.hover } else { t.panel });
            let area = egui::Rect::from_min_size(cr.min + vec2(20.0, 12.0), vec2(100.0, 108.0));
            match modified.and_then(|m| recent_preview_texture(ui.ctx(), &path, m)) {
                Some(tex) => {
                    let size = tex.size_vec2();
                    let k = (area.width() / size.x).min(area.height() / size.y);
                    let pr = egui::Rect::from_center_size(area.center(), size * k);
                    ui.painter().image(tex.id(), pr, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), Color32::WHITE);
                    ui.painter().rect_stroke(pr, 0.0, Stroke::new(1.0, t.border), egui::StrokeKind::Outside);
                }
                // No picture (saved before files had one) or the file is gone: a blank page.
                None => {
                    let pr = egui::Rect::from_center_size(area.center(), vec2(62.0, 80.0));
                    ui.painter().rect_filled(pr, 0.0, if there { Color32::from_gray(245) } else { Color32::from_gray(110) });
                }
            }
            let color = if there { t.text_strong } else { t.text_dim };
            // The name without `.designcraft`, on up to two lines (broken after a space or dash).
            let label = crate::menus::recent_label(&path);
            let name = label.strip_suffix(".designcraft").unwrap_or(&label).to_string();
            let mut job = egui::text::LayoutJob::simple(name, semibold(12.0), color, cr.width() - 12.0);
            job.wrap.max_rows = 2;
            job.halign = egui::Align::Center;
            let galley = ui.painter().layout_job(job);
            ui.painter().galley(egui::pos2(cr.center().x, area.max.y + 8.0), galley, color);
            if resp.clicked() {
                open = Some(i);
            }
        }
    });
    if let Some(i) = open {
        let _ = app.run("app.openRecent", json!({"index": i}));
    }
}

/// How often a recent file is looked up on disk again, in ms: the start screen and the Open Recent
/// menu draw every frame and must not touch the file system each time.
const RECENT_STAT_MS: f64 = 3000.0;

/// A recent file as last seen on disk.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct RecentStat {
    pub exists: bool,
    pub modified: Option<std::time::SystemTime>,
    checked: f64,
}

/// Whether recent file `path` is there and when it last changed, looked up again at most every
/// [`RECENT_STAT_MS`].
pub(crate) fn recent_file_stat(ctx: &egui::Context, path: &str) -> RecentStat {
    recent_file_stat_at(ctx, path, crate::now_ms())
}

fn recent_file_stat_at(ctx: &egui::Context, path: &str, now: f64) -> RecentStat {
    let key = egui::Id::unique(("recent_stat", path));
    if let Some(st) = ctx.data(|d| d.get_temp::<RecentStat>(key))
        && (0.0..RECENT_STAT_MS).contains(&(now - st.checked))
    {
        return st;
    }
    let meta = designcraft_engine::io::metadata(path).ok().filter(designcraft_engine::io::Metadata::is_file);
    let st = RecentStat { exists: meta.is_some(), modified: meta.and_then(|m| m.modified().ok()), checked: now };
    ctx.data_mut(|d| d.insert_temp(key, st.clone()));
    st
}

/// The texture of a recent file's saved first-page picture, decoded once per file version.
fn recent_preview_texture(ctx: &egui::Context, path: &str, modified: Option<std::time::SystemTime>) -> Option<egui::TextureHandle> {
    type Cached = (Option<std::time::SystemTime>, Option<egui::TextureHandle>);
    let key = egui::Id::unique(("recent_preview", path));
    if let Some((m, tex)) = ctx.data(|d| d.get_temp::<Cached>(key))
        && m == modified
    {
        return tex;
    }
    let tex = designcraft_engine::cmd::recent_preview(path).and_then(|p| {
        let size = [usize::try_from(p.width).ok()?, usize::try_from(p.height).ok()?];
        (size[0].checked_mul(size[1])?.checked_mul(4)? == p.rgba.len()).then(|| {
            ctx.load_texture(format!("recent_preview:{path}"), egui::ColorImage::from_rgba_unmultiplied(size, &p.rgba), egui::TextureOptions::LINEAR)
        })
    });
    ctx.data_mut(|d| d.insert_temp::<Cached>(key, (modified, tex.clone())));
    tex
}

/// Start screen: the ArtCraft community (Discord first) and project links.
fn community_card(app: &mut DesignApp, ui: &mut egui::Ui) {
    let t = Tokens::get(ui.ctx());
    egui::Frame::NONE.fill(t.panel).corner_radius(10.0).inner_margin(egui::Margin::same(14)).show(ui, |ui| {
        // Wide enough for the longer translations of the line under the title, with the button
        // at the right edge; the text wraps before the button's space rather than run under it.
        const BUTTON: egui::Vec2 = vec2(190.0, 38.0);
        ui.set_max_width(760.0);
        ui.horizontal(|ui| {
            let (r, _) = ui.allocate_exact_size(vec2(44.0, 44.0), Sense::hover());
            crate::about::paint_mark(ui, r, crate::about::BRAND);
            ui.add_space(8.0);
            let label = crate::i18n::tr(&app.ui.language, "Join our Discord");
            let button = crate::about::discord_button_size(ui.ctx(), label, BUTTON);
            let text_width = (ui.available_width() - button.x - 16.0).max(120.0);
            ui.vertical(|ui| {
                ui.set_max_width(text_width);
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
                ui.label(crate::rtl::widget(
                    ui,
                    egui::RichText::new(crate::i18n::tr(&app.ui.language, "Join the ArtCraft community")).font(semibold(14.0)).color(t.text_strong),
                ));
                ui.label(crate::rtl::widget(
                    ui,
                    egui::RichText::new(crate::i18n::tr(&app.ui.language, "Get help, share your layouts and shape what we build next."))
                        .size(12.0)
                        .color(t.text_dim),
                ));
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    for (text, id) in [("DesignCraft page", "help.appPage"), ("GitHub", "help.github"), ("getartcraft.com", "help.website")] {
                        let url = designcraft_engine::links::get(&id[5..]).unwrap_or_default();
                        let l = ui
                            .add(egui::Label::new(egui::RichText::new(text).size(12.0).color(t.accent).underline()).sense(Sense::click()))
                            .on_hover_text(url)
                            .on_hover_cursor(egui::CursorIcon::PointingHand);
                        if l.clicked() {
                            let _ = app.run(id, json!({}));
                        }
                        ui.add_space(8.0);
                    }
                });
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if crate::about::discord_button(ui, label, BUTTON) {
                    let _ = app.run("help.discord", json!({}));
                }
            });
        });
    });
}

#[cfg(test)]
mod tests;
