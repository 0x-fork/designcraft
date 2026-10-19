//! Keyboard shortcuts on `craft-keymap`: the catalogue (UI commands, engine commands, panels and
//! tools with their default keys and menu paths), shortcut text, and the edits that
//! `window.setShortcut`, the Keyboard Shortcuts dialog and the shortcut-set importer make.

use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use craft_keymap::{Bindable, Binding, Chord, CommandRef, Key, Keymap, Kind, Mods, Overrides, Platform, Policy, Scope, Scopes};
use serde_json::{Value, json};

use crate::DesignApp;
use crate::menus::{self, Item};

/// The context bindings such as `Text: Cmd+B` name; active while text is being typed on the canvas.
pub const TEXT_CONTEXT: &str = "Text";

/// The command that opens a panel; a binding on it carries the panel id.
pub const PANEL_COMMAND: &str = "window.panel";
/// The command that selects a tool; a binding on it carries the tool id.
pub const TOOL_COMMAND: &str = "tool.select";

/// InDesign's default keys for the panels DesignCraft has, where DesignCraft's own defaults leave
/// the key free.
const PANEL_KEYS: &[(&str, &str)] = &[
    ("swatches", "F5"),
    ("color", "F6"),
    ("layers", "F7"),
    ("info", "F8"),
    ("stroke", "F10"),
    ("paragraphStyles", "F11"),
    ("characterStyles", "Shift+F11"),
    ("pages", "F12"),
    ("align", "Shift+F7"),
];

/// Other names shortcut sets use for a command (InDesign's feature names), by command id.
const IMPORT_ALIASES: &[(&str, &str)] = &[
    ("app.quit", "Exit"),
    ("app.preferences", "Preferences: General…"),
    ("app.insertTableDialog", "Insert Table"),
    ("view.togglePresentation", "Presentation"),
    ("color.defaultFillStroke", "Apply Default Fill and Stroke Colors"),
    ("color.swapFillStroke", "Swap Fill and Stroke Colors"),
    ("view.togglePreview", "Toggle View Setting Between Default and Preview"),
    ("type.bold", "Apply Bold"),
    ("type.italic", "Apply Italic"),
    ("window.hidePanels", "Show/Hide All Panels"),
    ("window.hidePanelsExceptTools", "Show/Hide All Panels Except Toolbox"),
    ("window.nextDocument", "Next Window"),
    ("window.previousDocument", "Previous Window"),
];

/// Commands that show and hide something without "Show/Hide" in their label. A shortcut set may
/// name a toggle by what it does now (`Hide Hidden Characters`); the importer matches such names
/// only to toggles.
const TOGGLES: &[&str] = &["view.hiddenCharacters", "view.tagMarkers"];

/// Extra menu paths for import, where an alias would leave two commands on one name: InDesign's
/// Tools area has Apply Color, which is also `object.color`'s label.
const IMPORT_PATHS: &[(&str, &[&str])] = &[("color.applyColor", &["Tools", "Apply Color"])];

/// Commands that open a panel, by panel id. The panel's binding gets the command's default key
/// too, so the two are twins: a shortcut set that gives either one the key leaves both with it.
const PANEL_TWINS: &[(&str, &str)] = &[
    ("character", "app.characterPanel"),
    ("paragraph", "app.paragraphPanel"),
    ("glyphs", "app.glyphsPanel"),
    ("characterStyles", "app.characterStylesPanel"),
    ("paragraphStyles", "app.paragraphStylesPanel"),
    ("table", "app.tablePanel"),
    ("textWrap", "object.textWrap"),
];

pub fn panel(id: &str) -> CommandRef {
    CommandRef::with_params(PANEL_COMMAND, json!({ "panel": id }))
}

pub fn tool(id: &str) -> CommandRef {
    CommandRef::with_params(TOOL_COMMAND, json!({ "tool": id }))
}

/// The command a menu row or a `window.setShortcut` call names.
pub fn command_ref(id: &str, params: &Value) -> CommandRef {
    match params {
        Value::Null => CommandRef::new(id),
        Value::Object(m) if m.is_empty() => CommandRef::new(id),
        p => CommandRef::with_params(id, p.clone()),
    }
}

