//! Application-owned panel IDs and commands around the shared docking layout.
//!
//! The window's panels and the canvas are one craft-ui layout: a horizontal split of the
//! protected `canvas` panel and a dock at the logical end edge (the right in left-to-right
//! languages, the left in right-to-left ones).

use craft_ui::docking::{Action, Column, Floating, Group, Layout, Node, Permissions, Placement, Side, Target, Zone};
use serde_json::{Value, json};

use crate::DesignApp;

pub(crate) fn cancel_gesture(ctx: &egui::Context) {
    craft_ui::docking::cancel_drag::<String>(ctx, egui::Id::new("designcraft-panel-docking"));
}

fn normalize(raw: &str) -> Option<&'static str> {
    crate::dock::DOCK_TABS
        .iter()
        .chain(crate::dock::ICON_PANELS)
        .find(|(id, label, _)| id.eq_ignore_ascii_case(raw.trim()) || label.eq_ignore_ascii_case(raw.trim()))
        .map(|(id, _, _)| *id)
}

/// The panel that holds the document area. It never moves, floats, closes or takes tabs.
pub(crate) const CANVAS: &str = "canvas";
/// Width of an expanded dock column.
const COLUMN_WIDTH: f32 = 260.0;
/// Width of an iconic column of bare icons, and of one whose groups show their names.
const ICON_WIDTH: f32 = 37.0;
const NAMED_ICON_WIDTH: f32 = 150.0;
/// Height of the group a workspace opens below Properties · Pages · Layers.
const OPEN_PANEL_HEIGHT: f32 = 260.0;

fn registered(id: &str) -> bool {
    id == CANVAS || normalize(id) == Some(id)
}

/// The policy commands and the renderer apply: the canvas is protected.
fn model_permissions(id: &String) -> Permissions {
    if id == CANVAS { Permissions::PROTECTED } else { Permissions::default() }
}

pub(crate) fn valid(layout: &Layout<String>) -> bool {
    layout.validate().is_ok()
        && layout.panels().iter().all(|id| registered(id))
        && Layout { root: layout.root.clone(), floating: Vec::new() }.contains(&CANVAS.to_string())
}

/// Panels already given a place, so each appears once.
#[derive(Default)]
struct Placed(Vec<String>);

impl Placed {
    /// The registered id of `raw` if no other place holds it yet.
    fn take(&mut self, raw: &str) -> Option<String> {
        let id = normalize(raw)?;
        if self.0.iter().any(|p| p == id) {
            return None;
        }
        self.0.push(id.to_string());
        Some(id.to_string())
    }

    /// A group of the unplaced registered panels of `panels`, with the one at `active` in front
    /// when it is among them.
    fn group(&mut self, panels: &[String], active: usize) -> Option<Group<String>> {
        let front = panels.get(active).and_then(|p| normalize(p));
        let kept: Vec<String> = panels.iter().filter_map(|p| self.take(p)).collect();
        let active = front.and_then(|f| kept.iter().position(|p| p == f)).unwrap_or(0);
        (!kept.is_empty()).then(|| Group::new(kept, active))
    }
}

/// A workspace's iconic column: its named `icon_groups` (Essentials Classic, Typography), or
/// one group per panel not placed yet. Window › Styles opens the style panels as a floating
/// group, so they start outside it.
fn icon_column(icon_groups: Option<&[Vec<String>]>, placed: &mut Placed) -> Option<Column<String>> {
    let (groups, width): (Vec<Group<String>>, f32) = match icon_groups {
        Some(groups) => (groups.iter().filter_map(|g| placed.group(g, 0)).collect(), NAMED_ICON_WIDTH),
        None => (
            crate::dock::DOCK_TABS
                .iter()
                .chain(crate::dock::ICON_PANELS)
                .filter(|p| !FLOATING_GROUPS.iter().any(|g| g.contains(&p.0)))
                .filter_map(|p| placed.take(p.0))
                .map(|id| Group::new(vec![id], 0))
                .collect(),
            ICON_WIDTH,
        ),
    };
    (!groups.is_empty()).then_some(Column { groups, width: COLUMN_WIDTH, iconic: true, iconic_width: Some(width) })
}

/// The canvas beside a dock of `columns` (from the window edge inwards) at the logical end edge.
fn with_canvas(columns: Vec<Column<String>>, floating: Vec<Floating<String>>) -> Layout<String> {
    let width: f32 = columns.iter().map(|c| if c.iconic { c.iconic_width.unwrap_or(ICON_WIDTH) } else { c.width }).sum();
    Layout {
        root: Some(Node::Split {
            axis: craft_ui::layout::SplitAxis::Horizontal,
            size: craft_ui::layout::SplitSize::FixedSecond(width),
            first: Box::new(Node::Tabs { panels: vec![CANVAS.to_string()], active: 0 }),
            second: Box::new(Node::Dock { side: Side::End, columns }),
        }),
        floating,
    }
}

/// The layout of a workspace that has not been customized, from its panel settings: the
/// canvas beside a dock whose outer column is the Properties · Pages · Layers group (with an
/// the panel the workspace opens stacked below it) and whose inner column is iconic. Essentials Classic
/// and Typography (`icon_groups`) have the iconic column only.
pub(crate) fn default_layout(ui: &crate::UiState) -> Layout<String> {
    let mut placed = Placed::default();
    let mut floating = Vec::new();
    for (id, at) in &ui.floating {
        if at.iter().all(|v| v.is_finite())
            && let Some(id) = placed.take(id)
        {
            floating.push(Floating::single(vec![id], 0, [at[0], at[1], 280.0, 360.0]));
        }
    }
    let mut columns = Vec::new();
    if ui.icon_groups.is_none() {
        let panels: Vec<String> = crate::dock::DOCK_TABS.iter().filter_map(|p| placed.take(p.0)).collect();
        let active = panels.iter().position(|p| p == &ui.dock_tab).unwrap_or(0);
        let mut groups = Vec::new();
        if !panels.is_empty() {
            groups.push(Group::new(panels, active));
        }
        // The panel a workspace opens (Printing and Proofing: Swatches) goes below at a fixed
        // height, so Properties keeps most of the column.
        if let Some(id) = ui.open_panel.as_deref().and_then(|id| placed.take(id)) {
            groups.push(Group { height: Some(OPEN_PANEL_HEIGHT), ..Group::new(vec![id], 0) });
        }
        if !groups.is_empty() {
            columns.push(Column { groups, width: COLUMN_WIDTH, iconic: !ui.dock_expanded, iconic_width: Some(ICON_WIDTH) });
        }
    }
    columns.extend(icon_column(ui.icon_groups.as_deref(), &mut placed));
    with_canvas(columns, floating)
}