/// The keymap with the user's overrides.
pub fn new_keymap(overrides: Overrides) -> Keymap {
    Keymap::new(catalogue().to_vec(), Scopes::new().with_context(TEXT_CONTEXT), Platform::current()).with_overrides(overrides)
}

/// Every bindable command, in dispatch order: where two share keys the earlier one runs. UI
/// commands come first (⌘N is the New Document dialog, not a bare `file.new`), then engine
/// commands, panels and tools.
pub fn catalogue() -> &'static [Bindable] {
    static CATALOGUE: OnceLock<Vec<Bindable>> = OnceLock::new();
    CATALOGUE.get_or_init(build_catalogue)
}

fn build_catalogue() -> Vec<Bindable> {
    let paths = menu_paths();
    // An engine command's own menu placement counts only where no menu row stands: a row there
    // runs a UI command in its place (File › Save is the Save dialog when needed), and two
    // commands on one path leave the importer unable to pick either.
    let normalized = |path: &[String]| path.iter().map(|s| craft_keymap::import::normalize_label(s)).collect::<Vec<_>>();
    let rows: HashSet<Vec<String>> = paths.values().flatten().map(|p| normalized(p)).collect();
    let app = Scope::app();
    let mut out = Vec::new();
    let mut add = |command: CommandRef, label: &str, defaults: &[&str], fallback_path: Option<Vec<String>>, kind: Kind| {
        let mut b = Bindable::new(command, label, app.clone()).with_kind(kind);
        match paths.get(&b.command.key()) {
            Some(found) => b.paths.extend(found.iter().cloned()),
            None => b.paths.extend(fallback_path.filter(|p| !rows.contains(&normalized(p)))),
        }
        for binding in defaults.iter().filter_map(|d| Binding::parse(d).ok()) {
            b = b.with_default(binding);
        }
        out.push(b);
    };
    for (id, label, _, _) in menus::UI_COMMANDS {
        add(CommandRef::new(*id), label, menus::default_shortcut(id).as_slice(), None, Kind::Command);
    }
    for c in designcraft_engine::command_specs() {
        let path = (!c.menu.is_empty()).then(|| c.menu.iter().map(|s| s.to_string()).chain([c.label.to_string()]).collect());
        add(CommandRef::new(c.id), c.label, c.shortcut.as_slice(), path, Kind::Command);
    }
    let mut seen = Vec::new();
    for (id, label, _) in crate::dock::DOCK_TABS.iter().chain(crate::dock::ICON_PANELS) {
        if seen.contains(id) {
            continue;
        }
        seen.push(id);
        let own = PANEL_KEYS.iter().find(|(p, _)| p == id).map(|(_, k)| *k);
        let twin = PANEL_TWINS.iter().find(|(p, _)| p == id).and_then(|(_, c)| menus::default_shortcut(c));
        let defaults: Vec<&str> = own.into_iter().chain(twin).collect();
        add(panel(id), label, &defaults, Some(vec!["Window".into(), (*label).into()]), Kind::Command);
    }
    for group in designcraft_tools::TOOL_GROUPS {
        let Some(first) = group.first() else { continue };
        for t in group.iter() {
            let kind = Kind::Tool { group: Some(first.id.to_string()) };
            add(tool(t.id), t.label, t.shortcut.as_slice(), Some(vec!["Tools".into(), t.label.into()]), kind);
        }
    }
    for b in &mut out {
        if !b.command.params.is_null() {
            continue;
        }
        let id = b.command.id.as_str();
        b.is_toggle = b.label.starts_with("Show/Hide ") || TOGGLES.contains(&id);
        b.aliases.extend(IMPORT_ALIASES.iter().filter(|(c, _)| *c == id).map(|(_, a)| a.to_string()));
        b.paths.extend(IMPORT_PATHS.iter().filter(|(c, _)| *c == id).map(|(_, p)| p.iter().map(|s| s.to_string()).collect()));
    }
    out
}