/// Is `layout` from before the canvas joined the panel layout? Those layouts held only the
/// right dock's panels and floating windows; every later one holds the canvas.
pub(crate) fn dock_only(layout: &Layout<String>) -> bool {
    !layout.panels().iter().any(|p| p.as_str() == CANVAS)
}

/// A [`dock_only`] layout in the current shape: its tab groups, accordions and splits become
/// the dock's expanded columns (collapsed to icons when the dock was), the panels it does not
/// hold form the iconic column, and its floating windows stay. Panels closed with a remembered
/// place (`hidden`) stay closed. None when the layout is invalid.
pub(crate) fn from_dock_only<'a>(
    old: &Layout<String>,
    hidden: impl IntoIterator<Item = &'a String>,
    dock_expanded: bool,
    icon_groups: Option<&[Vec<String>]>,
) -> Option<Layout<String>> {
    old.validate().ok()?;
    let mut placed = Placed::default();
    // Reading order (start to end); a dock lists its columns from the window edge inwards.
    let mut reading = old.root.as_ref().map(|root| dock_only_columns(root, 0, &mut placed)).unwrap_or_default();
    reading.reverse();
    let mut columns: Vec<Column<String>> =
        reading.into_iter().map(|groups| Column { groups, width: COLUMN_WIDTH, iconic: !dock_expanded, iconic_width: Some(ICON_WIDTH) }).collect();
    let mut floating = Vec::new();
    for window in &old.floating {
        let groups: Vec<Group<String>> = window
            .groups
            .iter()
            .filter_map(|g| placed.group(&g.panels, g.active).map(|kept| Group { minimized: g.minimized, height: g.height, ..kept }))
            .collect();
        if !groups.is_empty() {
            floating.push(Floating { groups, rect: window.rect, native: None });
        }
    }
    for panel in hidden {
        let _ = placed.take(panel);
    }
    columns.extend(icon_column(icon_groups, &mut placed));
    let layout = with_canvas(columns, floating);
    valid(&layout).then_some(layout)
}

/// The groups of a dock-only layout's node, column by column in reading order. A vertical split
/// stacks its parts in one column; a horizontal split puts them side by side.
fn dock_only_columns(node: &Node<String>, depth: usize, placed: &mut Placed) -> Vec<Vec<Group<String>>> {
    if depth > craft_ui::docking::MAX_DEPTH {
        return Vec::new();
    }
    let column = |groups: Vec<Group<String>>| if groups.is_empty() { Vec::new() } else { vec![groups] };
    match node {
        Node::Tabs { panels, active } => column(placed.group(panels, *active).into_iter().collect()),
        Node::Stack { entries } => column(
            entries
                .iter()
                .filter_map(|e| placed.take(&e.panel).map(|id| Group { minimized: !e.open, height: e.height, ..Group::new(vec![id], 0) }))
                .collect(),
        ),
        Node::Split { axis, first, second, .. } => {
            let mut parts = dock_only_columns(first, depth.saturating_add(1), placed);
            parts.extend(dock_only_columns(second, depth.saturating_add(1), placed));
            match axis {
                craft_ui::layout::SplitAxis::Horizontal => parts,
                _ => column(parts.into_iter().flatten().collect()),
            }
        }
        Node::Dock { columns, .. } => columns
            .iter()
            .rev()
            .filter_map(|c| {
                let groups: Vec<Group<String>> = c.groups.iter().filter_map(|g| placed.group(&g.panels, g.active)).collect();
                (!groups.is_empty()).then_some(groups)
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// The layout in use: the customized one, or the workspace's default.
fn layout_of(app: &DesignApp) -> Layout<String> {
    app.ui.docking.clone().unwrap_or_else(|| default_layout(&app.ui))
}

/// Is `panel` showing: in front of its group, and not in a column collapsed to icons?
pub(crate) fn showing(app: &DesignApp, panel: &str) -> bool {
    layout_of(app).panel_state(&panel.to_string()) == craft_ui::docking::PanelState::Front
}

/// Is `panel` in a dock column collapsed to icons?
fn in_iconic_column(layout: &Layout<String>, panel: &str) -> bool {
    dock_columns(layout).iter().any(|c| c.iconic && c.groups.iter().any(|g| g.panels.iter().any(|p| p == panel)))
}

/// Every dock column, from each dock's window edge inwards.
fn dock_columns(layout: &Layout<String>) -> Vec<&Column<String>> {
    let mut out = Vec::new();
    let mut visit: Vec<&Node<String>> = layout.root.iter().collect();
    let mut seen = 0usize;
    while let Some(node) = visit.pop() {
        seen = seen.saturating_add(1);
        if seen > 2 * craft_ui::docking::MAX_PANELS {
            break;
        }
        match node {
            Node::Split { first, second, .. } => {
                visit.push(second);
                visit.push(first);
            }
            Node::Dock { columns, .. } => out.extend(columns),
            _ => {}
        }
    }
    out
}

/// The panels tabbed with `panel`: its floating group, or its tab group in the dock. A panel in
/// an accordion, or not in the layout, is a group of its own.
#[cfg(test)]
pub(crate) fn tab_group(layout: &Layout<String>, panel: &str) -> Vec<String> {
    if let Some(group) = layout.floating.iter().flat_map(|w| &w.groups).find(|g| g.panels.iter().any(|p| p == panel)) {
        return group.panels.clone();
    }
    let mut visit: Vec<&Node<String>> = layout.root.iter().collect();
    let mut seen = 0usize;
    while let Some(node) = visit.pop() {
        seen = seen.saturating_add(1);
        if seen > 2 * craft_ui::docking::MAX_PANELS {
            break;
        }
        match node {
            Node::Split { first, second, .. } => {
                visit.push(first);
                visit.push(second);
            }
            Node::Tabs { panels, .. } if panels.iter().any(|p| p == panel) => return panels.clone(),
            Node::Dock { columns, .. } => {
                if let Some(group) = columns.iter().flat_map(|c| &c.groups).find(|g| g.panels.iter().any(|p| p == panel)) {
                    return group.panels.clone();
                }
            }
            _ => {}
        }
    }
    vec![panel.to_string()]
}

/// Is `panel` an open entry of an accordion (which Minimize closes)?
fn open_in_stack(layout: &Layout<String>, panel: &str) -> bool {
    let mut visit: Vec<&Node<String>> = layout.root.iter().collect();
    let mut seen = 0usize;
    while let Some(node) = visit.pop() {
        seen = seen.saturating_add(1);
        if seen > 2 * craft_ui::docking::MAX_PANELS {
            break;
        }
        match node {
            Node::Split { first, second, .. } => {
                visit.push(first);
                visit.push(second);
            }
            Node::Stack { entries } => {
                if let Some(e) = entries.iter().find(|e| e.panel == panel) {
                    return e.open;
                }
            }
            _ => {}
        }
    }
    false
}

/// Where a panel with no other destination goes: a panel of the first expanded dock column,
/// else of any column, else of the root.
fn first_dock_panel(layout: &Layout<String>, except: &str) -> Option<String> {
    let usable = |p: &&String| p.as_str() != except && p.as_str() != CANVAS;
    let columns = dock_columns(layout);
    let in_columns =
        |iconic: bool| columns.iter().filter(|c| c.iconic == iconic).flat_map(|c| &c.groups).flat_map(|g| &g.panels).find(usable).cloned();
    in_columns(false).or_else(|| in_columns(true)).or_else(|| {
        let root = Layout { root: layout.root.clone(), floating: Vec::new() };
        root.panels().into_iter().find(usable).cloned()
    })
}

fn ensure(layout: &mut Layout<String>, panel: &str) -> Result<(), String> {
    let anchor = first_dock_panel(layout, panel);
    apply(layout, Action::Open { panel: panel.into(), anchor })
}

fn apply(layout: &mut Layout<String>, action: Action<String>) -> Result<(), String> {
    layout.apply_with_permissions(action, model_permissions).map_err(|e| e.to_string())
}

fn number(params: &Value, key: &str, default: f32) -> Result<f32, String> {
    match params.get(key) {
        None => Ok(default),
        Some(v) => v
            .as_f64()
            .filter(|v| v.is_finite() && v.abs() <= 1_000_000.0)
            .map(|v| v as f32)
            .ok_or_else(|| format!("{key} must be a finite coordinate")),
    }
}

/// The side of the dock holding `panel`, if a dock column holds it.
fn dock_side(layout: &Layout<String>, panel: &str) -> Option<Side> {
    let mut visit: Vec<&Node<String>> = layout.root.iter().collect();
    let mut seen = 0usize;
    while let Some(node) = visit.pop() {
        seen = seen.saturating_add(1);
        if seen > 2 * craft_ui::docking::MAX_PANELS {
            break;
        }
        match node {
            Node::Split { first, second, .. } => {
                visit.push(first);
                visit.push(second);
            }
            Node::Dock { side, columns } if columns.iter().flat_map(|c| &c.groups).any(|g| g.panels.iter().any(|p| p == panel)) => {
                return Some(*side);
            }
            _ => {}
        }
    }
    None
}

/// The placement a `zone` names beside `anchor`. In a dock column or a floating window, top and
/// bottom add a group above or below the anchor's group; in a dock column, left and right add a
/// column on that side of the anchor's. Elsewhere the sides split the anchor's group.
fn placement(params: &Value, layout: &Layout<String>, anchor: &str, rtl: bool) -> Result<Placement<String>, String> {
    if let Some(before) = params.get("before") {
        let id = before.as_str().and_then(normalize).ok_or("before must name a panel")?;
        return Ok(Placement::Tab { before: Some(id.into()) });
    }
    let zone = match params.get("zone") {
        None => "tab",
        Some(value) => value.as_str().ok_or("zone must be a string")?,
    };
    let side = dock_side(layout, anchor);
    let floating = layout.floating.iter().any(|w| w.contains(anchor));
    Ok(match zone {
        "tab" => Placement::Tab { before: None },
        "center" => Placement::Split(Zone::Center),
        "top" | "bottom" if side.is_some() || floating => Placement::Group { below: zone == "bottom" },
        "left" | "right" if floating => return Err("a floating window has no columns; use top, bottom or tab".into()),
        "left" | "right" if let Some(side) = side => {
            // Columns count from the window edge inwards; `after` is away from the edge.
            let edge_is_right = (side == Side::End) != rtl;
            Placement::Column { after: (zone == "right") != edge_is_right }
        }
        "left" => Placement::Split(Zone::Left),
        "right" => Placement::Split(Zone::Right),
        "top" => Placement::Split(Zone::Top),
        "bottom" => Placement::Split(Zone::Bottom),
        _ => return Err("zone must be tab, center, left, right, top or bottom".into()),
    })
}

pub(crate) fn command(app: &mut DesignApp, id: &str, params: &Value) -> Option<Result<Value, String>> {
    if id == "app.tablePanel" {
        return Some(change(app, if showing(app, "table") { "close" } else { "activate" }, &json!({"panel": "table"})));
    }
    if id == "window.collapseToIcons" {
        return Some(set_iconic(app, true));
    }
    if id == "window.panel.closeGroup" {
        return Some(
            params
                .get("panel")
                .and_then(Value::as_str)
                .and_then(normalize)
                .ok_or_else(|| "panel must name a registered panel".to_string())
                .and_then(|panel| apply_action(app, Action::CloseGroup { panel: panel.into() })),
        );
    }
    if matches!(id, "window.floatPanel" | "window.dockPanel" | "window.panel.float" | "window.panel.dock")
        && let Some(panel) = params.get("panel").and_then(Value::as_str).filter(|p| crate::dock::float_only(p))
    {
        return Some(float_only_panel(app, id, panel, params));
    }
    if id == "window.panel"
        && let Some(result) = open_floating_group(app, params)
    {
        return Some(result);
    }
    if id == "window.panel.layout" {
        return Some(
            params
                .get("action")
                .ok_or_else(|| "action is required".to_string())
                .and_then(|value| serde_json::from_value::<Action<String>>(value.clone()).map_err(|error| error.to_string()))
                .and_then(|action| apply_action(app, action)),
        );
    }
    let operation = match id {
        "window.panel.move" => "move",
        "window.panel.float" | "window.floatPanel" => "float",
        "window.panel.dock" | "window.dockPanel" => "dock",
        "window.panel.activate" => "activate",
        "window.panel.close" => "close",
        // Float-only panels (Type › Tabs) are not in the layout.
        "window.panel" if params.get("panel").and_then(Value::as_str).and_then(normalize).is_some() => "activate",
        _ => return None,
    };
    Some(change(app, operation, params))
}

// UI output and automation share this atomic dispatcher. The application owns the registry
// and saved return locations; craft-ui owns tree validation and structural changes.
fn apply_action(app: &mut DesignApp, action: Action<String>) -> Result<Value, String> {
    let registered = |id: &String| self::registered(id);
    let known = match &action {
        Action::Move { panel, anchor, placement } | Action::OpenAt { panel, anchor, placement } => {
            registered(panel)
                && registered(anchor)
                && match placement {
                    Placement::Tab { before: Some(id) } => registered(id),
                    _ => true,
                }
        }
        Action::Open { panel, anchor } => registered(panel) && anchor.as_ref().is_none_or(registered),
        Action::Float { panel, .. }
        | Action::Close { panel }
        | Action::Activate { panel }
        | Action::MoveFloating { panel, .. }
        | Action::SetStackOpen { panel, .. }
        | Action::ResizeStack { panel, .. } => registered(panel),
        Action::ResizeSplit { path, .. } => path.len() <= craft_ui::docking::MAX_DEPTH,
        Action::MoveGroup { panel, to } | Action::MoveColumn { panel, to } | Action::MoveWindow { panel, to } => {
            registered(panel)
                && match to {
                    Target::At { anchor, placement } => {
                        registered(anchor)
                            && match placement {
                                Placement::Tab { before: Some(id) } => registered(id),
                                _ => true,
                            }
                    }
                    _ => true,
                }
        }
        Action::CloseGroup { panel }
        | Action::CloseWindow { panel }
        | Action::CloseOthers { panel }
        | Action::SetMinimized { panel, .. }
        | Action::SetIconic { panel, .. }
        | Action::ResizeColumn { panel, .. }
        | Action::ResizeGroup { panel, .. }
        | Action::ResizeIconic { panel, .. } => registered(panel),
        // Native OS windows are not enabled in DesignCraft; newer actions are rejected.
        _ => false,
    };
    if !known {
        return Err("docking action contains an unknown panel or an excessive split path".into());
    }
    if app.ui.docking.as_ref().is_some_and(|layout| !valid(layout)) {
        return Err("saved panel layout is invalid; reset the workspace first".into());
    }
    let mut layout = layout_of(app);
    let closed = layout.closed_by(&action).map_err(|e| e.to_string())?;
    // Window › a closed panel, and Dock Panel after a float, bring it back where it was.
    let returning: Vec<_> = match &action {
        Action::Float { panel, .. } => {
            layout.location(panel).ok().filter(|location| location.floating.is_none()).map(|location| (panel.clone(), location)).into_iter().collect()
        }
        _ => closed.iter().filter_map(|panel| layout.location(panel).ok().map(|location| (panel.clone(), location))).collect(),
    };
    let reveal = match &action {
        Action::Activate { panel } | Action::Open { panel, .. } | Action::OpenAt { panel, .. } => Some(panel.clone()),
        _ => None,
    };
    apply(&mut layout, action)?;
    for (panel, location) in returning {
        app.ui.docking_hidden.insert(panel, location);
    }
    app.ui.docking = Some(layout);
    release_closed(app, &closed);
    keep_float_only(app);
    if let Some(panel) = reveal {
        app.ui.docking_opened = Some(panel.clone());
        app.ui.docking_raise = Some(panel);
        app.ui.dock_expanded = true;
    } else if app.ui.docking_opened.as_ref().is_some_and(|panel| !app.ui.docking.as_ref().is_some_and(|layout| layout.contains(panel))) {
        app.ui.docking_opened = None;
    }
    app.ui.open_panel = None;
    Ok(json!({"layout": app.ui.docking, "closed": closed}))
}

/// Type › Tabs and the other float-only panels: their own windows outside the docking layout,
/// listed in `UiState::floating` with their top-left positions.
fn float_only_panel(app: &mut DesignApp, id: &str, panel: &str, params: &Value) -> Result<Value, String> {
    let at = if matches!(id, "window.floatPanel" | "window.panel.float") {
        Some([number(params, "x", 400.0)?, number(params, "y", 160.0)?])
    } else {
        None
    };
    app.ui.floating.retain(|(p, _)| p != panel);
    if let Some(at) = at {
        app.ui.floating.push((panel.to_string(), at));
    }
    Ok(json!({"panel": panel, "floating": floating_panels(app)}))
}

/// Once the layout is customized it holds every floating dock panel, so `UiState::floating`
/// keeps only the float-only panels.
fn keep_float_only(app: &mut DesignApp) {
    if app.ui.docking.is_some() {
        app.ui.floating.retain(|(id, _)| crate::dock::float_only(id));
    }
}

/// Every floating panel: the layout's floating windows, then the float-only panels.
pub(crate) fn floating_panels(app: &DesignApp) -> Vec<String> {
    let layout = layout_of(app);
    let docked = layout.floating.iter().flat_map(|w| w.panels().cloned());
    docked.chain(app.ui.floating.iter().filter(|(id, _)| crate::dock::float_only(id)).map(|(id, _)| id.clone())).collect()
}

/// The panel in front of each tab group in the dock (not the floating groups).
fn front_tabs(layout: &Layout<String>) -> Vec<String> {
    let mut out = Vec::new();
    let mut visit: Vec<&Node<String>> = layout.root.iter().collect();
    while let Some(node) = visit.pop() {
        if out.len() > craft_ui::docking::MAX_PANELS {
            break;
        }
        match node {
            Node::Split { first, second, .. } => {
                visit.push(first);
                visit.push(second);
            }
            Node::Tabs { panels, active } => out.extend(panels.get(*active).cloned()),
            Node::Dock { columns, .. } => out.extend(columns.iter().flat_map(|c| &c.groups).filter_map(|g| g.active_panel().cloned())),
            _ => {}
        }
    }
    out
}

/// Bring each of `front` back in front of the dock tab group that holds it.
fn restore_front_tabs(layout: &mut Layout<String>, front: &[String]) {
    let mut visit: Vec<&mut Node<String>> = layout.root.iter_mut().collect();
    let mut seen = 0usize;
    while let Some(node) = visit.pop() {
        seen = seen.saturating_add(1);
        if seen > 2 * craft_ui::docking::MAX_PANELS {
            break;
        }
        match node {
            Node::Split { first, second, .. } => {
                visit.push(first);
                visit.push(second);
            }
            Node::Tabs { panels, active } => {
                if let Some(i) = panels.iter().position(|p| front.contains(p)) {
                    *active = i;
                }
            }
            Node::Dock { columns, .. } => {
                for group in columns.iter_mut().flat_map(|c| &mut c.groups) {
                    if let Some(i) = group.panels.iter().position(|p| front.contains(p)) {
                        group.active = i;
                    }
                }
            }
            _ => {}
        }
    }
}

/// Panels InDesign opens together as one floating group from the Window menu, in tab order:
/// Window › Styles › Paragraph Styles (or Character Styles) brings both.
const FLOATING_GROUPS: &[&[&str]] = &[&["characterStyles", "paragraphStyles"]];

/// Window › a panel of a [`FLOATING_GROUPS`] group none of whose panels is showing: the group
/// opens floating over the right of the canvas, with that panel in front. None otherwise (the
/// panel opens as before).
fn open_floating_group(app: &mut DesignApp, params: &Value) -> Option<Result<Value, String>> {
    let panel = params.get("panel").and_then(Value::as_str).and_then(normalize)?;
    let group = FLOATING_GROUPS.iter().find(|g| g.contains(&panel))?;
    // Panels in the workspace's icon groups open from there.
    if app.ui.icon_groups.as_ref().is_some_and(|g| g.iter().flatten().any(|p| group.contains(&p.as_str()))) {
        return None;
    }
    // A panel closed from a place in the dock comes back there.
    if group.iter().any(|p| app.ui.docking_hidden.get(*p).is_some_and(|l| l.floating.is_none())) {
        return None;
    }
    let layout = layout_of(app);
    if group.iter().any(|p| layout.contains(&p.to_string())) {
        return None;
    }
    let (w, h) = (290.0, 340.0);
    let (x, y) = app.canvas_rect.map_or((400.0, 140.0), |r| ((r.right() - w - 24.0).max(r.left()), r.top() + 96.0));
    let before = (app.ui.docking.clone(), app.ui.docking_hidden.clone(), app.ui.floating.clone(), app.ui.open_panel.clone());
    // The dock's tab in front stays in front (opening a panel passes through the dock).
    let front = front_tabs(&layout);
    let open = |app: &mut DesignApp| -> Result<Value, String> {
        let first = group.first().ok_or("empty panel group")?;
        change(app, "float", &json!({"panel": first, "x": x, "y": y, "width": w, "height": h}))?;
        for p in group.get(1..).unwrap_or_default() {
            change(app, "move", &json!({"panel": p, "anchor": first}))?;
        }
        change(app, "activate", &json!({"panel": panel}))
    };
    let result = open(app);
    if result.is_ok()
        && let Some(layout) = app.ui.docking.as_mut()
    {
        restore_front_tabs(layout, &front);
    }
    if result.is_err() {
        // All or nothing.
        (app.ui.docking, app.ui.docking_hidden, app.ui.floating, app.ui.open_panel) = before;
    }
    Some(result)
}

fn change(app: &mut DesignApp, operation: &str, params: &Value) -> Result<Value, String> {
    let panel = params.get("panel").and_then(Value::as_str).and_then(normalize).ok_or("panel must name a registered panel")?;
    if app.ui.docking.as_ref().is_some_and(|layout| !valid(layout)) {
        return Err("saved panel layout is invalid; reset the workspace first".into());
    }
    let mut layout = layout_of(app);
    let mut hidden: std::collections::BTreeMap<_, _> =
        app.ui.docking_hidden.iter().filter(|(id, _)| normalize(id).is_some()).take(64).map(|(id, at)| (id.clone(), at.clone())).collect();
    let mut restored = false;
    if operation == "dock"
        && params.get("anchor").or_else(|| params.get("onto")).is_none()
        && let Some(location) = hidden.get(panel).filter(|location| {
            location.floating.is_none()
                && location.anchor.as_ref().is_none_or(|anchor| Layout { root: layout.root.clone(), floating: Vec::new() }.contains(anchor))
        })
    {
        let mut candidate = layout.clone();
        if candidate.contains(&panel.to_string()) {
            apply(&mut candidate, Action::Close { panel: panel.into() })?;
        }
        if candidate.restore(panel.into(), location).is_ok() {
            layout = candidate;
            restored = true;
            hidden.remove(panel);
        }
    }
    match operation {
        "dock" if restored => {}
        "float" => {
            let rect = [number(params, "x", 400.0)?, number(params, "y", 140.0)?, number(params, "width", 300.0)?, number(params, "height", 400.0)?];
            ensure(&mut layout, panel)?;
            if let Ok(location) = layout.location(&panel.to_string())
                && location.floating.is_none()
            {
                hidden.insert(panel.into(), location);
            }
            apply(&mut layout, Action::Float { panel: panel.into(), rect })?;
        }
        "move" | "dock" => {
            if operation == "move" && params.get("anchor").or_else(|| params.get("onto")).is_none() {
                return Err("move requires a destination panel".into());
            }
            let anchor = match params.get("anchor").or_else(|| params.get("onto")) {
                Some(v) => Some(v.as_str().and_then(normalize).ok_or("anchor must name a registered panel")?.to_string()),
                None => first_dock_panel(&layout, panel),
            };
            if let Some(anchor) = anchor {
                let placement = placement(params, &layout, &anchor, crate::i18n::is_rtl(&app.ui.language))?;
                ensure(&mut layout, panel)?;
                apply(&mut layout, Action::Move { panel: panel.into(), anchor, placement })?;
            } else if operation == "dock" {
                if layout.contains(&panel.to_string()) {
                    apply(&mut layout, Action::Close { panel: panel.into() })?;
                }
                apply(&mut layout, Action::Open { panel: panel.into(), anchor: None })?;
            } else {
                return Err("move requires an existing destination panel".into());
            }
        }
        "close" => {
            hidden.insert(panel.into(), layout.location(&panel.to_string()).map_err(|e| e.to_string())?);
            apply(&mut layout, Action::Close { panel: panel.into() })?;
        }
        _ => {
            if !layout.contains(&panel.to_string())
                && let Some(location) = hidden.get(panel)
            {
                // A missing neighbor is allowed: the normal visible dock becomes the fallback.
                let _ = layout.restore(panel.into(), location);
            }
            ensure(&mut layout, panel)?;
            hidden.remove(panel);
            // Iconic columns open no flyouts yet, so showing a panel there expands its column.
            if in_iconic_column(&layout, panel) {
                apply(&mut layout, Action::SetIconic { panel: panel.into(), iconic: false })?;
            }
        }
    }
    app.ui.docking = Some(layout);
    app.ui.docking_hidden = hidden;
    if operation == "close" {
        release_closed(app, &[panel.to_owned()]);
    }
    keep_float_only(app);
    app.ui.open_panel = None;
    app.ui.docking_opened = if operation == "close" { None } else { Some(panel.to_owned()) };
    app.ui.docking_raise = if operation == "close" { None } else { Some(panel.to_owned()) };
    if operation != "close" {
        app.ui.dock_expanded = true;
    }
    Ok(json!({"panel": panel, "floating": floating_panels(app), "layout": app.ui.docking}))
}

/// Forget what DesignCraft keeps about panels that just closed: the focus, the panel last
/// opened (for `ui.set closePanel`) and a pending raise.
fn release_closed(app: &mut DesignApp, closed: &[String]) {
    for slot in [&mut app.ui.docking_focused, &mut app.ui.docking_opened, &mut app.ui.docking_raise] {
        if slot.as_ref().is_some_and(|panel| closed.contains(panel)) {
            *slot = None;
        }
    }
}

/// Window › Collapse to Icons collapses every dock column; `iconic: false` (the control
/// channel's `dockExpanded`) expands the outer column.
pub(crate) fn set_iconic(app: &mut DesignApp, iconic: bool) -> Result<Value, String> {
    if app.ui.docking.as_ref().is_some_and(|layout| !valid(layout)) {
        return Err("saved panel layout is invalid; reset the workspace first".into());
    }
    let mut layout = layout_of(app);
    let columns = dock_columns(&layout);
    let firsts = |c: &&Column<String>| c.groups.first().and_then(|g| g.panels.first()).cloned();
    let targets: Vec<String> = if iconic {
        columns.iter().filter(|c| !c.iconic).filter_map(firsts).collect()
    } else {
        columns.first().filter(|c| c.iconic).and_then(firsts).into_iter().collect()
    };
    for panel in targets {
        apply(&mut layout, Action::SetIconic { panel, iconic })?;
    }
    app.ui.docking = Some(layout);
    app.ui.dock_expanded = !iconic;
    app.ui.open_panel = None;
    Ok(Value::Null)
}

pub(crate) fn show(app: &mut DesignApp, ui: &mut egui::Ui) {
    let area = egui::Id::new("designcraft-panel-docking");
    let changed_workspace = ui.ctx().data_mut(|data| {
        let previous = data.get_temp::<u64>(area.with("generation"));
        data.insert_temp(area.with("generation"), app.ui.docking_generation);
        previous.is_some_and(|generation| generation != app.ui.docking_generation)
    });
    if changed_workspace {
        craft_ui::docking::cancel_drag::<String>(ui.ctx(), area);
    }
    if app.ui.docking.as_ref().is_some_and(|layout| !valid(layout)) {
        app.ui.docking = None;
        app.status(crate::i18n::tr(&app.ui.language, "Invalid panel layout; restored the default dock"));
    }
    let layout = layout_of(app);
    let raise = app.ui.docking_raise.take();
    let mut blocked = Vec::new();
    if app.ui.dialog.is_some() || app.ui.about || app.ui.palette.is_some() || egui::Popup::is_any_open(ui.ctx()) {
        blocked.push(ui.ctx().content_rect());
    } else if crate::story_editor::visible(app) {
        // The editor is rendered after the dock; block its unmeasured first frame conservatively.
        let rect = ui.ctx().memory(|m| m.area_rect(egui::Id::new("story_editor"))).unwrap_or_else(|| ui.ctx().content_rect());
        blocked.push(rect);
    }
    let style = dock_style(ui, &app.ui.language);
    let mut dock = craft_ui::docking::DockArea::new(area).blocked_rects(&blocked);
    if let Some(panel) = raise {
        dock = dock.raise_panel(panel);
    }
    let mut content = PanelContent(app, Vec::new());
    let output = dock.show_content(ui, &layout, &style, &mut content);
    let floats = content.1;
    if let Some(panel) = output.focused {
        if panel != CANVAS {
            app.ui.docking_opened = Some(panel.clone());
        }
        app.ui.docking_focused = Some(panel);
    }
    if let Some(error) = output.error {
        app.ui.status = error.to_string();
    }
    for action in output.actions.into_iter().chain(floats) {
        if let Err(error) = app.run("window.panel.layout", json!({"action": action})) {
            app.ui.status = error;
        }
    }
}

/// What the control channel reports: the layout in use (craft-ui's serialized `Layout`),
/// whether it is the workspace's default, the panel (or `canvas`) last clicked or selected, and
/// the panels closed with a remembered place.
pub(crate) fn inspect(app: &DesignApp) -> Value {
    json!({
        "layout": layout_of(app),
        "customized": app.ui.docking.is_some(),
        "focused": app.ui.docking_focused,
        "rightToLeft": crate::i18n::is_rtl(&app.ui.language),
        "hidden": app.ui.docking_hidden.keys().collect::<Vec<_>>(),
    })
}

fn dock_style(ui: &egui::Ui, language: &str) -> craft_ui::docking::DockStyle {
    let t = crate::theme::Tokens::get(ui.ctx());
    let mut style = craft_ui::docking::DockStyle::from_ui(ui);
    style.tab_height = 27.0;
    style.background = t.panel;
    style.tab_background = t.tab_strip;
    style.active_background = t.panel;
    style.text = t.text_strong;
    style.inactive_text = t.text_dim;
    style.border = egui::Stroke::new(1.0, t.border);
    style.accent = t.accent;
    style.right_to_left = crate::i18n::is_rtl(language);
    style.localized(|key| DOCK_LABELS.iter().find(|(k, _)| *k == key).map(|(_, english)| crate::i18n::tr(language, english).to_owned()))
}

/// DesignCraft's wording of craft-ui's docking labels, translated through `i18n::tr`.
const DOCK_LABELS: &[(&str, &str)] = &[
    ("dock.float", "Float Panel"),
    ("dock.close", "Close"),
    ("dock.move", "Group with"),
    ("dock.panels", "Panels"),
    ("dock.resize", "Resize panels"),
    ("dock.resizeWindow", "Resize panel window"),
    ("dock.moveGroup", "Move Tab Group"),
    ("dock.closeGroup", "Close Tab Group"),
    ("dock.minimize", "Minimize"),
    ("dock.expand", "Expand"),
    ("dock.floatGroup", "Float Tab Group"),
    ("dock.groupWith", "Group Tab Group with"),
    ("dock.moveColumn", "Move Panel Column"),
    ("dock.collapseToIcons", "Collapse to Icons"),
    ("dock.expandPanels", "Expand Panels"),
    ("dock.moveWindow", "Move Panel Window"),
    ("dock.closeWindow", "Close Panel Window"),
    ("dock.undock", "Undock Panel"),
    ("dock.undockGroup", "Undock Panel Group"),
    ("dock.closePanelGroup", "Close Panel Group"),
    ("dock.closeOthers", "Close Other Panels in Group"),
    ("dock.maximize", "Maximize Frame"),
    ("dock.restoreFrame", "Restore Frame Size"),
    ("dock.newWindow", "Move Panel to New Window"),
];

/// The menu behind a panel's three-line button: the panel's own commands, then its docking
/// commands.
fn three_line_menu(app: &mut DesignApp, ui: &mut egui::Ui, panel: &str) {
    if panel == "pages" {
        crate::panels::pages::panel_menu(app, ui);
        ui.separator();
    }
    panel_menu(app, ui, panel);
}

/// The three-line button at the right of a tab strip, for the panel in front.
fn three_line_button(app: &mut DesignApp, ui: &mut egui::Ui, panel: &str) {
    let t = crate::theme::Tokens::get(ui.ctx());
    let area = ui.max_rect();
    let r = egui::Rect::from_center_size(area.center(), egui::vec2(18.0, 18.0));
    let menu = ui.interact(r, ui.id().with("panel-menu"), egui::Sense::click());
    if menu.hovered() {
        ui.painter().rect_filled(r, 3.0, t.hover);
    }
    crate::icons::paint(ui.painter(), r, "menu", t.icon);
    egui::Popup::menu(&menu).close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside).show(|ui| three_line_menu(app, ui, panel));
}

/// The docking commands of a panel: its tab's context menu and the end of its panel menu.
pub(crate) fn panel_menu(app: &mut DesignApp, ui: &mut egui::Ui, panel: &str) {
    let Some(panel) = normalize(panel) else { return };
    tab_commands(app, ui, panel);
    ui.separator();
    let language = app.ui.language.clone();
    if ui.button(crate::rtl::widget(ui, crate::i18n::tr(&language, "Float Panel"))).clicked() {
        if let Err(error) = app.run("window.panel.float", json!({"panel": panel})) {
            app.status(error);
        }
        ui.close();
    }
    ui.menu_button(crate::rtl::widget(ui, crate::i18n::tr(&language, "Group with")), |ui| {
        egui::ScrollArea::vertical().max_height(360.0).show(ui, |ui| {
            for (anchor, label, _) in crate::dock::DOCK_TABS.iter().chain(crate::dock::ICON_PANELS) {
                if *anchor != panel && ui.button(crate::rtl::widget(ui, crate::i18n::tr(&language, label))).clicked() {
                    // A closed icon panel is made visible before becoming the destination.
                    let result = app
                        .run("window.panel.activate", json!({"panel": anchor}))
                        .and_then(|_| app.run("window.panel.move", json!({"panel":panel,"anchor":anchor})));
                    if let Err(error) = result {
                        app.status(error);
                    }
                    ui.close();
                }
            }
        });
    });
}

/// The size a panel floats at (tab strip included), whatever the size of the dock it left.
fn floating_size(panel: &str) -> egui::Vec2 {
    match panel {
        "properties" | "pages" | "layers" => egui::vec2(300.0, 420.0),
        "characterStyles" | "paragraphStyles" => egui::vec2(290.0, 340.0),
        "glyphs" | "library" | "links" | "preflight" | "dataMerge" => egui::vec2(320.0, 400.0),
        _ => egui::vec2(280.0, 360.0),
    }
}

/// The panels' content, and the floats released over the canvas this frame.
struct PanelContent<'a>(&'a mut DesignApp, Vec<Action<String>>);
impl craft_ui::docking::DockContent<String> for PanelContent<'_> {
    /// A panel or group released over the document area floats there, as in InDesign, instead of
    /// splitting the canvas. The window's edge bands still make new docks and root splits.
    fn resolve_drag(
        &mut self,
        _: &egui::Context,
        style: &craft_ui::docking::DockStyle,
        request: &craft_ui::docking::DropRequest<'_, String>,
    ) -> Option<craft_ui::docking::DropFeedback> {
        let pointer = request.pointer;
        let over_canvas = request.targets.iter().rev().find(|t| t.rect.contains(pointer)).is_some_and(|t| t.anchor == CANVAS);
        let band = style.edge_band.max(style.new_dock_band);
        if !over_canvas || !request.workspace.shrink(band).contains(pointer) {
            return None;
        }
        // A floating window moving live has no float rect; it moves as before.
        if request.whole_group && request.floating_rect.is_none() {
            return None;
        }
        if request.released
            && let Some(r) = request.floating_rect
        {
            let rect = [r.left(), r.top(), r.width(), r.height()];
            let panel = request.panel.clone();
            self.1.push(if request.whole_group {
                Action::MoveGroup { panel, to: Target::Float { rect: Some(rect) } }
            } else {
                Action::Float { panel, rect }
            });
        }
        Some(craft_ui::docking::DropFeedback { preview: request.floating_rect, insertion: None })
    }
    fn body(&mut self, ui: &mut egui::Ui, id: &String) {
        if id == CANVAS {
            let fill = crate::theme::Tokens::get(ui.ctx()).pasteboard;
            ui.painter().rect_filled(ui.max_rect(), 0.0, fill);
            // A bare panel has no tab to report focus through, so the canvas reports its own.
            if ui.input(|i| i.pointer.any_pressed()) && ui.ui_contains_pointer() {
                self.0.ui.docking_focused = Some(CANVAS.to_string());
            }
            self.0.canvas_area(ui, false);
            return;
        }
        egui::Frame::NONE.inner_margin(egui::Margin::same(8)).show(ui, |ui| {
            if crate::dock::scrolls_itself(id) {
                crate::dock::panel_body(self.0, ui, id);
                return;
            }
            egui::ScrollArea::vertical()
                .id_salt(("shared-panel-body", id))
                .auto_shrink([false, false])
                .show(ui, |ui| crate::dock::panel_body(self.0, ui, id));
        });
    }
    fn logical_label(&self, id: &String) -> String {
        let english = crate::dock::DOCK_TABS.iter().chain(crate::dock::ICON_PANELS).find(|p| p.0 == id).map_or(id.as_str(), |p| p.1);
        crate::i18n::tr(&self.0.ui.language, english).to_owned()
    }
    fn permissions(&self, id: &String) -> Permissions {
        model_permissions(id)
    }
    fn limits(&self, id: &String) -> craft_ui::docking::PanelLimits {
        let min = if id == CANVAS { egui::vec2(200.0, 120.0) } else { egui::vec2(220.0, 80.0) };
        craft_ui::docking::PanelLimits { min, ..Default::default() }
    }
    fn floating_size(&self, id: &String) -> Option<egui::Vec2> {
        Some(floating_size(id))
    }
    fn bare(&self, id: &String) -> bool {
        id == CANVAS
    }
    fn group_header_width(&self, _: &[String], _: &String, _: bool) -> f32 {
        24.0
    }
    fn group_header(&mut self, ui: &mut egui::Ui, _: &[String], active: &String, _: bool) {
        if let Some(panel) = normalize(active) {
            three_line_button(self.0, ui, panel);
        }
    }
    fn label(&mut self, ui: &egui::Ui, _: &String, logical: &str, font: &egui::FontId, color: egui::Color32) -> egui::WidgetText {
        egui::WidgetText::Galley(crate::rtl::plain(ui.ctx(), logical, font.clone(), color))
    }
    fn action_label(&mut self, ui: &egui::Ui, logical: &str, font: &egui::FontId, color: egui::Color32) -> egui::WidgetText {
        egui::WidgetText::Galley(crate::rtl::plain(ui.ctx(), logical, font.clone(), color))
    }
}

/// InDesign's panel tab commands at the end of a panel's three-line menu: Close, Close Tab Group,
/// Minimize (an open accordion entry) and Collapse to Icons. A tab's own context menu is
/// craft-ui's, which lists the same commands.
fn tab_commands(app: &mut DesignApp, ui: &mut egui::Ui, panel: &'static str) {
    let language = app.ui.language.clone();
    let tr = |s: &str| crate::i18n::tr(&language, s).to_owned();
    let run = |app: &mut DesignApp, ui: &mut egui::Ui, id: &str, p: Value| {
        if let Err(error) = app.run(id, p) {
            app.status(error);
        }
        ui.close();
    };
    if ui.button(crate::rtl::widget(ui, tr("Close"))).clicked() {
        run(app, ui, "window.panel.close", json!({"panel": panel}));
    }
    if ui.button(crate::rtl::widget(ui, tr("Close Tab Group"))).clicked() {
        run(app, ui, "window.panel.closeGroup", json!({"panel": panel}));
    }
    let layout = layout_of(app);
    let minimizable = open_in_stack(&layout, panel);
    if ui.add_enabled(minimizable, egui::Button::new(crate::rtl::widget(ui, tr("Minimize")))).clicked() {
        run(app, ui, "window.panel.layout", json!({"action": {"SetStackOpen": {"panel": panel, "open": false}}}));
    }
    ui.separator();
    let expanded = dock_columns(&layout).iter().any(|c| !c.iconic);
    if ui.add_enabled(expanded, egui::Button::new(crate::rtl::widget(ui, tr("Collapse to Icons")))).clicked() {
        run(app, ui, "window.collapseToIcons", json!({}));
    }
}

/// Close the last explicitly opened panel through the same validated layout transaction.
pub(crate) fn close_visible(app: &mut DesignApp) -> Result<(), String> {
    if let Some(panel) = app.ui.docking_opened.clone() {
        change(app, "close", &json!({"panel": panel}))?;
    }
    app.ui.open_panel = None;
    Ok(())
}

#[cfg(test)]
#[path = "panel_docking_tests.rs"]
mod tests;