/// Menu paths (menu, submenus, label) of every menu row, by command key.
fn menu_paths() -> HashMap<String, Vec<Vec<String>>> {
    fn walk(items: &[Item], path: &mut Vec<String>, out: &mut HashMap<String, Vec<Vec<String>>>) {
        for it in items {
            match it {
                Item::Sub(name, children) => {
                    path.push(name.clone());
                    walk(children, path, out);
                    path.pop();
                }
                Item::Cmd { label, id, params, .. } => {
                    let mut full = path.clone();
                    full.push(label.clone());
                    out.entry(command_ref(id, params).key()).or_default().push(full);
                }
                Item::Sep => {}
            }
        }
    }
    let mut out = HashMap::new();
    for (menu, entries) in menus::menu_tree() {
        walk(&entries, &mut vec![menu.to_string()], &mut out);
    }
    out
}

/// The binding a command shows: the first in its home scope, else its first.
fn shown_binding(keymap: &Keymap, cmd: &CommandRef) -> Option<Binding> {
    let bindings = keymap.bindings(cmd);
    bindings.iter().find(|b| b.scope.is_none()).or_else(|| bindings.first()).cloned()
}

/// The shortcut in effect for a command, as stored and sent ("Cmd+Alt+K", "Text: F7").
pub fn shortcut(keymap: &Keymap, cmd: &CommandRef) -> Option<String> {
    let b = shown_binding(keymap, cmd)?;
    let home = keymap.bindable(cmd).map_or_else(Scope::app, |c| c.scope.clone());
    Some(b.text(&home))
}

/// The shortcut in effect for a command, for menus and lists (⌥⌘K on macOS, Ctrl+Alt+K elsewhere).
pub fn shortcut_display(keymap: &Keymap, cmd: &CommandRef) -> Option<String> {
    shown_binding(keymap, cmd).map(|b| b.trigger.display(keymap.platform()))
}

/// A key press as shortcut text ("Cmd+Shift+K"); `None` for a key the keymap has no name for.
pub fn chord_text(m: egui::Modifiers, key: egui::Key) -> Option<String> {
    let key = Key::from_egui(key)?;
    Some(Chord::new(Mods::from_egui(&m, Platform::current()), key).to_string())
}

/// Copies the keymap's overrides to the saved UI state.
pub fn store(app: &mut DesignApp) {
    if app.ui.shortcuts != *app.keymap.overrides() {
        app.ui.shortcuts = app.keymap.overrides().clone();
    }
}

/// Gives a command keys (replacing its others), no keys (`""`), or its defaults back (`None`).
/// Other commands on the same keys keep them unless `policy` is [`Policy::Steal`]. Returns the
/// commands it took the keys from.
pub fn set(app: &mut DesignApp, cmd: &CommandRef, shortcut: Option<&str>, policy: Policy) -> Result<Vec<CommandRef>, String> {
    app.sync_keymap();
    if app.keymap.bindable(cmd).is_none() {
        return Err(format!("unknown command `{cmd}`"));
    }
    let mut taken = Vec::new();
    match shortcut.map(str::trim) {
        None => {
            app.keymap.reset(cmd);
        }
        Some("") => {
            if !app.keymap.clear(cmd) {
                return Err("too many changed shortcuts".into());
            }
        }
        Some(sc) => {
            let binding = Binding::parse(sc).map_err(|e| format!("can't read shortcut `{sc}`: {e}"))?;
            let done = app.keymap.set_bindings(cmd, vec![binding], policy).map_err(|e| format!("can't assign `{sc}`: {e}"))?;
            taken = done.taken_from.into_iter().map(|h| h.command).collect();
        }
    }
    store(app);
    Ok(taken)
}

/// Other commands on the keys `cmd` holds, by command key.
pub fn conflicts(keymap: &Keymap, cmd: &CommandRef) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for c in keymap.conflicts_with(cmd) {
        let key = c.b.0.key();
        if !out.contains(&key) {
            out.push(key);
        }
    }
    out
}

/// A command's label, by its key.
pub fn label(keymap: &Keymap, key: &str) -> String {
    CommandRef::parse_key(key).and_then(|c| keymap.bindable(&c).map(|b| b.label.clone())).unwrap_or_else(|| key.to_string())
}
