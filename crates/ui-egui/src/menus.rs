//! The menu bar tree (InDesign order), UI-only commands, keyboard shortcuts and Quick Apply.

use serde_json::{Value, json};

use crate::{DesignApp, ScreenMode};

/// Menu and Quick Apply entries that only CJK typesetting uses: hidden unless
/// [`crate::cjk_features`]. The commands still run from scripts, the control channel and MCP.
pub const CJK_ONLY: &[&str] = &["type.tateChuYoko", "app.rubyDialog", "type.kenten", "type.warichu"];

/// Whether command `id` has a menu item or Quick Apply entry in the current interface (CJK-only
/// ones need [`crate::cjk_features`]).
pub fn shown(app: &DesignApp, id: &str) -> bool {
    !CJK_ONLY.contains(&id) || crate::cjk_features(app)
}

/// UI commands: (id, label, shortcut, params).
pub const UI_COMMANDS: &[(&str, &str, Option<&str>, &str)] = &[
    ("app.newDocumentDialog", "New Document…", Some("Cmd+N"), "{}"),
    ("app.openDialog", "Open…", Some("Cmd+O"), "{}"),
    ("app.openRecent", "Open Recent File", None, "{index} — file.openRecent, with an alert when the file can't be opened"),
    ("app.placeDialog", "Place…", Some("Cmd+D"), "{}"),
    ("app.saveDialog", "Save As…", Some("Cmd+Shift+S"), "{}"),
    ("app.save", "Save", Some("Cmd+S"), "{}"),
    ("app.quit", "Quit DesignCraft", Some("Cmd+Q"), "{} — close the window; each document with unsaved changes asks first"),
    ("app.saveCopyDialog", "Save a Copy…", Some("Cmd+Alt+S"), "{} — choose where to write a copy (the document stays as it is)"),
    ("app.loadSwatches", "Load Swatches…", None, "{} — pick a swatch exchange (.ase) file"),
    (
        "app.loadParagraphStyles",
        "Load Paragraph Styles…",
        None,
        "{} — pick a document (.designcraft or .idml) and choose which of its paragraph styles to load (style.load)",
    ),
    ("app.loadCharacterStyles", "Load Character Styles…", None, "{} — the same for its character styles"),
    ("app.loadTextStyles", "Load All Text Styles…", None, "{} — the same for its paragraph and character styles"),
    (
        "effects.dialog",
        "Effects…",
        None,
        "{page?: transparency|dropShadow|innerShadow|outerGlow|innerGlow|bevel|satin|feather|directionalFeather|gradientFeather} — the Effects dialog for the selected objects",
    ),
    ("app.packageDialog", "Package…", Some("Cmd+Alt+Shift+P"), "{} — choose the package folder to create (file.package)"),
    ("app.saveSwatches", "Save Swatches for Exchange…", None, "{path?} — write the colour swatches as .ase"),
    ("app.exportPng", "Export Page as PNG…", Some("Cmd+E"), "{}"),
    ("app.exportIdml", "Export IDML…", None, "{path?} — InDesign Markup (IDML) package"),
    (
        "app.exportEpub",
        "Export EPUB…",
        None,
        "{path?} — reflowable EPUB 3 (with the first page as its cover); without a path, EPUB Export Options (Split Document) and then the save dialog",
    ),
    (
        "app.exportInteractivePdf",
        "Export PDF (Interactive)…",
        None,
        "{path?, fullScreen?, advanceSeconds?} — single pages fitted, bookmarks panel open, buttons, forms, transitions",
    ),
    ("app.exportFixedEpub", "Export EPUB (Fixed Layout)…", None, "{path?} — pre-paginated EPUB 3"),
    ("app.exportHtml", "Export HTML…", None, "{path?} — one self-contained web page"),
    ("app.exportPptx", "Export PowerPoint…", None, "{path?} — every page as a slide of editable shapes, pictures and text"),
    ("app.exportXml", "Export XML…", None, "{path?} — the tagged content"),
    ("app.printBooklet", "Print Booklet…", None, "{path?} — saddle-stitched printer spreads as PDF"),
    ("app.qrCode", "Generate QR Code…", None, "{} — the QR Code dialog (object.qrCode does the work)"),
    ("app.userDictionary", "User Dictionary…", None, "{} — hyphenation exceptions (hyphenation.* commands)"),
    ("app.fittingOptions", "Frame Fitting Options…", None, "{} — the dialog for object.fittingOptions"),
    ("app.menus", "Menus…", None, "{} — show or hide menu items"),
    ("window.hideMenuItem", "Hide Menu Item", None, "{item: \"Menu/Label\", hidden?: bool}"),
    ("edit.dynamicSpelling", "Dynamic Spelling", None, "{on?: bool} — underline misspelled words on the canvas"),
    (
        "app.language",
        "Interface Language",
        None,
        "{lang?: \"\"|de|fr|es|ja|zh|ar|pt-br|it|uk|ru|cs|pl|ka} — menus and panel names (the macOS menu bar follows at once); without lang, opens Preferences, where it is the first setting",
    ),
    (
        "app.flattener",
        "Transparency Flattener Presets",
        None,
        "{preset: \"\"|high|medium|low} — PDF export rasterises spreads with transparency at 300 / 150 / 72 ppi",
    ),
    ("app.graphicCell", "Convert Cell to Graphic Cell…", None, "{} — pick an image for the target table cell"),
    ("app.placeAndLink", "Place and Link", None, "{} — a linked copy of the selected frame's story, beside it"),
    ("app.placeWithOptions", "Place with Import Options…", Some("Cmd+Shift+D"), "{} — Word/RTF style mapping before placing"),
    ("app.exportText", "Export Text…", None, "{path?} — the story being edited, as Text Only (.txt) or Rich Text Format (.rtf)"),
    ("app.exportPdf", "Export PDF…", None, "{path?, …file.exportPdf options} — asks for a path when none is given"),
    ("app.palette", "Quick Apply…", Some("Cmd+Return"), "{} — search styles and commands"),
    ("app.preferences", "Preferences…", Some("Cmd+K"), "{}"),
    ("window.uiScale", "UI Scaling", None, "{scale: 0.5–3 (1 = 100%)} — the size of the whole interface"),
    (
        "app.keyboardShortcuts",
        "Keyboard Shortcuts…",
        None,
        "{command?, params?} — view and change the shortcut of any command, and the shortcut sets; with command, the dialog opens on it",
    ),
    (
        "window.shortcutSets",
        "Shortcut Sets",
        None,
        "{select?: name} — the shortcut sets (Default is built in; Edit › Keyboard Shortcuts makes the others); select makes one active → {active, sets, builtIn}",
    ),
    ("window.richBlack", "Appearance of Black", None, "{on: bool} — show 100% black as rich black on screen (off: accurately)"),
    (
        "window.setShortcut",
        "Set Shortcut",
        None,
        "{id, params?, shortcut: \"Cmd+Alt+J\" | \"\" (none) | null (default), steal?} — id is a command; a panel is id \"window.panel\" with params {panel: \"layers\"}, a tool is id \"tool.select\" with params {tool: \"pen\"} (or its tool id, \"pen\"); steal: true takes the keys from the commands holding them → {shortcut, conflicts: [command keys: ids, or `window.panel {\"panel\":\"layers\"}`]}",
    ),
    ("window.resetShortcuts", "Reset Shortcuts", None, "{} — every command of the active shortcut set back to its default shortcut"),
    (
        "window.importShortcuts",
        "Import InDesign Shortcut Set",
        None,
        "{path | text} — a set saved by InDesign's Show Set (.txt), imported as a new shortcut set, which becomes active: matched commands, panels (Window menu items) and tools (Tools items) take its Default shortcuts (an item without Default keys takes its Text keys), or none when every item matched to them has none; other commands lose keys it takes unless they share them as defaults → {set, imported, changed [{id, label, item, line, from, to}], conflictsCleared [{id, label, shortcut}], skipped [{id, item, line, shortcut, takenBy, reason}] (an earlier item whose keys a later one took, or a reserved key), failed [{id, label, change, shortcuts}], unmatched [{item, line, keys, reason: noMatch | ambiguous | tool | context}], unmatchedWithoutShortcut, unreadable [{item, line, keys, reason}], approximated [{item, line, keys, note}] (numeric keypad keys bound as the main keys), suggestions [{item, line, candidates}] (never applied), matchedWithoutChange [{id, label, item, line, reason}] (a keyless panel-menu item that clears nothing), truncated, shortcutSet (the new set's name)}; ids are command keys (`window.panel {\"panel\":\"layers\"}`)",
    ),
    ("app.findChange", "Find/Change…", Some("Cmd+F"), "{}"),
    ("app.insertTableDialog", "Create Table…", None, "{} — Insert Table dialog (body/header/footer rows, columns)"),
    (
        "app.cellOptionsDialog",
        "Strokes and Fills…",
        None,
        "{} — Cell Options › Strokes and Fills of the target cells: edge proxy, stroke weight, type, colour, tint, gap colour, gap tint, and cell fill (table.setCell)",
    ),
    ("app.footnoteOptionsDialog", "Document Footnote Options…", None, "{} — numbering, formatting and layout of footnotes"),
    ("app.rubyDialog", "Ruby…", None, "{} — the reading set over the selected text"),
    (
        "app.paragraphRulesDialog",
        "Paragraph Rules…",
        None,
        "{rule?: ruleAbove|ruleBelow} — Rule Above / Rule Below of the selected paragraphs (type.para)",
    ),
    (
        "app.paragraphBordersDialog",
        "Paragraph Borders and Shading…",
        None,
        "{tab?: border|shading} — border and shading of the selected paragraphs (type.para)",
    ),
    ("app.spanColumnsDialog", "Span Columns…", None, "{} — span or split columns of the selected paragraphs (type.para)"),
    ("app.keepOptionsDialog", "Keep Options…", None, "{} — Keep with Next and Keep Lines Together of the selected paragraphs (type.para)"),
    (
        "app.nestedStylesDialog",
        "Drop Caps and Nested Styles…",
        None,
        "{} — drop cap, nested styles and nested line styles of the selected paragraphs (type.para)",
    ),
    ("app.grepStylesDialog", "GREP Styles…", None, "{} — GREP styles of the selected paragraphs (type.para)"),
    ("app.bulletsNumberingDialog", "Bullets and Numbering…", None, "{} — bullets or numbers of the selected paragraphs (type.para)"),
    (
        "app.hyphenationDialog",
        "Hyphenation…",
        None,
        "{} — Hyphenation Settings of the selected paragraphs (type.para): limits, zone, Better Spacing ↔ Fewer Hyphens, Preview",
    ),
    (
        "app.justificationDialog",
        "Justification…",
        Some("Cmd+Alt+Shift+J"),
        "{} — word and letter spacing, glyph scaling and composer of the selected paragraphs (type.para)",
    ),
    ("app.findFontDialog", "Find/Replace Font…", None, "{} — fonts used (missing ones flagged) and replacing them"),
    ("app.insertXrefDialog", "Insert Cross-Reference…", None, "{} — New Cross-Reference dialog (paragraph or text anchor, format)"),
    ("app.deleteAllGuides", "Delete All Guides on Spread", None, "{} — the spread in view"),
    ("app.deletePage", "Delete Page", None, "{} — deletes the page in view"),
    ("app.duplicateSpread", "Duplicate Spread", None, "{} — duplicates the spread in view (pages and items)"),
    ("app.tablePanel", "Table Panel", Some("Shift+F9"), "{}"),
    ("app.tabsPanel", "Tabs", Some("Cmd+Shift+T"), "{} — show or hide the Tabs panel (it opens above the selected text frame)"),
    ("app.characterPanel", "Character", Some("Cmd+T"), "{} — the Character panel"),
    ("app.paragraphPanel", "Paragraph", Some("Cmd+Alt+T"), "{} — the Paragraph panel"),
    ("app.glyphsPanel", "Glyphs", Some("Alt+Shift+F11"), "{} — the Glyphs panel"),
    ("app.characterStylesPanel", "Character Styles", Some("Cmd+Shift+F11"), "{} — the Character Styles panel"),
    ("app.paragraphStylesPanel", "Paragraph Styles", Some("Cmd+F11"), "{} — the Paragraph Styles panel"),
    ("app.insertCurrentPageNumber", "Current Page Number", Some("Cmd+Alt+Shift+N"), "{} — the Current Page Number marker at the insertion point"),
    ("app.storyEditor", "Edit in Story Editor", Some("Cmd+Y"), "{story?}"),
    ("view.zoomIn", "Zoom In", Some("Cmd+="), "{}"),
    ("view.zoomOut", "Zoom Out", Some("Cmd+-"), "{}"),
    ("view.fitPage", "Fit Page in Window", Some("Cmd+0"), "{}"),
    ("view.fitSelection", "Fit Selection in Window", Some("Cmd+Alt+="), "{}"),
    ("view.fitSpread", "Fit Spread in Window", Some("Cmd+Alt+0"), "{}"),
    ("view.actualSize", "Actual Size", Some("Cmd+1"), "{}"),
    ("view.entirePasteboard", "Entire Pasteboard", Some("Cmd+Alt+Shift+0"), "{}"),
    ("view.zoom", "Zoom To", None, "{zoom: 1.0 = 100%}"),
    ("view.screenMode", "Screen Mode", None, "{mode: normal|preview|bleed|slug|presentation}"),
    ("view.togglePreview", "Toggle Normal/Preview", Some("W"), "{}"),
    (
        "view.togglePresentation",
        "Toggle Presentation",
        Some("Shift+W"),
        "{} — enters Presentation, or returns to the mode it was entered from; Esc also leaves it",
    ),
    (
        "app.formattingAffectsText",
        "Formatting Affects Text",
        Some("J"),
        "{on?: bool} — with text frames selected, colours go to their text (on) or the frames (off); toggles without `on`",
    ),
    (
        "color.defaultFillStroke",
        "Default Fill and Stroke",
        Some("D"),
        "{} — the selection gets no fill and a black stroke (object.defaultFillStroke); text colour edits (Formatting Affects Text) get a black fill and no stroke",
    ),
    (
        "color.swapFillStroke",
        "Swap Fill and Stroke",
        Some("Shift+X"),
        "{} — each selected object's fill and stroke colours (and tints) trade places (object.swapFillStroke), or the text's with Formatting Affects Text",
    ),
    ("color.toggleFillStroke", "Toggle Fill and Stroke", Some("X"), "{} → {strokeFront} — which of fill and stroke the proxy has in front"),
    (
        "color.applyNone",
        "Apply None",
        Some("/"),
        "{} — [None] for the fill or stroke in front of the proxy, of the objects or the text (Formatting Affects Text)",
    ),
    ("color.applyColor", "Apply Color", Some(","), "{} — the colour applied last ([Black] at first) to the fill or stroke in front of the proxy"),
    (
        "color.applyGradient",
        "Apply Gradient",
        Some("."),
        "{} — the gradient applied last (else the document's first, else white to black) to the fill or stroke in front of the proxy",
    ),
    ("view.frameEdges", "Show/Hide Frame Edges", Some("Cmd+H"), "{}"),
    ("view.rulers", "Show/Hide Rulers", Some("Cmd+R"), "{}"),
    ("view.guides", "Show/Hide Guides", Some("Cmd+;"), "{}"),
    ("view.lockGuides", "Lock Guides", Some("Cmd+Alt+;"), "{on?: bool} — ruler guides can't be dragged; toggles without `on`"),
    ("view.snapToGuides", "Snap to Guides", Some("Cmd+Shift+;"), "{}"),
    ("view.snapToDocumentGrid", "Snap to Document Grid", None, "{}"),
    ("view.smartGuides", "Smart Guides", None, "{}"),
    (
        "view.snapPreferences",
        "Smart Guide Options",
        None,
        "{alignEdges?, alignCenters?, smartDimensions?, smartSpacing?: bool, zone?: px} — what smart guides snap to and how close; returns the current values",
    ),
    ("view.baselineGrid", "Show/Hide Baseline Grid", Some("Cmd+Alt+'"), "{}"),
    ("view.documentGrid", "Show/Hide Document Grid", Some("Cmd+'"), "{}"),
    ("view.textThreads", "Show/Hide Text Threads", Some("Cmd+Alt+Y"), "{}"),
    ("view.hiddenCharacters", "Show Hidden Characters", Some("Cmd+Alt+I"), "{}"),
    ("view.taggedFrames", "Show/Hide Tagged Frames", None, "{} — XML-tagged frames outlined in their tag colour"),
    ("window.hidePanels", "Show/Hide Panels", Some("Tab"), "{} — every panel, the Tools panel included"),
    ("window.hidePanelsExceptTools", "Show/Hide Panels Except Tools", Some("Shift+Tab"), "{}"),
    ("window.nextDocument", "Next Document", Some("Cmd+F6"), "{}"),
    ("window.previousDocument", "Previous Document", Some("Cmd+Shift+F6"), "{}"),
    ("app.layerOptions", "Object Layer Options…", None, "{} — show or hide the layers of the selected placed PDF"),
    (
        "app.layerOptionsDialog",
        "Layer Options…",
        None,
        "{id?} (default: the active layer) — name, colour, show, lock, print, guides, text wrap (layer.set)",
    ),
    ("app.swatchOptionsDialog", "Swatch Options…", None, "{name} — name, colour type, mode and values, tint (swatch.options)"),
    ("app.colorGroupOptionsDialog", "Color Group Options…", None, "{name} — rename a colour group (swatch.renameColorGroup)"),
    ("view.tagMarkers", "Show Tag Markers", None, "{on?: bool} — brackets around inline-tagged text"),
    ("view.flattenerPreview", "Flattener Preview", None, "{on?: bool} — highlight objects that involve transparency"),
    ("view.rotateSpread", "Rotate Spread", None, "{angle: 90 (clockwise) | -90 | 180 | 0 (clear)} — turn the view in quarter turns"),
    ("view.proofColors", "Proof Colors", None, "{on?: bool} — simulate the proof target on screen"),
    (
        "view.proofSetup",
        "Proof Setup",
        None,
        "{target?: workingCmyk|srgb|legacyMacRgb|monitorRgb|protanopia|deuteranopia|cmyk:<profile>, intent?, simulatePaper?: bool|\"toggle\"}",
    ),
    ("app.colorSettings", "Color Settings…", None, "{} — the working spaces, intent and black-point compensation"),
    (
        "view.separations",
        "Separations Preview",
        None,
        "{on?: bool, plate?: all|cyan|magenta|yellow|black|spot ink name|null (null: all inks, and off unless an ink limit stays), inkLimit?: percent|null} — the plates of the PDF Export PDF writes → {on, plate, inkLimit, spread, inks: [{name, spot}], dpi, warning?, error?}",
    ),
    ("view.overprintPreview", "Overprint Preview", Some("Cmd+Alt+Shift+Y"), "{} — show how overprinting inks mix"),
    ("view.fastDisplay", "Fast Display", Some("Cmd+Alt+Shift+Z"), "{} — placed graphics as grey boxes, no effects"),
    ("view.typicalDisplay", "Typical Display", Some("Cmd+Alt+Z"), "{} — low-resolution image proxies"),
    ("view.highQualityDisplay", "High Quality Display", Some("Cmd+Alt+H"), "{} — full-resolution images"),
    ("view.firstPage", "First Page", Some("Cmd+Shift+PageUp"), "{} — scroll to the document's first page (zoom kept)"),
    ("view.previousPage", "Previous Page", Some("Shift+PageUp"), "{} — scroll to the page before the one in view"),
    ("view.nextPage", "Next Page", Some("Shift+PageDown"), "{} — scroll to the page after the one in view"),
    ("view.lastPage", "Last Page", Some("Cmd+Shift+PageDown"), "{} — scroll to the document's last page"),
    ("view.nextSpread", "Next Spread", Some("Alt+PageDown"), "{} — centre the spread after the one in view"),
    ("view.previousSpread", "Previous Spread", Some("Alt+PageUp"), "{} — centre the spread before the one in view"),
    ("view.goToPage", "Go to Page…", Some("Cmd+J"), "{page: number (position) | \"name\" (section page name, or \"+n\" for a position)}"),
    ("view.goBack", "Go Back", Some("Cmd+PageUp"), "{} — show the page viewed before this one in the window"),
    ("view.goForward", "Go Forward", Some("Cmd+PageDown"), "{} — show the page viewed after this one (after Go Back)"),
    (
        "window.panel",
        "Show Panel",
        None,
        "{panel: properties|pages|layers|swatches|paragraphStyles|characterStyles|stroke|character|paragraph|textWrap|links|table|preflight}",
    ),
    ("window.floatPanel", "Float Panel", None, "{panel, x?, y?, width?, height?} — the panel in its own floating window (window.panel.float)"),
    (
        "window.panel.layout",
        "Change Panel Layout",
        None,
        "{action} — serialized docking action, including floating geometry, divider sizes and accordion state",
    ),
    (
        "window.panel.move",
        "Move Panel",
        None,
        "{panel, anchor, zone?: tab|center|left|right|top|bottom|above|below, before?: panel|null, group?, placement?} — group, split or reorder a panel (group: its whole tab group)",
    ),
    (
        "window.panel.float",
        "Float Panel",
        None,
        "{panel, x?, y?, width?, height?, group?, column?} — detach a panel (or its group, or its dock column) into a floating window",
    ),
    (
        "window.panel.dock",
        "Dock Panel",
        None,
        "{panel, anchor?, zone?, before?, group?, column?, side?: Start|End} — dock a floating panel: beside anchor, in a new column at side, or back where it floated from",
    ),
    ("window.panel.activate", "Activate Panel", None, "{panel} — show and select a panel in its current group"),
    ("window.panel.close", "Close Panel", None, "{panel} — hide a panel"),
    ("window.panel.closeGroup", "Close Tab Group", None, "{panel} — hide the panel and the panels tabbed with it"),
    ("window.panel.closeOthers", "Close Other Panels in Group", None, "{panel} — hide the panels tabbed with it"),
    ("window.panel.minimize", "Minimize", None, "{panel, minimized?} — minimize the panel's group to its tab strip (toggles without minimized)"),
    ("window.panel.iconic", "Collapse to Icons", None, "{panel, iconic?} — collapse the panel's dock column to icons (toggles without iconic)"),
    (
        "window.panel.window",
        "Move Panel to New Window",
        None,
        "{panel, native?} — not available: a floating panel has its own window on the desktop and stays in the page on the web",
    ),
    ("window.workspace.new", "New Workspace", None, "{name} — save the current workspace (window.newWorkspace)"),
    ("window.workspace.delete", "Delete Workspace", None, "{name} — delete a saved workspace (window.deleteWorkspace)"),
    ("window.workspace.reset", "Reset Workspace", None, "{} — back to the current workspace as saved (window.resetWorkspace)"),
    ("window.workspace.lock", "Lock Workspace", None, "{locked?} — not available in DesignCraft"),
    (
        "ui.dock",
        "Change Panel Layout",
        None,
        "{action} | {operation: open|close|activate|float|moveFloating|move|setStackOpen|resizeStack|resizeSplit, …} — craft-ui's shared ui.dock shape",
    ),
    ("ui.dock.reset", "Reset Workspace", None, "{} — window.workspace.reset"),
    ("window.collapseToIcons", "Collapse to Icons", None, "{} — the panel dock shrinks to its column of icons"),
    ("window.dockPanel", "Dock Panel", None, "{panel} — back to its place in the dock (window.panel.dock)"),
    ("window.controlBar", "Control", Some("Cmd+Alt+6"), "{}"),
    // Object › Fitting with InDesign's shortcuts (one `object.fit` mode each).
    ("fit.fillFrameProportionally", "Fill Frame Proportionally", Some("Cmd+Alt+Shift+C"), "{} — object.fit {mode: fillProportionally}"),
    ("fit.contentProportionally", "Fit Content Proportionally", Some("Cmd+Alt+Shift+E"), "{} — object.fit {mode: fitProportionally}"),
    ("fit.frameToContent", "Fit Frame to Content", Some("Cmd+Alt+C"), "{} — object.fit {mode: fitFrameToContent}"),
    ("fit.contentToFrame", "Fit Content to Frame", Some("Cmd+Alt+E"), "{} — object.fit {mode: fitContentToFrame}"),
    ("fit.centerContent", "Center Content", Some("Cmd+Shift+E"), "{} — object.fit {mode: centerContent}"),
    ("window.split", "Split Window", None, "{on?: bool} — two views of the document side by side, each with its own zoom and scroll"),
    ("window.newWindow", "New Window", None, "{on?: bool} — another view of the active document in its own window"),
    ("window.minimize", "Minimize Window", None, "{} — minimize the desktop window"),
    ("window.maximize", "Maximize/Restore Window", None, "{on?: bool} — toggle maximization when omitted"),
    ("window.close", "Close Window", None, "{} — request that the desktop window close"),
    ("window.drag", "Move Window", None, "{} — begin a native window drag while the primary mouse button is held"),
    ("window.resize", "Resize Window", None, "{edge: north|south|east|west|northEast|southEast|northWest|southWest} — begin a native edge drag"),
    ("window.taskBar", "Contextual Task Bar", None, "{}"),
    (
        "window.taskBarPin",
        "Pin Bar Position",
        None,
        "{on?: bool, at?: [x, y] (points from the canvas's top-left)} — the Contextual Task Bar stays where it is (or at `at`) instead of following the selection; with neither, toggles",
    ),
    ("window.taskBarReset", "Reset Bar Position", None, "{} — the Contextual Task Bar follows the selection again, under it"),
    ("help.discord", "Join the ArtCraft Discord…", None, "{} — opens https://discord.gg/artcraft in the browser"),
    ("help.appPage", "DesignCraft Website…", None, "{} — opens https://getartcraft.com/apps/designcraft"),
    ("help.github", "DesignCraft on GitHub…", None, "{} — opens https://github.com/storytold/designcraft"),
    ("help.issues", "Report an Issue…", None, "{} — opens the GitHub issue tracker"),
    ("help.website", "ArtCraft Website…", None, "{} — opens https://getartcraft.com"),
    ("help.app", "ArtCraft App Page…", None, "{app} — opens https://getartcraft.com/apps/{app} (e.g. photocraft)"),
    (
        "help.about",
        "About DesignCraft",
        None,
        "{open?: true, tab?: \"about\"|\"contributors\"|\"models\"} — the About window: splash with community links, contributor and model credits (open: false closes it)",
    ),
    ("help.checkForUpdates", "Check for Updates…", None, "{} — checks for newer releases and offers to update"),
    ("window.toolsDoubleColumn", "Tools: Double Column", None, "{}"),
    (
        "window.workspace",
        "Workspace",
        None,
        "{name: Advanced|Book|Digital Publishing|Essentials|Essentials Classic|Interactive for PDF|Printing and Proofing|Typography or a saved workspace}",
    ),
    (
        "window.newWorkspace",
        "New Workspace…",
        None,
        "{name, panelLocations?: bool (default true: the bars and panels), menuCustomization?: bool (default true: the menu items hidden with Edit › Menus)} — saves the workspace",
    ),
    ("window.deleteWorkspace", "Delete Workspace…", None, "{name}"),
    ("window.resetWorkspace", "Reset Workspace", None, "{} — back to the current workspace as saved (or its defaults)"),
    ("window.brightness", "Interface Color Theme", None, "{brightness: dark|mediumDark|mediumLight|light|highContrast}"),
];

/// Menu bar: (menu, entries). Entries: `cmd:<id>`, `ui:<id>`, `-` separator, `>Submenu` … `<`.
pub const MENUS: &[(&str, &[&str])] = &[
    (
        "File",
        &[
            "ui:app.newDocumentDialog",
            "ui:app.openDialog",
            ">Open Recent",
            "cmd:file.clearRecent",
            "<",
            "-",
            "cmd:file.close",
            "ui:app.save",
            "ui:app.saveDialog",
            "ui:app.saveCopyDialog",
            "cmd:file.revert",
            "-",
            "ui:app.placeDialog",
            "ui:app.placeWithOptions",
            "-",
            "ui:app.exportPdf",
            "ui:app.packageDialog",
            "ui:app.printBooklet",
            "ui:app.exportPng",
            "ui:app.exportIdml",
            "ui:app.exportEpub",
            "ui:app.exportFixedEpub",
            "ui:app.exportInteractivePdf",
            "ui:app.exportHtml",
            "ui:app.exportPptx",
            "ui:app.exportXml",
            "ui:app.exportText",
            ">Export",
            "cmd:snippet.export",
            "<",
            "-",
            "cmd:layout.documentSetup",
            "cmd:file.info",
        ],
    ),
    (
        "Edit",
        &[
            "cmd:edit.undo",
            "cmd:edit.redo",
            "-",
            "cmd:edit.cut",
            "cmd:edit.copy",
            "cmd:edit.paste",
            "cmd:edit.pasteWithoutFormatting",
            "cmd:edit.pasteInto",
            "cmd:edit.pasteInPlace",
            "cmd:edit.clear",
            "-",
            "cmd:edit.duplicate",
            "ui:app.placeAndLink",
            "cmd:edit.stepAndRepeat",
            "-",
            "cmd:edit.selectAll",
            "cmd:guide.selectAll",
            "cmd:edit.deselectAll",
            "-",
            "ui:app.findChange",
            "cmd:find.next",
            "ui:app.storyEditor",
            "-",
            ">Spelling",
            "cmd:spelling.check",
            "ui:edit.dynamicSpelling",
            "cmd:spelling.addWord",
            "ui:app.userDictionary",
            "<",
            ">Transparency Blend Space",
            "cmd:edit.transparencyBlendSpace|Document RGB|{\"space\": \"rgb\"}",
            "cmd:edit.transparencyBlendSpace|Document CMYK|{\"space\": \"cmyk\"}",
            "<",
            "ui:app.colorSettings",
            ">Transparency Flattener Presets",
            "ui:app.flattener|None (keep transparency)|{\"preset\": \"\"}",
            "ui:app.flattener|High Resolution|{\"preset\": \"high\"}",
            "ui:app.flattener|Medium Resolution|{\"preset\": \"medium\"}",
            "ui:app.flattener|Low Resolution|{\"preset\": \"low\"}",
            "<",
            "-",
            "ui:app.palette",
            "ui:app.keyboardShortcuts",
            "ui:app.menus",
            "ui:app.preferences",
        ],
    ),
    (
        "Layout",
        &[
            ">Pages",
            "cmd:layout.pages.insert",
            "cmd:layout.pages.move",
            "ui:app.duplicateSpread",
            "ui:app.deletePage",
            "-",
            "cmd:layout.pages.applyParent",
            "cmd:layout.parents.new",
            "<",
            "cmd:layout.marginsAndColumns",
            "cmd:layout.createGuides",
            "cmd:layout.createAlternate",
            "-",
            "ui:view.firstPage",
            "ui:view.previousPage",
            "ui:view.nextPage",
            "ui:view.lastPage",
            "ui:view.nextSpread",
            "ui:view.previousSpread",
            "ui:view.goToPage",
            "ui:view.goBack",
            "ui:view.goForward",
            "-",
            "cmd:layout.section",
            "cmd:toc.generate",
            "cmd:toc.update",
            ">Index",
            "cmd:index.addReference",
            "cmd:index.generate",
            "cmd:index.update",
            "<",
        ],
    ),
    (
        "Type",
        &[
            ">Size",
            "cmd:type.char|6 pt|{\"attrs\": {\"size\": 6}}",
            "cmd:type.char|7 pt|{\"attrs\": {\"size\": 7}}",
            "cmd:type.char|8 pt|{\"attrs\": {\"size\": 8}}",
            "cmd:type.char|9 pt|{\"attrs\": {\"size\": 9}}",
            "cmd:type.char|10 pt|{\"attrs\": {\"size\": 10}}",
            "cmd:type.char|11 pt|{\"attrs\": {\"size\": 11}}",
            "cmd:type.char|12 pt|{\"attrs\": {\"size\": 12}}",
            "cmd:type.char|14 pt|{\"attrs\": {\"size\": 14}}",
            "cmd:type.char|18 pt|{\"attrs\": {\"size\": 18}}",
            "cmd:type.char|24 pt|{\"attrs\": {\"size\": 24}}",
            "cmd:type.char|30 pt|{\"attrs\": {\"size\": 30}}",
            "cmd:type.char|36 pt|{\"attrs\": {\"size\": 36}}",
            "cmd:type.char|48 pt|{\"attrs\": {\"size\": 48}}",
            "cmd:type.char|60 pt|{\"attrs\": {\"size\": 60}}",
            "cmd:type.char|72 pt|{\"attrs\": {\"size\": 72}}",
            "<",
            "-",
            "ui:app.characterPanel",
            "ui:app.paragraphPanel",
            "ui:app.tabsPanel",
            "ui:app.glyphsPanel",
            "-",
            "ui:app.characterStylesPanel",
            "ui:app.paragraphStylesPanel",
            "-",
            "cmd:type.createOutlines",
            "ui:app.findFontDialog",
            "cmd:font.reload",
            ">Change Case",
            "cmd:type.changeCase|UPPERCASE|{\"case\": \"upper\"}",
            "cmd:type.changeCase|lowercase|{\"case\": \"lower\"}",
            "cmd:type.changeCase|Title Case|{\"case\": \"title\"}",
            "cmd:type.changeCase|Sentence case|{\"case\": \"sentence\"}",
            "<",
            "-",
            ">Type on a Path",
            "cmd:type.pathOptions",
            "cmd:type.pathOptions|Delete Type from Path|{\"delete\": true}",
            "<",
            "-",
            ">Notes",
            "cmd:note.new",
            "cmd:note.convertFromText",
            "ui:window.panel|Notes|{\"panel\": \"notes\"}",
            "-",
            "cmd:note.previous",
            "cmd:note.next",
            "<",
            ">Track Changes",
            "cmd:changes.track",
            "cmd:changes.acceptAll",
            "cmd:changes.rejectAll",
            "<",
            "-",
            "cmd:footnote.insert",
            "ui:app.footnoteOptionsDialog",
            "cmd:endnote.insert",
            "cmd:footnote.goToReference",
            "-",
            ">Hyperlinks & Cross-References",
            "cmd:hyperlink.create",
            "cmd:anchor.create",
            "-",
            "ui:app.insertXrefDialog",
            "cmd:xref.defineFormat",
            "<",
            ">Text Variables",
            "cmd:variables.define",
            "cmd:variables.insert",
            "cmd:variables.convertToText",
            "<",
            "-",
            ">Bulleted & Numbered Lists",
            "cmd:list.bullets",
            "cmd:list.numbers",
            "-",
            "cmd:list.define",
            "<",
            "-",
            ">Insert Special Character",
            ">Symbols",
            "cmd:text.insert|Bullet Character|{\"text\": \"\u{2022}\", \"raw\": true}",
            "cmd:text.insert|Copyright Symbol|{\"text\": \"\u{a9}\", \"raw\": true}",
            "cmd:text.insert|Ellipsis|{\"text\": \"\u{2026}\", \"raw\": true}",
            "cmd:text.insert|Paragraph Symbol|{\"text\": \"\u{b6}\", \"raw\": true}",
            "cmd:text.insert|Registered Trademark Symbol|{\"text\": \"\u{ae}\", \"raw\": true}",
            "cmd:text.insert|Section Symbol|{\"text\": \"\u{a7}\", \"raw\": true}",
            "cmd:text.insert|Trademark Symbol|{\"text\": \"\u{2122}\", \"raw\": true}",
            "<",
            ">Markers",
            "ui:app.insertCurrentPageNumber",
            "cmd:text.insert|Next Page Number|{\"text\": \"\u{e007}\", \"raw\": true}",
            "cmd:text.insert|Previous Page Number|{\"text\": \"\u{e008}\", \"raw\": true}",
            "cmd:text.insert|Section Marker|{\"text\": \"\u{e001}\", \"raw\": true}",
            "<",
            ">Hyphens and Dashes",
            "cmd:text.insert|Em Dash|{\"text\": \"\u{2014}\", \"raw\": true}",
            "cmd:text.insert|En Dash|{\"text\": \"\u{2013}\", \"raw\": true}",
            "cmd:text.insert|Discretionary Hyphen|{\"text\": \"\u{ad}\", \"raw\": true}",
            "cmd:text.insert|Nonbreaking Hyphen|{\"text\": \"\u{2011}\", \"raw\": true}",
            "<",
            ">Quotation Marks",
            "cmd:text.insert|Double Left Quotation Marks|{\"text\": \"\u{201c}\", \"raw\": true}",
            "cmd:text.insert|Double Right Quotation Marks|{\"text\": \"\u{201d}\", \"raw\": true}",
            "cmd:text.insert|Single Left Quotation Mark|{\"text\": \"\u{2018}\", \"raw\": true}",
            "cmd:text.insert|Single Right Quotation Mark|{\"text\": \"\u{2019}\", \"raw\": true}",
            "cmd:text.insert|Straight Double Quotation Marks|{\"text\": \"\\\"\", \"raw\": true}",
            "cmd:text.insert|Straight Single Quotation Mark (Apostrophe)|{\"text\": \"'\", \"raw\": true}",
            "<",
            ">Other",
            "cmd:text.insert|Tab|{\"text\": \"\\t\", \"raw\": true}",
            "cmd:text.insert|Right Indent Tab|{\"text\": \"\u{e006}\", \"raw\": true}",
            "cmd:text.insert|Indent to Here|{\"text\": \"\u{e005}\", \"raw\": true}",
            "<",
            "<",
            ">Insert White Space",
            "cmd:text.insert|Em Space|{\"text\": \"\u{2003}\", \"raw\": true}",
            "cmd:text.insert|En Space|{\"text\": \"\u{2002}\", \"raw\": true}",
            "cmd:text.insert|Nonbreaking Space|{\"text\": \"\u{a0}\", \"raw\": true}",
            "cmd:text.insert|Hair Space|{\"text\": \"\u{200a}\", \"raw\": true}",
            "cmd:text.insert|Sixth Space|{\"text\": \"\u{2006}\", \"raw\": true}",
            "cmd:text.insert|Thin Space|{\"text\": \"\u{2009}\", \"raw\": true}",
            "cmd:text.insert|Quarter Space|{\"text\": \"\u{2005}\", \"raw\": true}",
            "cmd:text.insert|Third Space|{\"text\": \"\u{2004}\", \"raw\": true}",
            "cmd:text.insert|Punctuation Space|{\"text\": \"\u{2008}\", \"raw\": true}",
            "cmd:text.insert|Figure Space|{\"text\": \"\u{2007}\", \"raw\": true}",
            "cmd:text.insert|Flush Space|{\"text\": \"\u{2001}\", \"raw\": true}",
            "<",
            ">Insert Break Character",
            "cmd:text.insert|Column Break|{\"text\": \"\u{e002}\", \"raw\": true}",
            "cmd:text.insert|Frame Break|{\"text\": \"\u{e003}\", \"raw\": true}",
            "cmd:text.insert|Page Break|{\"text\": \"\u{e004}\", \"raw\": true}",
            "cmd:text.insert|Odd Page Break|{\"text\": \"\u{e011}\", \"raw\": true}",
            "cmd:text.insert|Even Page Break|{\"text\": \"\u{e012}\", \"raw\": true}",
            "cmd:text.insert|Paragraph Return|{\"text\": \"\\n\", \"raw\": true}",
            "cmd:text.insert|Forced Line Break|{\"text\": \"\u{2028}\", \"raw\": true}",
            "<",
            "cmd:type.fillWithPlaceholder",
            "-",
            "ui:view.hiddenCharacters",
            "-",
            "cmd:math.insert",
            ">Story Direction",
            "cmd:type.storyDirection|Horizontal|{\"vertical\": false}",
            "cmd:type.storyDirection|Vertical|{\"vertical\": true}",
            "<",
            "cmd:type.tateChuYoko",
            "ui:app.rubyDialog",
            "cmd:type.kenten",
            "cmd:type.warichu",
        ],
    ),
    (
        "Object",
        &[
            ">Transform",
            "cmd:transform.move",
            "cmd:transform.scale",
            "cmd:transform.rotate",
            "cmd:transform.shear",
            "-",
            "cmd:transform.flip",
            "-",
            "cmd:transform.clear",
            "<",
            ">Transform Again",
            "cmd:transform.again",
            "cmd:transform.again|Transform Again Individually|{\"individually\": true}",
            "cmd:transform.again|Transform Sequence Again|{\"sequence\": true}",
            "cmd:transform.again|Transform Sequence Again Individually|{\"sequence\": true, \"individually\": true}",
            "<",
            ">Flex Layout",
            "cmd:flex.add",
            "cmd:flex.convert",
            "cmd:flex.remove",
            "-",
            "ui:window.panel|Show Flex Controls|{\"panel\": \"flexLayout\"}",
            "<",
            ">Paths",
            "cmd:object.makeCompoundPath",
            "cmd:object.releaseCompoundPath",
            "-",
            "cmd:path.reverse",
            "<",
            ">Pathfinder",
            "cmd:object.pathfinder|Add|{\"op\": \"add\"}",
            "cmd:object.pathfinder|Subtract|{\"op\": \"subtract\"}",
            "cmd:object.pathfinder|Intersect|{\"op\": \"intersect\"}",
            "cmd:object.pathfinder|Exclude Overlap|{\"op\": \"exclude\"}",
            "cmd:object.pathfinder|Minus Back|{\"op\": \"minusBack\"}",
            "<",
            ">Arrange",
            "cmd:object.bringToFront",
            "cmd:object.bringForward",
            "cmd:object.sendBackward",
            "cmd:object.sendToBack",
            "<",
            "-",
            "cmd:object.group",
            "cmd:object.ungroup",
            "cmd:object.lock",
            "cmd:object.unlockAll",
            "cmd:object.hide",
            "cmd:object.showAll",
            "-",
            "cmd:object.textFrameOptions",
            "ui:app.layerOptions",
            "cmd:object.exportOptions",
            "cmd:object.primaryTextFrame",
            "cmd:object.cornerOptions",
            "-",
            ">Effects",
            "ui:effects.dialog|Transparency…|{\"page\": \"transparency\"}",
            "cmd:object.dropShadow",
            "cmd:object.innerShadow",
            "cmd:object.outerGlow",
            "cmd:object.innerGlow",
            "cmd:object.bevel",
            "cmd:object.satin",
            "cmd:object.feather",
            "cmd:object.directionalFeather",
            "<",
            ">Select",
            "cmd:select.firstAbove",
            "cmd:select.nextAbove",
            "cmd:select.nextBelow",
            "cmd:select.lastBelow",
            "-",
            "cmd:select.container",
            "cmd:select.content",
            "-",
            "cmd:select.previousInGroup",
            "cmd:select.nextInGroup",
            "<",
            ">Convert Shape",
            "cmd:object.convertShape|Rectangle|{\"to\": \"rectangle\"}",
            "cmd:object.convertShape|Rounded Rectangle|{\"to\": \"roundedRectangle\"}",
            "cmd:object.convertShape|Beveled Rectangle|{\"to\": \"beveledRectangle\"}",
            "cmd:object.convertShape|Inverse Rounded Rectangle|{\"to\": \"inverseRoundedRectangle\"}",
            "cmd:object.convertShape|Ellipse|{\"to\": \"ellipse\"}",
            "cmd:object.convertShape|Triangle|{\"to\": \"triangle\"}",
            "cmd:object.convertShape|Polygon|{\"to\": \"polygon\"}",
            "cmd:object.convertShape|Line|{\"to\": \"line\"}",
            "cmd:object.convertShape|Orthogonal Line|{\"to\": \"orthogonalLine\"}",
            "-",
            "cmd:object.convertShape|Open Path|{\"to\": \"openPath\"}",
            "cmd:object.convertShape|Closed Path|{\"to\": \"closedPath\"}",
            "<",
            "ui:app.qrCode",
            ">Clipping Path",
            "cmd:object.clippingPathOptions|None|{\"type\": \"none\"}",
            "cmd:object.clippingPath|Alpha Channel|{\"type\": \"alpha\"}",
            "cmd:object.clippingPath|Detect Edges|{\"type\": \"edges\"}",
            "<",
            ">Captions",
            "cmd:object.caption",
            "cmd:object.caption|Generate Live Caption|{\"live\": true}",
            "<",
            ">Anchored Object",
            "cmd:anchored.options",
            "cmd:anchored.release",
            "<",
            ">Fitting",
            "ui:fit.fillFrameProportionally",
            "ui:fit.contentProportionally",
            "ui:fit.frameToContent",
            "ui:fit.contentToFrame",
            "ui:fit.centerContent",
            "-",
            "ui:app.fittingOptions",
            "<",
            ">Content",
            "cmd:object.content|Graphic|{\"type\":\"graphic\"}",
            "cmd:object.content|Text|{\"type\":\"text\"}",
            "cmd:object.content|Unassigned|{\"type\":\"unassigned\"}",
            "<",
        ],
    ),
    (
        "Table",
        &[
            "ui:app.insertTableDialog",
            "cmd:table.convertFromText",
            "cmd:table.convertToText",
            "-",
            ">Insert",
            "cmd:table.insertRowAbove",
            "cmd:table.insertRowBelow",
            "cmd:table.insertColumnLeft",
            "cmd:table.insertColumnRight",
            "<",
            ">Delete",
            "cmd:table.deleteRow",
            "cmd:table.deleteColumn",
            "cmd:table.delete",
            "<",
            ">Select",
            "cmd:table.selectRow",
            "cmd:table.selectColumn",
            "cmd:table.selectTable",
            "<",
            "-",
            ">Cell Options",
            "ui:app.cellOptionsDialog",
            "<",
            "cmd:table.merge",
            "cmd:table.unmerge",
            ">Convert Cell Type",
            "ui:app.graphicCell|Convert Cell to Graphic Cell…",
            "cmd:table.textCell",
            "<",
            ">Convert Rows",
            "cmd:table.convertRowsToHeader",
            "cmd:table.convertRowsToBody",
            "cmd:table.convertRowsToFooter",
            "<",
            "cmd:table.splitHorizontally",
            "cmd:table.splitVertically",
            "cmd:table.distributeColumns",
            "cmd:table.distributeRows",
            "-",
            "ui:app.tablePanel",
        ],
    ),
    (
        "View",
        &[
            "ui:view.overprintPreview",
            ">Proof Setup",
            "ui:view.proofSetup|Working CMYK|{\"target\": \"workingCmyk\"}",
            "ui:view.proofSetup|Internet Standard RGB (sRGB)|{\"target\": \"srgb\"}",
            "ui:view.proofSetup|Legacy Macintosh RGB|{\"target\": \"legacyMacRgb\"}",
            "ui:view.proofSetup|Monitor RGB|{\"target\": \"monitorRgb\"}",
            "ui:view.proofSetup|Color Blindness – Protanopia|{\"target\": \"protanopia\"}",
            "ui:view.proofSetup|Color Blindness – Deuteranopia|{\"target\": \"deuteranopia\"}",
            "-",
            "ui:view.proofSetup|Simulate Paper Color|{\"simulatePaper\": \"toggle\"}",
            "<",
            "ui:view.proofColors",
            ">Separations Preview",
            "ui:view.separations|Off|{\"plate\": null, \"inkLimit\": null}",
            "ui:view.separations|All Inks|{\"plate\": \"all\"}",
            "ui:view.separations|Cyan|{\"plate\": \"cyan\"}",
            "ui:view.separations|Magenta|{\"plate\": \"magenta\"}",
            "ui:view.separations|Yellow|{\"plate\": \"yellow\"}",
            "ui:view.separations|Black|{\"plate\": \"black\"}",
            "ui:view.separations|Ink Limit 300%|{\"inkLimit\": 300}",
            "<",
            "ui:view.flattenerPreview",
            "-",
            "ui:view.zoomIn",
            "ui:view.zoomOut",
            "ui:view.fitPage",
            "ui:view.fitSelection",
            "ui:view.fitSpread",
            "ui:view.actualSize",
            "ui:view.entirePasteboard",
            "-",
            ">Rotate Spread",
            "ui:view.rotateSpread|90° CW|{\"angle\": 90}",
            "ui:view.rotateSpread|90° CCW|{\"angle\": -90}",
            "ui:view.rotateSpread|180°|{\"angle\": 180}",
            "ui:view.rotateSpread|Clear Rotation|{\"angle\": 0}",
            "<",
            "-",
            ">Screen Mode",
            "ui:view.screenMode|Normal|{\"mode\": \"normal\"}",
            "ui:view.screenMode|Preview|{\"mode\": \"preview\"}",
            "ui:view.screenMode|Bleed|{\"mode\": \"bleed\"}",
            "ui:view.screenMode|Slug|{\"mode\": \"slug\"}",
            "ui:view.screenMode|Presentation|{\"mode\": \"presentation\"}",
            "<",
            ">Display Performance",
            "ui:view.fastDisplay",
            "ui:view.typicalDisplay",
            "ui:view.highQualityDisplay",
            "<",
            "-",
            "ui:view.rulers",
            "-",
            ">Extras",
            "ui:view.frameEdges",
            "ui:view.textThreads",
            "<",
            ">Grids & Guides",
            "ui:view.guides",
            "ui:view.lockGuides",
            "cmd:view.lockColumnGuides",
            "ui:view.snapToGuides",
            "ui:view.snapToDocumentGrid",
            "ui:view.smartGuides",
            "ui:view.baselineGrid",
            "ui:view.documentGrid",
            "-",
            "ui:app.deleteAllGuides",
            "<",
            "-",
            ">Structure",
            "ui:view.taggedFrames",
            "ui:view.tagMarkers",
            "<",
        ],
    ),
    (
        "Window",
        // InDesign's order: Arrange, Workspace, then the panels alphabetically with their groups.
        &[
            ">Arrange",
            "ui:window.newWindow",
            "ui:window.split",
            "-",
            "ui:window.nextDocument",
            "ui:window.previousDocument",
            "<",
            "ui:window.workspace",
            "-",
            "ui:window.panel|Articles|{\"panel\": \"articles\"}",
            ">Color",
            "ui:window.panel|Color|{\"panel\": \"color\"}",
            "ui:window.panel|Gradient|{\"panel\": \"gradient\"}",
            "ui:window.panel|Swatches|{\"panel\": \"swatches\"}",
            "<",
            "ui:window.taskBar",
            "ui:window.controlBar",
            ">Editorial",
            "ui:window.panel|Notes|{\"panel\": \"notes\"}",
            "ui:window.panel|Track Changes|{\"panel\": \"trackChanges\"}",
            "<",
            "ui:window.panel|Effects|{\"panel\": \"effects\"}",
            "ui:window.panel|Info|{\"panel\": \"info\"}",
            ">Interactive",
            "ui:window.panel|Bookmarks|{\"panel\": \"bookmarks\"}",
            "ui:window.panel|Buttons and Forms|{\"panel\": \"buttons\"}",
            "ui:window.panel|Hyperlinks|{\"panel\": \"hyperlinks\"}",
            "ui:window.panel|Liquid Layout|{\"panel\": \"liquid\"}",
            "ui:window.panel|Media|{\"panel\": \"media\"}",
            "ui:window.panel|Object States|{\"panel\": \"states\"}",
            "ui:window.panel|Page Transitions|{\"panel\": \"transitions\"}",
            "<",
            "ui:window.panel|Layers|{\"panel\": \"layers\"}",
            "ui:window.panel|Links|{\"panel\": \"links\"}",
            ">Object & Layout",
            "ui:window.panel|Align|{\"panel\": \"align\"}",
            "ui:window.panel|Flex Layout|{\"panel\": \"flexLayout\"}",
            "ui:window.panel|Pathfinder|{\"panel\": \"pathfinder\"}",
            "<",
            ">Output",
            "ui:window.panel|Attributes|{\"panel\": \"attributes\"}",
            "ui:window.panel|Preflight|{\"panel\": \"preflight\"}",
            "ui:window.panel|Separations Preview|{\"panel\": \"separations\"}",
            "<",
            "ui:window.panel|Pages|{\"panel\": \"pages\"}",
            "ui:window.panel|Properties|{\"panel\": \"properties\"}",
            "ui:window.panel|Stroke|{\"panel\": \"stroke\"}",
            ">Styles",
            "ui:window.panel|Character Styles|{\"panel\": \"characterStyles\"}",
            "ui:window.panel|Paragraph Styles|{\"panel\": \"paragraphStyles\"}",
            "<",
            "ui:window.panel|Text Wrap|{\"panel\": \"textWrap\"}",
            ">Type & Tables",
            "ui:window.panel|Character|{\"panel\": \"character\"}",
            "ui:window.panel|Conditional Text|{\"panel\": \"conditions\"}",
            "ui:window.panel|Glyphs|{\"panel\": \"glyphs\"}",
            "ui:window.panel|Paragraph|{\"panel\": \"paragraph\"}",
            "ui:window.panel|Story|{\"panel\": \"story\"}",
            "ui:window.panel|Table|{\"panel\": \"table\"}",
            "ui:app.tabsPanel",
            "ui:window.panel|Typography|{\"panel\": \"typography\"}",
            "<",
            ">Utilities",
            "ui:window.panel|Book|{\"panel\": \"book\"}",
            "ui:window.panel|Data Merge|{\"panel\": \"dataMerge\"}",
            "ui:window.panel|Library|{\"panel\": \"library\"}",
            "ui:window.panel|Scripts|{\"panel\": \"scripts\"}",
            "ui:window.panel|Tags|{\"panel\": \"tags\"}",
            "<",
            "-",
            "ui:window.hidePanels",
            "ui:window.hidePanelsExceptTools",
            "ui:window.toolsDoubleColumn",
            "-",
            "ui:window.brightness",
        ],
    ),
    (
        "Help",
        &[
            "ui:help.discord",
            "-",
            "ui:help.appPage",
            "ui:help.github",
            "ui:help.issues",
            "ui:help.website",
            "-",
            "cmd:file.newSample",
            "-",
            "ui:help.checkForUpdates",
            "ui:help.about",
        ],
    ),
];

/// The built-in workspaces, in InDesign's order.
pub const WORKSPACES: &[&str] =
    &["Advanced", "Book", "Digital Publishing", "Essentials", "Essentials Classic", "Interactive for PDF", "Printing and Proofing", "Typography"];

/// The built-in workspaces that open a panel below the Properties · Pages · Layers group.
pub const WORKSPACE_OPEN_PANELS: &[(&str, &str)] =
    &[("Printing and Proofing", "swatches"), ("Interactive for PDF", "buttons"), ("Digital Publishing", "liquid")];

/// The panel the built-in workspace `name` opens below the Properties · Pages · Layers group.
pub fn workspace_open_panel(name: &str) -> Option<&'static str> {
    WORKSPACE_OPEN_PANELS.iter().find(|(workspace, _)| *workspace == name).map(|(_, panel)| *panel)
}

/// Essentials Classic's iconic column: InDesign's iconic panel groups, top to bottom.
pub const CLASSIC_GROUPS: &[&[&str]] = &[
    &["pages", "layers", "links"],
    &["stroke", "color"],
    &["swatches"],
    &["effects"],
    &["paragraph"],
    &["textWrap"],
    &["characterStyles", "paragraphStyles"],
    &["gradient"],
];

/// The Typography preset's available panels in the order observed in InDesign's workspace.
/// CC Libraries has no corresponding DesignCraft panel, and where the Story panel sits in it is
/// not recorded, so these groups leave it out.
pub const TYPOGRAPHY_GROUPS: &[&[&str]] = &[
    &["pages", "layers", "swatches"],
    &["glyphs", "textWrap"],
    &["stroke", "gradient"],
    &["hyperlinks", "effects"],
    &["paragraph", "paragraphStyles"],
    &["character", "characterStyles"],
];

pub fn ui_label(id: &str) -> Option<(&'static str, Option<&'static str>)> {
    UI_COMMANDS.iter().find(|c| c.0 == id).map(|c| (c.1, c.2))
}

/// Type › Tabs: float the Tabs panel (placed above the selected text frame) or close it.
fn toggle_tabs_panel(app: &mut DesignApp) {
    if app.ui.floating.iter().any(|(p, _)| p == "tabs") {
        app.ui.floating.retain(|(p, _)| p != "tabs");
    } else {
        let at = app.canvas_rect.map_or(egui::pos2(300.0, 140.0), |r| r.min + egui::vec2(80.0, 60.0));
        app.ui.floating.push(("tabs".into(), [at.x, at.y]));
        app.ui.tabs_panel.snap_request = 2;
    }
}

/// Run a UI command; `None` if `id` isn't one.
pub fn run_ui(app: &mut DesignApp, id: &str, p: &Value) -> Option<Result<Value, String>> {
    let was_find = app.ui.dialog.as_ref().is_some_and(|d| d.id == "findChange");
    let result = run_ui_inner(app, id, p);
    if was_find && app.ui.dialog.as_ref().is_none_or(|d| d.id != "findChange") {
        app.session.reset_find_scope();
    }
    result
}

fn run_ui_inner(app: &mut DesignApp, id: &str, p: &Value) -> Option<Result<Value, String>> {
    if is_window_control(id) && !window_control_available(app, id) {
        return Some(Err("custom window controls are not available in this host".into()));
    }
    if let Some(result) = crate::panel_docking::command(app, id, p) {
        return Some(result);
    }
    let rect = app.canvas_rect;
    let flag = |b: &mut bool| {
        *b = !*b;
        Ok(json!(*b))
    };
    Some(match id {
        "window.minimize" => {
            app.pending_window_commands.push(egui::ViewportCommand::Minimized(true));
            Ok(Value::Null)
        }
        "window.maximize" => {
            let on = p.get("on").and_then(Value::as_bool).unwrap_or(!app.window_maximized);
            app.pending_window_commands.push(egui::ViewportCommand::Maximized(on));
            Ok(json!(on))
        }
        "window.close" => {
            app.pending_window_commands.push(egui::ViewportCommand::Close);
            Ok(Value::Null)
        }
        "window.drag" => {
            app.pending_window_commands.push(egui::ViewportCommand::StartDrag);
            Ok(Value::Null)
        }
        "window.resize" => {
            let edge = match p.get("edge").and_then(Value::as_str) {
                Some("north") => egui::ResizeDirection::North,
                Some("south") => egui::ResizeDirection::South,
                Some("east") => egui::ResizeDirection::East,
                Some("west") => egui::ResizeDirection::West,
                Some("northEast") => egui::ResizeDirection::NorthEast,
                Some("southEast") => egui::ResizeDirection::SouthEast,
                Some("northWest") => egui::ResizeDirection::NorthWest,
                Some("southWest") => egui::ResizeDirection::SouthWest,
                _ => return Some(Err("invalid window resize edge".into())),
            };
            app.pending_window_commands.push(egui::ViewportCommand::BeginResize(edge));
            Ok(Value::Null)
        }
        "app.newDocumentDialog" => {
            app.ui.dialog = Some(crate::dialogs::Dialog::new("newDocument", json!({})));
            Ok(Value::Null)
        }
        "app.openDialog" => app.pick_and_open("open"),
        "app.openRecent" => {
            let index = p.get("index").cloned().unwrap_or(Value::Null);
            let name = index
                .as_u64()
                .and_then(|i| usize::try_from(i).ok())
                .and_then(|i| app.session.prefs.recent_files.get(i))
                .map(|path| recent_label(path))
                .unwrap_or_default();
            app.open_named_file("file.openRecent", json!({"index": index}), name)
        }
        "app.placeDialog" => app.pick_and_open("place"),
        "app.placeWithOptions" => {
            // Word/RTF Import Options: style mapping and conflicts before the text lands.
            let Some(path) = app.services.pick_open.as_mut().and_then(|f| f("place")) else { return Some(Ok(Value::Null)) };
            let l = path.to_lowercase();
            if !(l.ends_with(".docx") || l.ends_with(".rtf")) {
                return Some(app.place_file(json!({"path": path})));
            }
            let info = match app.run("place.styles", json!({"path": path})) {
                Ok(v) => v,
                Err(e) => return Some(Err(e)),
            };
            app.ui.dialog = Some(crate::dialogs::Dialog::new(
                "importOptions",
                json!({"path": path, "styles": info["paragraph"], "charStyles": info["character"], "conflicts": info["conflicts"], "styleConflicts": "useExisting", "removeStyles": false, "map": {}}),
            ));
            Ok(Value::Null)
        }
        "app.save" | "app.saveDialog" if app.services.download.is_some() => download_document(app),
        "app.save" => {
            if app.session.active().is_some_and(|d| d.path.is_some()) {
                return Some(app.run("file.save", json!({})));
            }
            return run_ui(app, "app.saveDialog", p);
        }
        "app.saveDialog" => {
            let name = app.session.active().map(|d| format!("{}.designcraft", d.doc.title)).unwrap_or_default();
            if let Some(path) = app.pick_save(&name) {
                return Some(app.run("file.saveAs", json!({"path": path})));
            }
            Ok(Value::Null)
        }
        "app.quit" => {
            app.request_quit();
            Ok(Value::Null)
        }
        "app.saveCopyDialog" => {
            let name = app.session.active().map(|d| format!("{} copy.designcraft", d.doc.title)).unwrap_or_default();
            if let Some(path) = app.pick_save(&name) {
                return Some(app.run("file.saveACopy", json!({"path": path})));
            }
            Ok(Value::Null)
        }
        "app.exportPng" => export_png(app, p),
        "app.exportIdml" => export_idml(app, p),
        // From the menu, EPUB Export Options first; with a path, straight to the file.
        "app.exportEpub" if p.get("path").is_some() => export_epub(app, p),
        "app.exportEpub" => {
            crate::dialogs::open_epub_export(app);
            Ok(Value::Null)
        }
        "app.exportInteractivePdf" => {
            let mut q = if p.is_object() { p.clone() } else { json!({}) };
            for (k, v) in [("pageLayout", json!("single")), ("view", json!("fitPage")), ("bookmarksPanel", json!(true)), ("media", json!(true))] {
                q.as_object_mut().map(|o| o.entry(k).or_insert(v));
            }
            export_pdf(app, &q)
        }
        "app.exportFixedEpub" => export_bytes(app, p, "epub", "file.exportFixedEpub"),
        "app.exportHtml" => export_bytes(app, p, "html", "file.exportHtml"),
        "app.exportPptx" => export_bytes(app, p, "pptx", "file.exportPptx"),
        "app.exportXml" => export_bytes(app, p, "xml", "file.exportXml"),
        "app.printBooklet" => export_bytes(app, p, "pdf", "file.printBooklet"),
        "app.menus" => {
            app.ui.dialog = Some(crate::dialogs::Dialog::new("menus", json!({"query": ""})));
            Ok(Value::Null)
        }
        "app.cellOptionsDialog" => crate::dialogs::open_cell_options(app),
        "window.hideMenuItem" => {
            let Some(item) = p.get("item").and_then(Value::as_str) else { return Some(Err("missing item".into())) };
            let hide = p.get("hidden").and_then(Value::as_bool).unwrap_or(true);
            app.ui.hidden_menu_items.retain(|x| x != item);
            if hide {
                app.ui.hidden_menu_items.push(item.to_string());
            }
            app.ui.show_full_menus = false;
            Ok(Value::Null)
        }
        "app.placeAndLink" => {
            let target = app.session.active().and_then(|d| {
                let id = *d.selection.items.first()?;
                let it = d.doc.item(id)?;
                let loc = d.doc.find(id)?;
                Some((it.text_frame()?.story, it.bounds(), loc.spread))
            });
            let Some((sid, b, sr)) = target else { return Some(Err("select a text frame".into())) };
            Ok(app
                .run("story.placeAndLink", json!({"story": sid.0, "rect": [b.x0 + 24.0, b.y0 + 24.0, b.x1 + 24.0, b.y1 + 24.0], "spread": sr}))
                .unwrap_or(Value::Null))
        }
        "app.fittingOptions" => {
            let g = app.session.active().and_then(|d| d.selection.items.first().and_then(|id| d.doc.item(*id)).and_then(|it| it.graphic().cloned()));
            let Some(g) = g else { return Some(Err("select a frame with a placed graphic".into())) };
            let fitting = serde_json::to_value(g.auto_fit).unwrap_or(json!("none"));
            let c = |v: f64| json!(designcraft_geom::format_measure(v, designcraft_geom::Unit::Points));
            app.ui.dialog = Some(crate::dialogs::Dialog::new(
                "fittingOptions",
                json!({"autoFit": g.auto_fit != designcraft_doc::Fitting::None, "fitting": fitting, "align": g.fit_align,
                    "cropTop": c(g.crop[0]), "cropLeft": c(g.crop[1]), "cropBottom": c(g.crop[2]), "cropRight": c(g.crop[3])}),
            ));
            Ok(Value::Null)
        }
        "app.userDictionary" => {
            app.ui.dialog = Some(crate::dialogs::Dialog::new("userDictionary", json!({"word": ""})));
            Ok(Value::Null)
        }
        "app.qrCode" => {
            app.ui.dialog = Some(crate::dialogs::Dialog::new("qrCode", json!({})));
            Ok(Value::Null)
        }
        "app.exportText" => export_bytes(app, p, "rtf", "file.exportText"),
        "app.packageDialog" => {
            let name = app.session.active().map(|d| format!("{} Folder", d.doc.title)).unwrap_or_default();
            match app.pick_save(&name) {
                Some(dir) => {
                    let r = app.run("file.package", json!({"dir": dir}));
                    if let Ok(v) = &r {
                        app.status(
                            crate::i18n::tr(&app.ui.language, "Packaged {count} files into {dir}")
                                .replace("{count}", &v["files"].as_array().map_or(0, |f| f.len()).to_string())
                                .replace("{dir}", v["dir"].as_str().unwrap_or("")),
                        );
                    }
                    r
                }
                None => Ok(Value::Null),
            }
        }
        "app.language" => {
            let Some(v) = p.get("lang").and_then(Value::as_str) else { return Some(app.run("app.preferences", json!({}))) };
            if !crate::i18n::LANGUAGES.iter().any(|(c, _)| *c == v) {
                return Some(Err(format!("unknown language `{v}`")));
            }
            app.ui.language = v.to_string();
            Ok(json!(v))
        }
        "app.flattener" => {
            let v = p.get("preset").and_then(Value::as_str).unwrap_or("");
            if !["", "high", "medium", "low"].contains(&v) {
                return Some(Err(format!("unknown preset `{v}`")));
            }
            app.ui.flattener = v.to_string();
            Ok(json!(v))
        }
        "app.graphicCell" => {
            let Some(pick) = app.services.pick_open.as_mut() else { return Some(Err("needs a file picker".into())) };
            match pick("place") {
                Some(path) => app.run("table.placeGraphic", json!({"path": path})),
                None => Ok(Value::Null),
            }
        }
        "effects.dialog" => {
            crate::effects_dialog::open(app, p.get("page").and_then(Value::as_str).unwrap_or("transparency"));
            Ok(Value::Null)
        }
        "app.loadSwatches" => {
            if let Some(pick) = app.services.pick_open.as_mut() {
                return Some(match pick("swatches") {
                    Some(path) => app.run("swatch.load", json!({"path": path})),
                    None => Ok(Value::Null),
                });
            }
            let request = app.import_request("swatches");
            if let Some(open) = app.services.open_async.as_mut() {
                open(request);
            }
            Ok(Value::Null)
        }
        "app.loadParagraphStyles" => app.pick_styles("paragraph"),
        "app.loadCharacterStyles" => app.pick_styles("character"),
        "app.loadTextStyles" => app.pick_styles(""),
        "app.saveSwatches" => {
            let path = match p.get("path").and_then(Value::as_str) {
                Some(s) => Some(s.to_string()),
                None => app.pick_save("Swatches.ase"),
            };
            let Some(path) = path else { return Some(Ok(Value::Null)) };
            let r = match app.run("swatch.save", json!({})) {
                Ok(r) => r,
                Err(e) => return Some(Err(e)),
            };
            let bytes = designcraft_engine::cmd::base64_decode(r["base64"].as_str().unwrap_or_default());
            match app.services.write.as_mut() {
                Some(w) => w(&path, &bytes).map(|_| json!({"path": path, "bytes": bytes.len()})),
                None => Err("no writer".into()),
            }
        }
        "app.storyEditor" => {
            let target = app.session.active().and_then(|st| {
                let story = p.get("story").and_then(Value::as_u64).map(designcraft_doc::StoryId).or_else(|| {
                    st.selection
                        .text
                        .map(|t| t.story)
                        .or_else(|| st.selection.items.iter().find_map(|i| st.doc.item(*i)?.text_frame().map(|t| t.story)))
                })?;
                Some(crate::story_editor::Target { doc: st.uid, story })
            });
            match target {
                Some(t) => {
                    app.story_editor = if app.story_editor == Some(t) { None } else { Some(t) };
                    Ok(Value::Null)
                }
                None => Err("select a text frame or place the cursor in text".into()),
            }
        }
        "app.findChange" => {
            app.session.reset_find_scope();
            app.ui.dialog = Some(crate::dialogs::Dialog::new("findChange", json!({})));
            Ok(Value::Null)
        }
        "app.insertTableDialog" => {
            if app.session.active().is_none_or(|d| d.selection.text.is_none()) {
                return Some(Err("place the insertion point in a text frame to create a table".into()));
            }
            app.ui.dialog = Some(crate::dialogs::Dialog::new("insertTable", p.clone()));
            Ok(Value::Null)
        }
        "app.rubyDialog" => {
            let Some(st) = app.session.active() else { return Some(Err("no document open".into())) };
            let cur = st.selection.text.and_then(|t| {
                let story = st.doc.text_story(t.story, t.cell)?;
                let at = if t.range().is_empty() { t.range().start } else { t.range().start + 1 };
                story.char_format_at(at).over.ruby.clone()
            });
            let cur = cur.unwrap_or_default();
            // The ruby settings the panes show: the selection's resolved character attributes.
            let base = crate::panels::text_attrs(app).and_then(|a| a.get("chars").cloned()).unwrap_or(Value::Null);
            app.ui.dialog = Some(crate::dialogs::Dialog::new("ruby", json!({"text": cur, "base": base})));
            Ok(Value::Null)
        }
        "app.paragraphRulesDialog" => {
            let Some(a) = crate::panels::text_attrs(app) else { return Some(Err("select text or a text frame".into())) };
            let rule = if p.get("rule").and_then(Value::as_str) == Some("ruleBelow") { "ruleBelow" } else { "ruleAbove" };
            let current = json!({"ruleAbove": a["para"]["ruleAbove"], "ruleBelow": a["para"]["ruleBelow"]});
            app.ui.dialog = Some(crate::dialogs::Dialog::new("paragraphRules", json!({"rule": rule, "current": current})));
            Ok(Value::Null)
        }
        "app.paragraphBordersDialog"
        | "app.spanColumnsDialog"
        | "app.bulletsNumberingDialog"
        | "app.keepOptionsDialog"
        | "app.nestedStylesDialog"
        | "app.grepStylesDialog" => {
            let Some(a) = crate::panels::text_attrs(app) else { return Some(Err("select text or a text frame".into())) };
            let section = match id {
                "app.spanColumnsDialog" => "span",
                "app.bulletsNumberingDialog" => "bullets",
                "app.keepOptionsDialog" => "keep",
                "app.nestedStylesDialog" => "nested",
                "app.grepStylesDialog" => "grep",
                _ if p.get("tab").and_then(Value::as_str) == Some("shading") => "shading",
                _ => "border",
            };
            let fields = json!({
                "section": section,
                "current": a["para"],
                "family": a["chars"]["fontFamily"],
                "style": a["chars"]["fontStyle"],
            });
            app.ui.dialog = Some(crate::dialogs::Dialog::new("paragraphLocal", fields));
            Ok(Value::Null)
        }
        "app.hyphenationDialog" => {
            let Some(a) = app.session.execute("type.selectionAttrs", &json!({"mixed": true})).ok().filter(|a| !a.is_null()) else {
                return Some(Err("select text or a text frame".into()));
            };
            let mixed: Vec<&str> = a["paraMixed"].as_array().map(|m| m.iter().filter_map(Value::as_str).collect()).unwrap_or_default();
            let current: serde_json::Map<String, Value> = crate::dialogs::HYPH_ATTRS
                .iter()
                .map(|k| (k.to_string(), if mixed.contains(k) { Value::Null } else { a["para"][*k].clone() }))
                .collect();
            app.ui.dialog = Some(crate::dialogs::Dialog::new("hyphenation", json!({"current": current})));
            Ok(Value::Null)
        }
        "app.justificationDialog" => {
            let Some(a) = app.session.execute("type.selectionAttrs", &json!({"mixed": true})).ok().filter(|a| !a.is_null()) else {
                return Some(Err("select text or a text frame".into()));
            };
            let mixed: Vec<&str> = a["paraMixed"].as_array().map(|m| m.iter().filter_map(Value::as_str).collect()).unwrap_or_default();
            let current: serde_json::Map<String, Value> = crate::dialogs::JUSTIFY_ATTRS
                .iter()
                .map(|k| (k.to_string(), if mixed.contains(k) { Value::Null } else { a["para"][*k].clone() }))
                .collect();
            app.ui.dialog = Some(crate::dialogs::Dialog::new("justification", json!({"current": current})));
            Ok(Value::Null)
        }
        "app.footnoteOptionsDialog" => {
            let Some(st) = app.session.active() else { return Some(Err("no document open".into())) };
            let o = serde_json::to_value(&st.doc.footnote_options).unwrap_or_default();
            app.ui.dialog = Some(crate::dialogs::Dialog::new("footnoteOptions", o));
            Ok(Value::Null)
        }
        "app.findFontDialog" => {
            if app.session.active().is_none() {
                return Some(Err("no document open".into()));
            }
            app.ui.dialog = Some(crate::dialogs::Dialog::new("findFont", json!({})));
            Ok(Value::Null)
        }
        "app.insertXrefDialog" => {
            if app.session.active().is_none_or(|d| d.selection.text.is_none()) {
                return Some(Err("place the insertion point in text to insert a cross-reference".into()));
            }
            app.ui.dialog = Some(crate::dialogs::Dialog::new("insertXref", json!({})));
            Ok(Value::Null)
        }
        "app.deleteAllGuides" => {
            let Some(spread) = guide_spread(app) else { return Some(Err("no document open".into())) };
            return Some(app.run("guide.deleteAll", json!({"spread": spread})));
        }
        "app.deletePage" | "app.duplicateSpread" => {
            let Some(st) = app.session.active() else { return Some(Err("no document open".into())) };
            let page = crate::canvas::current_page(app).unwrap_or(0);
            let r = if id == "app.deletePage" {
                crate::dialogs::delete_pages(app, vec![page])
            } else {
                let spread = st.doc.page_loc(page).map_or(0, |l| l.0);
                app.run("layout.pages.duplicateSpread", json!({"spread": spread}))
            };
            return Some(r);
        }
        "app.tabsPanel" => {
            toggle_tabs_panel(app);
            Ok(Value::Null)
        }
        "app.exportPdf" => {
            if p.is_null() || p.as_object().is_none_or(|m| m.is_empty()) {
                crate::dialogs::open_pdf_export(app);
                Ok(Value::Null)
            } else {
                export_pdf(app, p)
            }
        }
        "app.palette" => {
            app.ui.palette = Some(String::new());
            Ok(Value::Null)
        }
        "app.preferences" => {
            let mut f = app.session.execute("prefs.set", &json!({})).unwrap_or_default();
            if app.session.active().is_some() {
                let doc = app.session.execute("document.preferences", &json!({})).unwrap_or_default();
                f["horizontalUnits"] = doc["horizontalUnits"].clone();
                f["rulerOrigin"] = doc["rulerOrigin"].clone();
                f["overprintBlack"] = doc["overprintBlack"].clone();
                f["glyphFallback"] = doc["glyphFallback"].clone();
                for k in ["superscriptSize", "superscriptPosition", "subscriptSize", "subscriptPosition"] {
                    f[format!("adv.{k}")] = json!(format!("{}", doc["advancedType"][k].as_f64().unwrap_or(0.0)));
                }
                f["verticalUnits"] = doc["verticalUnits"].clone();
                let words = app.session.execute("spelling.words", &json!({})).unwrap_or_default();
                f["userWords"] =
                    json!(words.as_array().map(|a| a.iter().filter_map(Value::as_str).collect::<Vec<_>>().join("\n")).unwrap_or_default());
                let inc = doc["keyboardIncrement"].as_f64().unwrap_or(1.0);
                f["keyboardIncrement"] = json!(designcraft_geom::format_measure(inc, designcraft_geom::Unit::Points));
                f["kerningIncrement"] = json!(format!("{}", doc["kerningIncrement"].as_f64().unwrap_or(20.0)));
                let units = app.session.active().map(|d| d.doc.settings.horizontal_units).unwrap_or_default();
                let m = |v: &Value| json!(designcraft_geom::format_measure(v.as_f64().unwrap_or(0.0), units));
                let (bg, g) = (&doc["baselineGrid"], &doc["grid"]);
                f["bg.start"] = m(&bg["start"]);
                f["bg.increment"] = m(&bg["increment"]);
                f["bg.relativeTo"] = bg["relativeTo"].clone();
                f["bg.viewThreshold"] = json!((bg["viewThreshold"].as_f64().unwrap_or(0.75) * 100.0).round());
                f["bg.color"] = bg["color"].clone();
                f["grid.horizontal"] = m(&g["horizontal"]);
                f["grid.vertical"] = m(&g["vertical"]);
                f["grid.subdivisions"] = g["subdivisions"].clone();
                f["grid.inBack"] = g["inBack"].clone();
                f["grid.color"] = g["color"].clone();
                for k in ["marginColor", "columnColor", "bleedColor", "slugColor"] {
                    f[k] = doc[k].clone();
                }
                f["pasteboard.h"] = m(&doc["pasteboard"][0]);
                f["pasteboard.v"] = m(&doc["pasteboard"][1]);
            }
            f["snap.alignEdges"] = json!(app.ui.align_edges);
            f["snap.alignCenters"] = json!(app.ui.align_centers);
            f["snap.dimensions"] = json!(app.ui.smart_dimensions);
            f["snap.spacing"] = json!(app.ui.smart_spacing);
            f["snap.zone"] = json!(app.ui.snap_zone);
            f["displayQuality"] = json!(match app.ui.display_quality {
                designcraft_render::DisplayQuality::Fast => "fast",
                designcraft_render::DisplayQuality::Typical => "typical",
                designcraft_render::DisplayQuality::High => "high",
            });
            f["uiScale"] = json!((app.ui.ui_scale * 100.0).round());
            f["richBlack"] = json!(app.ui.rich_black);
            f["dynamicSpelling"] = json!(app.ui.dynamic_spelling);
            f["storyEditorSize"] = json!(app.ui.story_editor_size);
            let list = app.session.prefs.autocorrect_list.iter().map(|(a, b)| format!("{a} → {b}")).collect::<Vec<_>>().join("\n");
            f["autocorrectText"] = json!(list);
            // The effective setting; it becomes an explicit preference only when the user changes it,
            // so an untouched one keeps following the interface language.
            let cjk = crate::cjk_features(app);
            f["cjkFeatures"] = json!(cjk);
            f["cjkFeatures.initial"] = json!(cjk);
            f["section"] = json!("general");
            // The language previews as it changes; Cancel goes back to this one.
            f["openedLanguage"] = json!(app.ui.language);
            app.ui.dialog = Some(crate::dialogs::Dialog::new("preferences", f));
            Ok(Value::Null)
        }
        "view.zoomIn" | "view.zoomOut" => {
            if let Some(r) = rect {
                crate::canvas::zoom_at(app, r.center(), if id == "view.zoomIn" { 1.5 } else { 1.0 / 1.5 });
            }
            Ok(Value::Null)
        }
        "view.fitSelection" => {
            if let Some(r) = rect {
                crate::canvas::fit(app, r, "selection");
            }
            Ok(Value::Null)
        }
        "view.fitPage" | "view.fitSpread" | "view.entirePasteboard" => {
            if let Some(r) = rect {
                crate::canvas::fit(
                    app,
                    r,
                    if id == "view.fitPage" {
                        "page"
                    } else if id == "view.fitSpread" {
                        "spread"
                    } else {
                        "all"
                    },
                );
            }
            Ok(Value::Null)
        }
        "view.actualSize" => {
            crate::canvas::set_zoom(app, 1.0);
            Ok(Value::Null)
        }
        "view.zoom" => {
            crate::canvas::set_zoom(app, p.get("zoom").and_then(Value::as_f64).unwrap_or(1.0));
            Ok(Value::Null)
        }
        "view.screenMode" => {
            let mode = match p.get("mode").and_then(Value::as_str).unwrap_or("normal") {
                "preview" => ScreenMode::Preview,
                "bleed" => ScreenMode::Bleed,
                "slug" => ScreenMode::Slug,
                "presentation" => ScreenMode::Presentation,
                _ => ScreenMode::Normal,
            };
            set_screen_mode(app, mode);
            Ok(Value::Null)
        }
        "view.togglePreview" => {
            let mode = if app.ui.screen_mode == ScreenMode::Normal { ScreenMode::Preview } else { ScreenMode::Normal };
            set_screen_mode(app, mode);
            Ok(Value::Null)
        }
        "view.togglePresentation" => {
            let mode = match (app.ui.screen_mode, app.ui.presentation_return) {
                // A settings file can hold Presentation as the return mode; leaving must still leave.
                (ScreenMode::Presentation, ScreenMode::Presentation) => ScreenMode::Normal,
                (ScreenMode::Presentation, back) => back,
                _ => ScreenMode::Presentation,
            };
            set_screen_mode(app, mode);
            Ok(Value::Null)
        }
        "color.defaultFillStroke" => crate::toolbar::default_fill_stroke(app),
        "color.swapFillStroke" => crate::toolbar::swap_fill_stroke(app),
        "color.toggleFillStroke" => crate::toolbar::toggle_fill_stroke(app),
        "color.applyColor" => crate::toolbar::apply_color(app),
        "color.applyGradient" => crate::toolbar::apply_gradient(app),
        "color.applyNone" => crate::toolbar::apply_none(app),
        "view.frameEdges" => flag(&mut app.ui.frame_edges),
        "view.overprintPreview" => {
            app.canvas.shown = None;
            flag(&mut app.ui.overprint_preview)
        }
        "view.fastDisplay" | "view.typicalDisplay" | "view.highQualityDisplay" => {
            use designcraft_render::DisplayQuality as Q;
            app.ui.display_quality = match id {
                "view.fastDisplay" => Q::Fast,
                "view.typicalDisplay" => Q::Typical,
                _ => Q::High,
            };
            // Redraw everything at the new quality.
            app.canvas.shown = None;
            Ok(Value::Null)
        }
        "view.rulers" => flag(&mut app.ui.rulers),
        "view.guides" => flag(&mut app.ui.guides),
        "view.lockGuides" => {
            app.ui.guides_locked = p.get("on").and_then(Value::as_bool).unwrap_or(!app.ui.guides_locked);
            Ok(json!(app.ui.guides_locked))
        }
        "view.snapToGuides" => flag(&mut app.ui.snap_to_guides),
        "view.snapToDocumentGrid" => flag(&mut app.ui.snap_to_document_grid),
        "view.smartGuides" => flag(&mut app.ui.smart_guides),
        "view.baselineGrid" => flag(&mut app.ui.baseline_grid),
        "view.documentGrid" => {
            // The grid is drawn into the canvas texture.
            app.canvas.shown = None;
            flag(&mut app.ui.document_grid)
        }
        "view.textThreads" => flag(&mut app.ui.text_threads),
        "view.hiddenCharacters" => flag(&mut app.ui.hidden_characters),
        "view.taggedFrames" => flag(&mut app.ui.tagged_frames),
        "window.hidePanels" | "window.hidePanelsExceptTools" => {
            let mode = if id == "window.hidePanels" { 1 } else { 2 };
            app.ui.hidden_panels = if app.ui.hidden_panels == mode { 0 } else { mode };
            Ok(json!(app.ui.hidden_panels))
        }
        "window.nextDocument" | "window.previousDocument" => {
            let n = app.session.documents().len();
            if n > 1 {
                let cur = app.session.active_index().unwrap_or(0);
                let next = if id == "window.nextDocument" { (cur + 1) % n } else { (cur + n - 1) % n };
                app.session.set_active(next);
            }
            Ok(json!(app.session.active_index()))
        }
        "app.layerOptionsDialog" => crate::dialogs::open_layer_options(app, p.get("id").and_then(Value::as_u64)).map(|()| Value::Null),
        "app.swatchOptionsDialog" => {
            crate::dialogs::open_swatch_options(app, p.get("name").and_then(Value::as_str).unwrap_or("")).map(|()| Value::Null)
        }
        "app.colorGroupOptionsDialog" => {
            crate::dialogs::open_color_group_options(app, p.get("name").and_then(Value::as_str).unwrap_or("")).map(|()| Value::Null)
        }
        "app.layerOptions" => {
            let layers = match app.session.execute("object.pdfLayers", &json!({})) {
                Ok(v) => v,
                Err(e) => return Some(Err(e.to_string())),
            };
            app.ui.dialog = Some(crate::dialogs::Dialog::new(
                "layerOptions",
                json!({"useDefaults": !layers.as_array().and_then(|a| a.first()).and_then(|l| l["overridden"].as_bool()).unwrap_or(false), "layers": layers}),
            ));
            Ok(Value::Null)
        }
        "view.rotateSpread" => {
            // Turns the spread in the middle of the view (InDesign's Rotate Spread).
            let angle = p.get("angle").and_then(Value::as_i64).unwrap_or(90);
            let Some(st) = app.session.active() else { return Some(Err("no document".into())) };
            let layout = designcraft_tools::CanvasLayout::new(&st.doc, st.editing_parents);
            let Some(slot) = crate::canvas::current_slot(app, &layout).and_then(|i| layout.slots.get(i).copied()) else {
                return Some(Err("no spread".into()));
            };
            let designcraft_doc::SpreadRef::Doc(si) = slot.spread else { return Some(Err("parent spreads don't turn".into())) };
            let r = app.run("layout.rotateSpreadView", json!({"spread": si, "angle": angle}));
            if let Some(rect) = app.canvas_rect {
                crate::canvas::fit(app, rect, "spread");
            }
            app.canvas.shown = None;
            r.map(|v| v["rotation"].clone())
        }
        "view.proofColors" => {
            app.ui.proof_colors = p.get("on").and_then(Value::as_bool).unwrap_or(!app.ui.proof_colors);
            app.canvas.shown = None;
            Ok(json!(app.ui.proof_colors))
        }
        "view.proofSetup" => {
            if let Some(t) = p.get("target").and_then(Value::as_str) {
                let Some(t) = designcraft_color::cms::ProofTarget::parse(t) else { return Some(Err(format!("unknown proof target `{t}`"))) };
                app.ui.proof_setup.target = t;
                app.ui.proof_colors = true;
            }
            if let Some(i) = p.get("intent").and_then(Value::as_str) {
                let Some(i) = designcraft_color::cms::Intent::parse(i) else { return Some(Err(format!("unknown intent `{i}`"))) };
                app.ui.proof_setup.intent = i;
            }
            match p.get("simulatePaper") {
                Some(Value::Bool(b)) => app.ui.proof_setup.simulate_paper = *b,
                Some(Value::String(s)) if s == "toggle" => app.ui.proof_setup.simulate_paper = !app.ui.proof_setup.simulate_paper,
                _ => {}
            }
            app.canvas.shown = None;
            Ok(json!({"target": app.ui.proof_setup.target.id(), "simulatePaper": app.ui.proof_setup.simulate_paper, "on": app.ui.proof_colors}))
        }
        "app.colorSettings" => {
            let f = app.session.execute("color.settings", &json!({})).unwrap_or_default();
            app.ui.dialog = Some(crate::dialogs::Dialog::new("colorSettings", f));
            Ok(Value::Null)
        }
        "view.separations" => separations_view(app, p),
        "view.firstPage" | "view.previousPage" | "view.nextPage" | "view.lastPage" => {
            let n = app.session.active().map_or(0, |d| d.doc.page_count());
            if n > 0 {
                let cur = crate::canvas::current_page(app).unwrap_or(0);
                let to = match id {
                    "view.firstPage" => 0,
                    "view.previousPage" => cur.saturating_sub(1),
                    "view.nextPage" => cur.saturating_add(1).min(n - 1),
                    _ => n - 1,
                };
                crate::canvas::go_to_page(app, to);
            }
            Ok(Value::Null)
        }
        "view.goBack" | "view.goForward" => {
            crate::canvas::go_through_history(app, if id == "view.goBack" { -1 } else { 1 });
            Ok(Value::Null)
        }
        "view.nextSpread" | "view.previousSpread" => {
            crate::canvas::step_spread(app, if id == "view.nextSpread" { 1 } else { -1 });
            Ok(Value::Null)
        }
        "view.goToPage" => {
            match p.get("page") {
                Some(Value::Number(n)) => crate::canvas::go_to_page(app, (n.as_u64().unwrap_or(1) as usize).saturating_sub(1)),
                Some(Value::String(s)) => match app.session.resolve_page(s) {
                    Some(abs) => crate::canvas::go_to_page(app, abs),
                    None => return Some(Err(format!("no page \"{s}\""))),
                },
                _ => app.ui.dialog = Some(crate::dialogs::Dialog::new("goToPage", json!({}))),
            }
            Ok(Value::Null)
        }
        "app.characterPanel" | "app.paragraphPanel" | "app.glyphsPanel" | "app.characterStylesPanel" | "app.paragraphStylesPanel" => {
            let panel = match id {
                "app.characterPanel" => "character",
                "app.paragraphPanel" => "paragraph",
                "app.glyphsPanel" => "glyphs",
                "app.characterStylesPanel" => "characterStyles",
                _ => "paragraphStyles",
            };
            run_ui(app, "window.panel", &json!({"panel": panel})).unwrap_or(Ok(Value::Null))
        }
        "app.insertCurrentPageNumber" => app.run("text.insert", json!({"text": "\u{e000}", "raw": true})),
        // `panel_docking::command` takes the dock panels (and the Window › Styles group) first.
        "window.panel" => {
            let panel = p.get("panel").and_then(Value::as_str).unwrap_or("properties");
            if panel != "tabs" {
                return crate::panel_docking::command(app, "window.panel.activate", &json!({"panel": panel}));
            }
            toggle_tabs_panel(app);
            Ok(Value::Null)
        }
        "fit.fillFrameProportionally" | "fit.contentProportionally" | "fit.frameToContent" | "fit.contentToFrame" | "fit.centerContent" => {
            let mode = match id {
                "fit.fillFrameProportionally" => "fillProportionally",
                "fit.contentProportionally" => "fitProportionally",
                "fit.frameToContent" => "fitFrameToContent",
                "fit.contentToFrame" => "fitContentToFrame",
                _ => "centerContent",
            };
            app.run("object.fit", json!({"mode": mode}))
        }
        "app.keyboardShortcuts" => {
            let select = p.get("command").and_then(Value::as_str).map(|id| crate::keys::command_ref(id, p.get("params").unwrap_or(&Value::Null)));
            crate::shortcuts_dialog::open(app, select.as_ref());
            Ok(Value::Null)
        }
        "window.shortcutSets" => {
            let builtins = &crate::shortcuts_dialog::BUILTINS;
            if let Some(name) = p.get("select").and_then(Value::as_str) {
                if app.shortcut_dialog.is_some() {
                    return Some(Err("close the Keyboard Shortcuts dialog first".into()));
                }
                if let Err(e) = app.ui.shortcut_sets.select(name, builtins) {
                    return Some(Err(format!("{name}: {e}")));
                }
                crate::shortcuts_dialog::load_active_set(app);
            }
            let sets = &app.ui.shortcut_sets;
            Ok(
                json!({"active": sets.active_name(builtins), "sets": sets.names(builtins), "builtIn": builtins.iter().map(|(n, _)| *n).collect::<Vec<_>>()}),
            )
        }
        // The open Keyboard Shortcuts dialog edits the keys itself, and its Cancel puts back the
        // ones it started with, which would undo a change made here.
        "window.setShortcut" | "window.resetShortcuts" | "window.importShortcuts" if app.shortcut_dialog.is_some() => {
            Err("close the Keyboard Shortcuts dialog first".into())
        }
        "window.setShortcut" => {
            let Some(id) = p.get("id").and_then(Value::as_str) else { return Some(Err("missing id".into())) };
            let mut cmd = crate::keys::command_ref(id, p.get("params").unwrap_or(&Value::Null));
            // A bare tool id ("pen") names its tool.
            if app.keymap.bindable(&cmd).is_none() && designcraft_tools::tool_info(id).is_some() {
                cmd = crate::keys::tool(id);
            }
            let shortcut = match p.get("shortcut") {
                None | Some(Value::Null) => None,
                Some(Value::String(sc)) => Some(sc.as_str()),
                Some(_) => return Some(Err("shortcut: a string or null".into())),
            };
            let steal = p.get("steal").and_then(Value::as_bool).unwrap_or(false);
            let policy = if steal { craft_keymap::Policy::Steal } else { craft_keymap::Policy::Keep };
            let taken = match crate::keys::set(app, &cmd, shortcut, policy) {
                Ok(t) => t,
                Err(e) => return Some(Err(e)),
            };
            let mut conflicts: Vec<String> = taken.iter().filter(|c| !crate::keys::twins(&cmd, c)).map(|c| c.key()).collect();
            for key in crate::keys::conflicts(&app.keymap, &cmd) {
                if !conflicts.contains(&key) {
                    conflicts.push(key);
                }
            }
            Ok(json!({"shortcut": crate::keys::shortcut(&app.keymap, &cmd), "conflicts": conflicts}))
        }
        "window.resetShortcuts" => {
            app.keymap.reset_all();
            crate::keys::store(app);
            Ok(Value::Null)
        }
        "window.importShortcuts" => {
            let text = match (p.get("text").and_then(Value::as_str), p.get("path").and_then(Value::as_str)) {
                (Some(text), _) => text.to_string(),
                (None, Some(path)) => match crate::shortcut_import::read_file(path) {
                    Ok(text) => text,
                    Err(e) => return Some(Err(e)),
                },
                (None, None) => return Some(Err("give the set's path or text".into())),
            };
            crate::shortcut_import::import(app, &text)
        }
        "view.snapPreferences" => {
            let b = |k: &str| p.get(k).and_then(Value::as_bool);
            let ui = &mut app.ui;
            ui.align_edges = b("alignEdges").unwrap_or(ui.align_edges);
            ui.align_centers = b("alignCenters").unwrap_or(ui.align_centers);
            ui.smart_dimensions = b("smartDimensions").unwrap_or(ui.smart_dimensions);
            ui.smart_spacing = b("smartSpacing").unwrap_or(ui.smart_spacing);
            if let Some(z) = p.get("zone").and_then(Value::as_f64).filter(|z| z.is_finite()) {
                ui.snap_zone = z.max(0.0);
            }
            Ok(json!({
                "alignEdges": ui.align_edges,
                "alignCenters": ui.align_centers,
                "smartDimensions": ui.smart_dimensions,
                "smartSpacing": ui.smart_spacing,
                "zone": ui.snap_zone,
            }))
        }
        "app.formattingAffectsText" => {
            app.ui.formatting_affects_text = p.get("on").and_then(Value::as_bool).unwrap_or(!app.ui.formatting_affects_text);
            Ok(json!({"on": app.ui.formatting_affects_text}))
        }
        "window.richBlack" => {
            app.ui.rich_black = p.get("on").and_then(Value::as_bool).unwrap_or(!app.ui.rich_black);
            app.canvas.shown = None;
            Ok(json!({"on": app.ui.rich_black}))
        }
        "window.uiScale" => {
            let v = p.get("scale").and_then(Value::as_f64).unwrap_or(1.0) as f32;
            app.ui.ui_scale = v.clamp(0.5, 3.0);
            Ok(json!({"scale": app.ui.ui_scale}))
        }
        "window.controlBar" => flag(&mut app.ui.control_bar),
        "view.flattenerPreview" => flag(&mut app.ui.flattener_preview),
        "view.tagMarkers" => {
            app.canvas.shown = None;
            flag(&mut app.ui.tag_markers)
        }
        "edit.dynamicSpelling" => flag(&mut app.ui.dynamic_spelling),
        "window.split" => {
            let on = p.get("on").and_then(Value::as_bool).unwrap_or(!app.split);
            app.split = on;
            if on {
                if app.second_window {
                    app.end_second_ime_viewport();
                }
                app.second_window = false;
            } else {
                app.switch_pane(0);
                app.focus_pane = 0;
            }
            Ok(json!(on))
        }
        "window.newWindow" => {
            // The second view moves into its own window (on the web: a floating window).
            let on = p.get("on").and_then(Value::as_bool).unwrap_or(true);
            if app.second_window && !on {
                app.end_second_ime_viewport();
            }
            app.second_window = on;
            app.split = false;
            if !app.second_window {
                app.switch_pane(0);
                app.focus_pane = 0;
            }
            Ok(json!(app.second_window))
        }
        "window.taskBar" => flag(&mut app.ui.task_bar),
        "window.taskBarPin" => {
            let at = match p.get("at") {
                None | Some(Value::Null) => None,
                Some(v) => match v.as_array().map(|a| a.iter().map(Value::as_f64).collect::<Vec<_>>()).as_deref() {
                    Some([Some(x), Some(y)]) if x.is_finite() && y.is_finite() => Some([x.clamp(-1e6, 1e6) as f32, y.clamp(-1e6, 1e6) as f32]),
                    _ => return Some(Err("window.taskBarPin: `at` is [x, y] in points".into())),
                },
            };
            let on = p.get("on").and_then(Value::as_bool).unwrap_or(at.is_some() || app.ui.task_bar_pin.is_none());
            app.ui.task_bar_pin = match (on, at.or(app.ui.task_bar_pin).or(app.ui.task_bar_at)) {
                (false, _) => None,
                (true, Some(at)) => Some(at),
                (true, None) => return Some(Err("window.taskBarPin: the Contextual Task Bar isn't showing; give `at`".into())),
            };
            Ok(json!(app.ui.task_bar_pin))
        }
        "window.taskBarReset" => {
            app.ui.task_bar_pin = None;
            Ok(Value::Null)
        }
        "help.about" => {
            if let Some(tab) = p.get("tab").and_then(Value::as_str) {
                let Some(i) = crate::about::ABOUT_TABS.iter().position(|t| t.eq_ignore_ascii_case(tab)) else {
                    return Some(Err(format!("help.about: unknown tab `{tab}` (about, contributors, models)")));
                };
                app.ui.about_tab = u8::try_from(i).unwrap_or(0);
            }
            app.ui.about = p.get("open").and_then(Value::as_bool).unwrap_or(true);
            Ok(json!(app.ui.about))
        }
        "help.checkForUpdates" => crate::updates_ui::check(app),
        "help.app" => match p.get("app").and_then(Value::as_str) {
            Some(slug) if !slug.is_empty() && slug.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') => {
                let url = format!("{}/apps/{slug}", designcraft_engine::links::WEBSITE);
                app.ui.pending_urls.push(url.clone());
                Ok(json!({"url": url}))
            }
            _ => Err("help.app: give `app` (e.g. \"photocraft\")".into()),
        },
        h if h.starts_with("help.") && designcraft_engine::links::get(&h[5..]).is_some() => {
            let url = designcraft_engine::links::get(&h[5..]).unwrap_or_default().to_string();
            app.ui.pending_urls.push(url.clone());
            Ok(json!({"url": url}))
        }
        "window.toolsDoubleColumn" => flag(&mut app.ui.tools_double_column),
        "window.newWorkspace" => {
            let Some(name) = p.get("name").and_then(Value::as_str).map(str::trim).filter(|n| !n.is_empty()) else {
                app.ui.dialog =
                    Some(crate::dialogs::Dialog::new("newWorkspace", json!({"name": "", "panelLocations": true, "menuCustomization": true})));
                return Some(Ok(Value::Null));
            };
            let u = &app.ui;
            let w = crate::SavedWorkspace {
                name: name.to_string(),
                control_bar: u.control_bar,
                task_bar: u.task_bar,
                task_bar_pin: u.task_bar_pin,
                tools_double_column: u.tools_double_column,
                dock_tab: u.dock_tab.clone(),
                dock_expanded: u.dock_expanded,
                open_panel: u.open_panel.clone(),
                floating: u.floating.clone(),
                docking: u.docking.clone(),
                docking_hidden: u.docking_hidden.clone(),
                icon_groups: u.icon_groups.clone(),
                keep_panels: !p.get("panelLocations").and_then(Value::as_bool).unwrap_or(true),
                hidden_menu_items: p.get("menuCustomization").and_then(Value::as_bool).unwrap_or(true).then(|| u.hidden_menu_items.clone()),
            };
            app.ui.custom_workspaces.retain(|x| x.name != name);
            app.ui.custom_workspaces.push(w);
            app.ui.workspace = name.to_string();
            Ok(Value::Null)
        }
        "window.deleteWorkspace" => {
            let name = p.get("name").and_then(Value::as_str).unwrap_or("");
            let n = app.ui.custom_workspaces.len();
            app.ui.custom_workspaces.retain(|x| x.name != name);
            if app.ui.custom_workspaces.len() == n {
                return Some(Err(format!("no saved workspace `{name}`")));
            }
            if app.ui.workspace == name {
                app.ui.workspace = "Essentials".into();
            }
            Ok(Value::Null)
        }
        "window.resetWorkspace" => {
            let name = app.ui.workspace.clone();
            return run_ui(app, "window.workspace", &json!({"name": name}));
        }
        "window.workspace" => {
            app.ui.docking_generation = app.ui.docking_generation.wrapping_add(1);
            crate::panel_docking::forget_panels(app);
            let name = p.get("name").and_then(Value::as_str).unwrap_or("Essentials");
            if let Some(w) = app.ui.custom_workspaces.iter().find(|w| w.name == name).cloned() {
                if let Some(hidden) = w.hidden_menu_items {
                    app.ui.hidden_menu_items = hidden;
                }
                if w.keep_panels {
                    app.ui.workspace = w.name;
                    return Some(Ok(Value::Null));
                }
                app.ui.control_bar = w.control_bar;
                app.ui.task_bar = w.task_bar;
                app.ui.task_bar_pin = w.task_bar_pin;
                app.ui.tools_double_column = w.tools_double_column;
                app.ui.dock_tab = w.dock_tab;
                app.ui.dock_expanded = w.dock_expanded;
                app.ui.open_panel = w.open_panel;
                app.ui.floating = w.floating;
                app.ui.docking = w.docking;
                app.ui.docking_hidden = w.docking_hidden;
                app.ui.icon_groups = w.icon_groups;
                app.ui.workspace = w.name;
                return Some(Ok(Value::Null));
            }
            // Workspaces choose which bars and panels are visible.
            if app.ui.docking.is_some() {
                app.ui.floating.clear();
            }
            app.ui.docking = None;
            app.ui.docking_hidden.clear();
            app.ui.control_bar = matches!(name, "Advanced" | "Typography" | "Printing and Proofing" | "Book" | "Essentials Classic");
            // Essentials Classic: the Control panel and an iconic column of grouped panels, no
            // Properties stack.
            let groups = match name {
                "Essentials Classic" => Some(CLASSIC_GROUPS),
                "Typography" => Some(TYPOGRAPHY_GROUPS),
                _ => None,
            };
            app.ui.icon_groups = groups.map(|groups| groups.iter().map(|g| g.iter().map(ToString::to_string).collect()).collect());
            app.ui.dock_expanded = groups.is_none();
            let dock_tab = match name {
                "Interactive for PDF" | "Digital Publishing" => Some("pages"),
                _ => None,
            };
            if let Some(tab) = dock_tab {
                app.ui.dock_tab = tab.into();
                app.ui.dock_expanded = true;
            }
            app.ui.open_panel = workspace_open_panel(name).map(str::to_string);
            app.ui.workspace = name.to_string();
            Ok(Value::Null)
        }
        "window.brightness" => {
            let b = p.get("brightness").and_then(Value::as_str).and_then(crate::theme::Brightness::parse).unwrap_or(crate::theme::Brightness::Dark);
            app.ui.brightness = b;
            app.restyle = true;
            Ok(Value::Null)
        }
        _ => return None,
    })
}

/// Save by handing the serialized document to the host as a download (web).
/// Switch screen modes; entering Presentation remembers the mode to return to.
fn set_screen_mode(app: &mut DesignApp, mode: ScreenMode) {
    let presenting = app.ui.screen_mode == ScreenMode::Presentation;
    if mode == ScreenMode::Presentation && !presenting {
        app.ui.presentation_return = app.ui.screen_mode;
        // Presentation fits the spread to the screen; leaving it puts the user's view back.
        app.ui.presentation_view = app.view_key().zip(app.view().copied());
        app.ui.presentation_page = crate::canvas::page_in_view(app);
        app.ui.presentation_history = app.page_history().cloned();
    }
    if mode != ScreenMode::Presentation
        && presenting
        && let Some((k, v)) = app.ui.presentation_view.take()
        && app.views.contains_key(&k)
    {
        // Go Back sees the presentation as one jump to the last spread presented, and putting the
        // view back as a jump from there: the spreads in between would otherwise have replaced the
        // place the view was at with the presentation's.
        let shown = app.ui.presentation_page.take().zip(crate::canvas::page_in_view(app)).filter(|_| app.view_key() == Some(k));
        let h = app.page_history.entry(k).or_default();
        *h = app.ui.presentation_history.take().unwrap_or_default();
        if let Some(((from, at), (to, offset))) = shown {
            h.leave(from, at);
            h.visit(to, offset);
            h.visit(from, at);
        }
        if let Some(view) = app.views.get_mut(&k) {
            *view = v;
        }
    }
    app.ui.screen_mode = mode;
}

fn download_document(app: &mut DesignApp) -> Result<Value, String> {
    let name = app.session.active().map(|d| format!("{}.designcraft", d.doc.title)).ok_or("no document")?;
    let ser = app.run("file.serialize", json!({}))?;
    let bytes = designcraft_engine::cmd::base64_decode(ser["base64"].as_str().unwrap_or_default());
    let download = app.services.download.as_mut().ok_or("no download service")?;
    download(&name, &bytes).map_err(|e| format!("could not download {name}: {e}"))?;
    // Marks the document saved; on the web the engine doesn't touch a file system.
    app.run("file.saveAs", json!({"path": name}))
}

fn export_png(app: &mut DesignApp, p: &Value) -> Result<Value, String> {
    let st = app.session.active().ok_or("no document")?;
    let page = crate::control::page_index(p, crate::canvas::current_page(app).unwrap_or(0), st.doc.page_count())?;
    let scale = p.get("scale").and_then(Value::as_f64).unwrap_or(2.0);
    let mut r = designcraft_render::Renderer::new();
    let img = r
        .render_page(
            &st.doc,
            &app.session.cache,
            page,
            scale,
            false,
            &designcraft_render::RenderOptions { printing_only: true, rich_black: app.session.prefs.rich_black_output, ..Default::default() },
        )
        .ok_or("no such page")?;
    let png = img.to_png();
    let path = match p.get("path").and_then(Value::as_str) {
        Some(s) => Some(s.to_string()),
        None => {
            let name = format!("{}-p{}.png", st.doc.title, page + 1);
            app.pick_save(&name)
        }
    };
    let Some(path) = path else { return Ok(Value::Null) };
    match app.services.write.as_mut() {
        Some(w) => w(&path, &png).map(|_| json!({"path": path, "width": img.width, "height": img.height})),
        None => Err("no writer".into()),
    }
}

/// File › Export IDML…: ask for a path, export through `file.exportIdml` and write the bytes with
/// the platform writer (a download on the web).
fn export_idml(app: &mut DesignApp, p: &Value) -> Result<Value, String> {
    let st = app.session.active().ok_or("no document")?;
    let path = match p.get("path").and_then(Value::as_str) {
        Some(s) => Some(s.to_string()),
        None => {
            let name = format!("{}.idml", st.doc.title);
            app.pick_save(&name)
        }
    };
    let Some(path) = path else { return Ok(Value::Null) };
    let r = app.run("file.exportIdml", json!({}))?;
    let bytes = designcraft_engine::cmd::base64_decode(r["base64"].as_str().unwrap_or_default());
    match app.services.write.as_mut() {
        Some(w) => w(&path, &bytes).map(|_| json!({"path": path, "bytes": bytes.len()})),
        None => Err("no writer".into()),
    }
}

/// Export EPUB / Text: ask for a path (default extension `ext`), run `cmd` without one and write
/// what it returns (`base64` or `text`) with the platform writer.
/// File › Export EPUB…: export through `file.exportEpub` (the first page as its cover) to `path`,
/// else to a file the user picks.
pub(crate) fn export_epub(app: &mut DesignApp, p: &Value) -> Result<Value, String> {
    let mut q = if p.is_object() { p.clone() } else { json!({}) };
    if let Some(o) = q.as_object_mut() {
        o.entry("cover").or_insert(json!(true));
    }
    export_bytes(app, &q, "epub", "file.exportEpub")
}

fn export_bytes(app: &mut DesignApp, p: &Value, ext: &str, cmd: &str) -> Result<Value, String> {
    let st = app.session.active().ok_or("no document")?;
    let path = match p.get("path").and_then(Value::as_str) {
        Some(s) => Some(s.to_string()),
        None => {
            let name = format!("{}.{ext}", st.doc.title);
            app.pick_save(&name)
        }
    };
    let Some(path) = path else { return Ok(Value::Null) };
    // The caller's options go to the engine command; the path is ours.
    let mut params = match p {
        Value::Object(o) => Value::Object(o.iter().filter(|(k, _)| k.as_str() != "path").map(|(k, v)| (k.clone(), v.clone())).collect()),
        _ => json!({}),
    };
    if cmd == "file.exportText" {
        let txt = path.to_ascii_lowercase().ends_with(".txt");
        params["format"] = json!(if txt { "txt" } else { "rtf" });
    }
    let r = app.run(cmd, params)?;
    let bytes = match r["text"].as_str() {
        Some(t) => t.as_bytes().to_vec(),
        None => designcraft_engine::cmd::base64_decode(r["base64"].as_str().unwrap_or_default()),
    };
    match app.services.write.as_mut() {
        Some(w) => w(&path, &bytes).map(|_| json!({"path": path, "bytes": bytes.len()})),
        None => Err("no writer".into()),
    }
}

/// View › Separations Preview (`view.separations`): updates the plate and the ink limit and
/// reports the plates of the spread nearest the view center.
fn separations_view(app: &mut DesignApp, p: &Value) -> Result<Value, String> {
    let mut sep = app.ui.separations.clone().unwrap_or_default();
    let mut on = app.ui.separations.is_some();
    match p.get("plate") {
        Some(Value::String(s)) if s.eq_ignore_ascii_case("all") || s.is_empty() => {
            sep.plate = None;
            on = true;
        }
        Some(Value::String(s)) => {
            let process = ["Cyan", "Magenta", "Yellow", "Black"].into_iter().find(|n| n.eq_ignore_ascii_case(s));
            sep.plate = Some(process.map_or_else(|| s.clone(), str::to_string));
            on = true;
        }
        Some(Value::Null) => {
            sep.plate = None;
            on = false;
        }
        Some(v) => return Err(format!("`plate`: an ink name, \"all\" or null, not {v}")),
        None => {}
    }
    match p.get("inkLimit") {
        Some(Value::Null) => sep.ink_limit = None,
        Some(v) => {
            let l = v.as_f64().filter(|l| l.is_finite() && *l >= 0.0).ok_or_else(|| format!("`inkLimit`: a percentage, not {v}"))?;
            sep.ink_limit = Some(l as f32);
            on = true;
        }
        None => {}
    }
    on |= sep.ink_limit.is_some();
    if let Some(v) = p.get("on").and_then(Value::as_bool) {
        on = v;
    }
    app.ui.separations = on.then_some(sep.clone());
    let mut out = json!({"on": on, "plate": sep.plate.as_deref().unwrap_or("all"), "inkLimit": sep.ink_limit});
    if on && app.session.active().is_some() {
        match crate::separations::current_sheet(app) {
            Ok(sheet) => {
                if let Some(name) = &sep.plate
                    && sheet.ink(name).is_none()
                {
                    out["warning"] = json!(format!("this spread prints no `{name}` plate"));
                }
                out["spread"] = json!(sheet.spread);
                out["dpi"] = json!(sheet.plates.dpi);
                out["inks"] = sheet.inks().into_iter().map(|(name, spot)| json!({"name": name, "spot": spot})).collect();
            }
            Err(e) => out["error"] = json!(e),
        }
    }
    Ok(out)
}

/// File › Export PDF…: ask for a path, export through `file.exportPdf` and write the bytes with the
/// platform writer (a download on the web).
pub(crate) fn export_pdf(app: &mut DesignApp, p: &Value) -> Result<Value, String> {
    let st = app.session.active().ok_or("no document")?;
    let path = match p.get("path").and_then(Value::as_str) {
        Some(s) => Some(s.to_string()),
        None => {
            let name = format!("{}.pdf", st.doc.title);
            app.pick_save(&name)
        }
    };
    let Some(path) = path else { return Ok(Value::Null) };
    let mut params = if p.is_object() { p.clone() } else { json!({}) };
    if let Some(o) = params.as_object_mut() {
        o.remove("path");
        o.entry("bleed").or_insert(json!(true));
        o.entry("tagged").or_insert(json!(true));
        if !app.ui.flattener.is_empty() {
            o.entry("flatten").or_insert(json!(app.ui.flattener));
        }
    }
    let r = app.run("file.exportPdf", params)?;
    let bytes = designcraft_engine::cmd::base64_decode(r["base64"].as_str().unwrap_or_default());
    match app.services.write.as_mut() {
        Some(w) => w(&path, &bytes).map(|_| json!({"path": path, "bytes": bytes.len(), "pages": r["pages"], "warnings": r["warnings"]})),
        None => Err("no writer".into()),
    }
}

/// Commands that act on the text being typed wait while an IME composes at the caret (its marked
/// text isn't typed yet): Undo and Redo (the native menu takes ⌘Z ahead of the IME), the clipboard
/// and the selection.
pub(crate) fn waits_for_ime(app: &DesignApp, id: &str) -> bool {
    app.session.tool_composing()
        && (matches!(
            id,
            "edit.undo" | "edit.redo" | "edit.cut" | "edit.copy" | "edit.clear" | "edit.duplicate" | "edit.selectAll" | "edit.deselectAll"
        ) || id.starts_with("edit.paste"))
}

/// The commands that act on the desktop window itself.
fn is_window_control(id: &str) -> bool {
    matches!(id, "window.minimize" | "window.maximize" | "window.close" | "window.drag" | "window.resize")
}

/// Whether the host can run a window command: all of them with our own title bar, and Minimize
/// with the macOS menu bar (Window › Minimize, ⌘M).
fn window_control_available(app: &DesignApp, id: &str) -> bool {
    app.custom_titlebar || (app.native_menu && id == "window.minimize")
}

/// Is a command enabled (for menu greying)?
pub fn enabled(app: &DesignApp, id: &str) -> bool {
    if waits_for_ime(app, id) {
        return false;
    }
    if is_window_control(id) {
        return window_control_available(app, id);
    }
    // Shared docking commands DesignCraft does not provide (native panel windows, Lock Workspace).
    if matches!(id, "window.panel.window" | "window.workspace.lock") {
        return false;
    }
    match designcraft_engine::find_command(id) {
        Some(c) => (c.enabled)(&app.session).is_ok(),
        None => true,
    }
}

/// The default shortcut of a command.
pub fn default_shortcut(id: &str) -> Option<&'static str> {
    designcraft_engine::find_command(id).and_then(|c| c.shortcut).or_else(|| ui_label(id).and_then(|(_, s)| s))
}

/// The shortcut in effect for a command (the user's, else the default), as stored ("Cmd+Alt+K").
pub fn shortcut_of(app: &DesignApp, id: &str) -> Option<String> {
    crate::keys::shortcut(&app.keymap, &craft_keymap::CommandRef::new(id))
}

/// The shortcut a menu row shows: its own binding (a panel's), else its command's.
pub fn shortcut_label(app: &DesignApp, id: &str, params: &Value) -> Option<String> {
    let cmd = crate::keys::command_ref(id, params);
    let cmd = if app.keymap.bindable(&cmd).is_some() { cmd } else { craft_keymap::CommandRef::new(id) };
    crate::keys::shortcut_display(&app.keymap, &cmd)
}

/// The shortcut of a tool, for its button's tooltip.
pub fn tool_shortcut_label(app: &DesignApp, tool: &str) -> Option<String> {
    crate::keys::shortcut_display(&app.keymap, &crate::keys::tool(tool))
}

/// A shortcut for display: ⌃⌥⇧⌘K on macOS, Ctrl+Alt+Shift+K elsewhere.
pub fn shortcut_text(sc: &str) -> String {
    match craft_keymap::Binding::parse(sc) {
        Ok(b) => b.trigger.display(crate::keys::platform()),
        Err(_) => sc.to_string(),
    }
}

/// A menu entry, shared by the in-window menu bar and the native macOS menu.
#[derive(Clone, Debug)]
pub enum Item {
    Sep,
    Sub(String, Vec<Item>),
    Cmd { label: String, id: String, params: Value, shortcut: Option<&'static str> },
}

fn cmd_item(id: &str, params: Value) -> Item {
    let (label, shortcut) = match ui_label(id) {
        Some((l, sc)) => (l.to_string(), sc),
        None => match designcraft_engine::find_command(id) {
            Some(c) => (c.label.to_string(), c.shortcut),
            None => (id.to_string(), None),
        },
    };
    Item::Cmd { label, id: id.to_string(), params, shortcut }
}

fn parse_entries(entries: &[&str]) -> Vec<Item> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < entries.len() {
        let e = entries[i];
        i += 1;
        if e == "-" {
            out.push(Item::Sep);
        } else if let Some(name) = e.strip_prefix('>') {
            // Find the matching `<` (submenus nest).
            let start = i;
            let mut depth = 1;
            while i < entries.len() {
                if entries[i].starts_with('>') {
                    depth += 1;
                } else if entries[i] == "<" {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                i += 1;
            }
            let sub = parse_entries(&entries[start..i]);
            i += 1;
            out.push(Item::Sub(name.to_string(), sub));
        } else if e == "ui:window.panel" {
            let items = crate::dock::DOCK_TABS
                .iter()
                .chain(crate::dock::ICON_PANELS)
                .map(|(id, label, _)| Item::Cmd { label: label.to_string(), id: "window.panel".into(), params: json!({"panel": id}), shortcut: None })
                .collect();
            out.push(Item::Sub("Panels".into(), items));
        } else if e == "ui:window.workspace" {
            // Shown as InDesign's "[Essentials]"; Reset names the current workspace (see `menu_items`).
            let mut items: Vec<Item> = WORKSPACES
                .iter()
                .map(|w| Item::Cmd { label: w.to_string(), id: "window.workspace".into(), params: json!({"name": w}), shortcut: None })
                .collect();
            items.push(Item::Sep);
            items.push(cmd_item("window.resetWorkspace", Value::Null));
            items.push(cmd_item("window.newWorkspace", Value::Null));
            out.push(Item::Sub("Workspace".into(), items));
        } else if e == "ui:window.brightness" {
            let items = crate::theme::Brightness::ALL
                .iter()
                .map(|b| Item::Cmd {
                    label: b.label().to_string(),
                    id: "window.brightness".into(),
                    params: json!({"brightness": b.id()}),
                    shortcut: None,
                })
                .collect();
            out.push(Item::Sub("Interface Color Theme".into(), items));
        } else {
            let (_, id) = e.split_once(':').unwrap_or(("cmd", e));
            // `id|Label|{params}`: a fixed-parameter variant of a command.
            let mut parts = id.splitn(3, '|');
            let id = parts.next().unwrap_or(id);
            match (parts.next(), parts.next().and_then(|j| serde_json::from_str::<Value>(j).ok())) {
                (Some(label), Some(params)) => {
                    let mut it = cmd_item(id, params);
                    if let Item::Cmd { label: l, .. } = &mut it {
                        *l = label.to_string();
                    }
                    out.push(it);
                }
                _ => out.push(cmd_item(id, Value::Null)),
            }
        }
    }
    out
}

/// The whole menu tree (InDesign order).
pub fn menu_tree() -> Vec<(&'static str, Vec<Item>)> {
    MENUS.iter().map(|(m, e)| (*m, parse_entries(e))).collect()
}

/// A row of the macOS application menu (the menu named after the app).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppMenuItem {
    /// Runs `command` (no parameters); `id` is the native menu item id, `label` goes through `i18n::tr`.
    Cmd {
        id: &'static str,
        label: &'static str,
        command: &'static str,
    },
    Sep,
    /// Interface language: a submenu with each UI language in its own name, the current one
    /// checked (`app.language {lang}`).
    Language,
    Services,
    Hide,
    HideOthers,
    ShowAll,
}

/// The macOS application menu. Settings… is `app.preferences`, which `native_menu_tree` leaves out of Edit.
pub const MAC_APP_MENU: &[AppMenuItem] = &[
    AppMenuItem::Cmd { id: "dc-about", label: "About DesignCraft", command: "help.about" },
    AppMenuItem::Cmd { id: "dc-updates", label: "Check for Updates…", command: "help.checkForUpdates" },
    AppMenuItem::Cmd { id: "dc-discord", label: "Join the ArtCraft Discord…", command: "help.discord" },
    AppMenuItem::Sep,
    AppMenuItem::Cmd { id: "dc-settings", label: "Settings…", command: "app.preferences" },
    AppMenuItem::Language,
    AppMenuItem::Sep,
    AppMenuItem::Services,
    AppMenuItem::Sep,
    AppMenuItem::Hide,
    AppMenuItem::HideOthers,
    AppMenuItem::ShowAll,
    AppMenuItem::Sep,
    // Not the predefined Quit: it terminates the app at once, past the unsaved-changes prompt.
    AppMenuItem::Cmd { id: "dc-quit", label: "Quit DesignCraft", command: "app.quit" },
];

/// A label as the macOS menu bar library takes it: it reads "&" as a mnemonic marker and drops
/// it, and shows "&&" as one "&".
pub fn native_label(label: &str) -> String {
    label.replace('&', "&&")
}

/// The menu tree for the macOS menu bar: `menu_tree` without the commands `MAC_APP_MENU` holds,
/// so each command (and its shortcut) appears once, and with the standard Window › Minimize (⌘M)
/// at the top of the Window menu.
pub fn native_menu_tree() -> Vec<(&'static str, Vec<Item>)> {
    let in_app_menu = |id: &str| MAC_APP_MENU.iter().any(|m| matches!(m, AppMenuItem::Cmd { command, .. } if *command == id));
    let mut tree = menu_tree();
    for (title, entries) in &mut tree {
        without_commands(entries, &in_app_menu);
        if *title == "Window" {
            let minimize = Item::Cmd { label: "Minimize".into(), id: "window.minimize".into(), params: Value::Null, shortcut: Some("Cmd+M") };
            entries.splice(0..0, [minimize, Item::Sep]);
        }
    }
    tree
}

/// Drops the parameterless commands `drop` matches from `entries` (recursively), then any separator
/// the removal left at an end or next to another.
fn without_commands(entries: &mut Vec<Item>, drop: &dyn Fn(&str) -> bool) {
    let before = entries.len();
    entries.retain(|e| !matches!(e, Item::Cmd { id, params, .. } if params.is_null() && drop(id)));
    for e in entries.iter_mut() {
        if let Item::Sub(_, children) = e {
            without_commands(children, drop);
        }
    }
    if entries.len() == before {
        return;
    }
    let mut tidy: Vec<Item> = Vec::with_capacity(entries.len());
    for e in entries.drain(..) {
        if matches!(e, Item::Sep) && tidy.last().is_none_or(|l| matches!(l, Item::Sep)) {
            continue;
        }
        tidy.push(e);
    }
    if matches!(tidy.last(), Some(Item::Sep)) {
        tidy.pop();
    }
    *entries = tidy;
}

/// Check state of a toggle command (None = not a toggle).
pub fn checked(app: &DesignApp, id: &str, params: &Value) -> Option<bool> {
    Some(match id {
        "window.workspace" => params.get("name").and_then(Value::as_str) == Some(app.ui.workspace.as_str()),
        "view.rulers" => app.ui.rulers,
        "view.frameEdges" => app.ui.frame_edges,
        "view.overprintPreview" => app.ui.overprint_preview,
        "view.separations" => {
            let sep = app.ui.separations.as_ref();
            match (params.get("plate"), params.get("inkLimit")) {
                (Some(Value::Null), _) => sep.is_none(),
                (Some(Value::String(s)), _) if s == "all" => sep.is_some_and(|x| x.plate.is_none()),
                (Some(Value::String(s)), _) => sep.and_then(|x| x.plate.as_deref()).is_some_and(|p| p.eq_ignore_ascii_case(s)),
                (None, Some(v)) => sep.and_then(|x| x.ink_limit).is_some_and(|l| Some(f64::from(l)) == v.as_f64()),
                _ => false,
            }
        }
        "view.fastDisplay" => app.ui.display_quality == designcraft_render::DisplayQuality::Fast,
        "view.typicalDisplay" => app.ui.display_quality == designcraft_render::DisplayQuality::Typical,
        "view.highQualityDisplay" => app.ui.display_quality == designcraft_render::DisplayQuality::High,
        "view.guides" => app.ui.guides,
        "view.lockGuides" => app.ui.guides_locked,
        "view.lockColumnGuides" => app.session.active().is_some_and(|d| d.doc.settings.column_guides_locked),
        "view.snapToGuides" => app.ui.snap_to_guides,
        "view.snapToDocumentGrid" => app.ui.snap_to_document_grid,
        "view.smartGuides" => app.ui.smart_guides,
        "view.baselineGrid" => app.ui.baseline_grid,
        "view.documentGrid" => app.ui.document_grid,
        "view.textThreads" => app.ui.text_threads,
        "view.hiddenCharacters" => app.ui.hidden_characters,
        "view.taggedFrames" => app.ui.tagged_frames,
        "changes.track" => app.session.active().is_some_and(|d| d.doc.settings.track_changes),
        // Pressed, as the Bulleted and Numbered List buttons: every paragraph the command applies
        // to (selected text, cells or text frames) has it.
        "list.bullets" | "list.numbers" => {
            app.session.selection_has_list(if id == "list.bullets" { designcraft_doc::ListType::Bullets } else { designcraft_doc::ListType::Numbers })
        }
        "window.controlBar" => app.ui.control_bar,
        "window.split" => app.split,
        "window.newWindow" => app.second_window,
        "view.proofColors" => app.ui.proof_colors,
        "view.flattenerPreview" => app.ui.flattener_preview,
        "view.tagMarkers" => app.ui.tag_markers,
        "view.screenMode" => {
            let mode = match params.get("mode").and_then(Value::as_str) {
                Some("normal") => crate::ScreenMode::Normal,
                Some("preview") => crate::ScreenMode::Preview,
                Some("bleed") => crate::ScreenMode::Bleed,
                Some("slug") => crate::ScreenMode::Slug,
                Some("presentation") => crate::ScreenMode::Presentation,
                _ => return None,
            };
            app.ui.screen_mode == mode
        }
        "view.rotateSpread" => match params.get("angle").and_then(Value::as_i64) {
            Some(0) => app.session.active().is_some_and(|st| st.doc.spreads.iter().all(|sp| sp.pages.first().is_none_or(|p| p.view_rotation == 0))),
            _ => return None,
        },
        "view.proofSetup" => match (params.get("target").and_then(Value::as_str), params.get("simulatePaper")) {
            (Some(t), _) => app.ui.proof_setup.target.id() == t,
            (None, Some(_)) => app.ui.proof_setup.simulate_paper,
            _ => return None,
        },
        "edit.dynamicSpelling" => app.ui.dynamic_spelling,
        "app.language" => app.ui.language == params.get("lang").and_then(Value::as_str).unwrap_or(""),
        "app.flattener" => app.ui.flattener == params.get("preset").and_then(Value::as_str).unwrap_or(""),
        "window.taskBar" => app.ui.task_bar,
        "window.taskBarPin" => app.ui.task_bar_pin.is_some(),
        "window.toolsDoubleColumn" => app.ui.tools_double_column,
        "view.togglePreview" => app.ui.screen_mode == crate::ScreenMode::Preview,
        "view.togglePresentation" => app.ui.screen_mode == crate::ScreenMode::Presentation,
        "window.brightness" => params.get("brightness").and_then(Value::as_str) == Some(app.ui.brightness.id()),
        "type.storyDirection" => {
            let st = app.session.active()?;
            let sid = st
                .selection
                .text
                .map(|t| t.story)
                .or_else(|| st.selection.items.iter().find_map(|i| st.doc.item(*i)?.text_frame().map(|t| t.story)))?;
            st.doc.story(sid)?.vertical == params.get("vertical").and_then(Value::as_bool)?
        }
        _ => return None,
    })
}

/// Enablement for menu display (UI commands need a document unless they're app/window-level).
pub fn menu_enabled(app: &DesignApp, id: &str) -> bool {
    if waits_for_ime(app, id) {
        return false;
    }
    if is_window_control(id) {
        return window_control_available(app, id);
    }
    if matches!(id, "app.justificationDialog" | "app.hyphenationDialog") {
        return enabled(app, "type.para");
    }
    if id.starts_with("fit.") {
        return enabled(app, "object.fit");
    }
    if id == "guide.selectAll" && (!app.ui.guides || app.ui.guides_locked) {
        return false;
    }
    if id == "view.goBack" || id == "view.goForward" {
        return crate::canvas::can_go_through_history(app, if id == "view.goBack" { -1 } else { 1 });
    }
    if ui_label(id).is_some() {
        return app.session.active().is_some() || id.starts_with("app.") || id.starts_with("window.") || id.starts_with("help.");
    }
    enabled(app, id)
}

/// Close document `index` (the active one when `None`) the way the user asks for it: with the
/// tab's ×, File ▸ Close or its shortcut. A document with unsaved changes asks first, because
/// closing also discards its recovery data. Scripts and agents use `file.close`, which never asks.
pub fn close_document(app: &mut DesignApp, index: Option<usize>) {
    let Some(i) = index.or(app.session.active_index()) else { return };
    match app.session.documents().get(i) {
        Some(d) if d.is_dirty() => {
            app.ui.dialog = Some(crate::dialogs::Dialog::new("closeDocument", json!({"uid": d.uid, "title": d.title(), "discard": false})));
        }
        _ => {
            let _ = app.run("file.close", json!({"index": i}));
        }
    }
}

/// A menu item was chosen. An engine command whose label ends in "…" and that takes parameters
/// opens a dialog built from its parameter documentation (see [`crate::dialogs::command_fields`]).
pub fn activate(app: &mut DesignApp, id: &str, params: &Value) {
    if waits_for_ime(app, id) {
        return;
    }
    let was_find = app.ui.dialog.as_ref().is_some_and(|d| d.id == "findChange");
    activate_inner(app, id, params);
    if was_find && app.ui.dialog.as_ref().is_none_or(|d| d.id != "findChange") {
        app.session.reset_find_scope();
    }
}

fn activate_inner(app: &mut DesignApp, id: &str, params: &Value) {
    // Object › Effects › Drop Shadow… (and its shortcut): the Effects dialog on that page.
    if params.is_null()
        && let Some(page) = crate::effects_dialog::page_of_command(id)
    {
        crate::effects_dialog::open(app, page);
        return;
    }
    if params.is_null() && id == "swatch.addColorTheme" {
        crate::dialogs::open_color_theme(app, json!({}));
        return;
    }
    if params.is_null() && id == "file.close" {
        close_document(app, None);
        return;
    }
    if params.is_null() && id == "layout.documentSetup" {
        crate::dialogs::open_document_setup(app);
        return;
    }
    if params.is_null() && id == "file.info" {
        crate::dialogs::open_file_info(app);
        return;
    }
    if params.is_null() && id == "toc.generate" {
        crate::dialogs::open_toc(app);
        return;
    }
    if params.is_null() && id == "file.print" {
        crate::dialogs::open_print(app);
        return;
    }
    if params.is_null() && id == "layout.section" {
        crate::section_options::open(app);
        return;
    }
    if params.is_null() && id == "layout.createGuides" {
        crate::dialogs::open_create_guides(app);
        return;
    }
    if params.is_null() && id == "layout.marginsAndColumns" {
        crate::dialogs::open_margins_and_columns(app);
        return;
    }
    if params.is_null() && id == "object.textFrameOptions" {
        app.ui.dialog = Some(crate::dialogs::Dialog::new("textFrameOptions", json!({})));
        return;
    }
    // New Paragraph/Character Style…: the style options for the new style.
    if params.is_null() && matches!(id, "style.paragraph.create" | "style.character.create") {
        crate::dialogs::open_new_style(app, id == "style.paragraph.create");
        return;
    }
    // Paragraph/Character Style Options…: those of the selected text's style.
    if params.is_null() && matches!(id, "style.paragraph.edit" | "style.character.edit") {
        let para = id == "style.paragraph.edit";
        let cur = crate::panels::text_attrs(app).and_then(|a| a[if para { "paragraphStyle" } else { "characterStyle" }].as_str().map(str::to_string));
        if let Some(name) = cur {
            crate::dialogs::open_style_options(app, para, &name);
            return;
        }
    }
    if params.is_null()
        && ui_label(id).is_none()
        && let Some(c) = designcraft_engine::find_command(id)
        && !crate::dialogs::command_fields(c.params).is_empty()
        && (c.label.ends_with('…') || crate::dialogs::command_fields(c.params).iter().any(|f| !f.optional))
    {
        app.ui.dialog = Some(crate::dialogs::Dialog::new(&format!("cmd:{id}"), json!({})));
        return;
    }
    // Select All Guides: on the spread in view, while guides are shown and not locked.
    if params.is_null() && id == "guide.selectAll" {
        if app.ui.guides
            && !app.ui.guides_locked
            && let Some(spread) = guide_spread(app)
        {
            let _ = app.run(id, json!({"spread": spread}));
        }
        return;
    }
    let p = if params.is_null() { json!({}) } else { params.clone() };
    // Update Link asks before replacing edits made to linked text in the layout.
    if id == "links.update" {
        crate::panels::properties::update_links(app, p);
        return;
    }
    let _ = app.run(id, p);
}

/// The spread the guide commands of the menus act on: the one in view (a parent spread while
/// parents are edited).
fn guide_spread(app: &DesignApp) -> Option<Value> {
    let st = app.session.active()?;
    let page = crate::canvas::current_page(app).unwrap_or(0);
    Some(if st.editing_parents {
        let layout = designcraft_tools::CanvasLayout::new(&st.doc, true);
        let shown = crate::canvas::current_slot(app, &layout).and_then(|i| layout.slots.get(i)).map(|s| s.spread);
        json!(shown.unwrap_or(designcraft_doc::SpreadRef::Parent(0)))
    } else {
        json!(st.doc.page_loc(page).map_or(0, |l| l.0))
    })
}

/// A menu button whose popup stays within the viewport and scrolls when needed.
/// All menu buttons share this wrapper, including workspace, zoom and panel menus.
pub fn menu_button<'a, R>(
    ui: &mut egui::Ui,
    atoms: impl egui::IntoAtoms<'a>,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<Option<R>> {
    use egui::containers::menu::{MenuButton, MenuConfig};

    // A menu opens from a button already shown in the previous frame. Base its height on
    // that anchor, not the popup's eventual position, so placement cannot oscillate.
    let button = ui.ctx().read_response(ui.next_auto_id()).map(|response| response.rect);
    let max_height = menu_max_height(ui, button);
    let contents = |ui: &mut egui::Ui| {
        // Override the popup's initial available height, including when it opens upwards.
        ui.set_max_height(max_height);
        let mut scroll = egui::ScrollArea::vertical().max_height(max_height).min_scrolled_height(1.0);
        if ui.is_sizing_pass() {
            // Closing and reopening a menu starts at its first command again.
            scroll = scroll.vertical_scroll_offset(0.0);
        }
        scroll.show(ui, add_contents).inner
    };
    // Scrollbar clicks must not dismiss the popup; command rows close it explicitly.
    let config = MenuConfig::find(ui).close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside);
    let (response, inner) = if egui::containers::menu::is_in_menu(ui) {
        clipped_submenu(ui, atoms, config, contents)
    } else {
        MenuButton::new(atoms).config(config).ui(ui, contents)
    };
    egui::InnerResponse { response, inner: inner.map(|response| response.inner) }
}

/// The tallest menu that fits on either side of its anchor, including the popup frame.
fn menu_max_height(ui: &egui::Ui, button: Option<egui::Rect>) -> f32 {
    let screen = ui.ctx().content_rect();
    let space = match button {
        // Submenus open beside their row, aligned with its top or bottom.
        Some(rect) if egui::containers::menu::is_in_menu(ui) => (screen.bottom() - rect.top()).max(rect.bottom() - screen.top()),
        Some(rect) => (screen.bottom() - rect.bottom()).max(rect.top() - screen.top()),
        None => screen.height(),
    };
    let chrome = egui::Frame::menu(ui.style()).total_margin().sum().y + 8.0;
    (space.min(screen.height()) - chrome).max(1.0)
}

/// egui's submenu hover test uses the full row rect, even outside a scroll area's clip.
fn clipped_submenu<'a, R>(
    ui: &mut egui::Ui,
    atoms: impl egui::IntoAtoms<'a>,
    config: egui::containers::menu::MenuConfig,
    contents: impl FnOnce(&mut egui::Ui) -> R,
) -> (egui::Response, Option<egui::InnerResponse<R>>) {
    use egui::containers::menu::{MenuState, SubMenu, SubMenuButton};

    let id = SubMenu::id_from_widget_id(ui.next_auto_id());
    let open = MenuState::from_ui(ui, |state, _| state.open_item == Some(id));
    let inactive = ui.style().visuals.widgets.inactive;
    if open {
        ui.style_mut().visuals.widgets.inactive = ui.style().visuals.widgets.open;
    }
    let button = SubMenuButton::new(atoms).config(config);
    let response = ui.add(button.button);
    ui.style_mut().visuals.widgets.inactive = inactive;
    let clip = ui.clip_rect();
    if !response.rect.intersect(clip).is_positive() {
        if open {
            MenuState::from_ui(ui, |state, _| state.open_item = None);
        }
        return (response, None);
    }
    // Undo the half-spacing expansion used by SubMenu::show after clamping to the clip.
    let hover_margin = ui.spacing().item_spacing / 2.0;
    let mut clipped = response.clone();
    clipped.rect = response.rect.expand2(hover_margin).intersect(clip).shrink2(hover_margin);
    clipped.interact_rect = response.interact_rect.intersect(clip);
    let inner = button.sub_menu.show(ui, &clipped, contents);
    (response, inner)
}

/// A native menu bar item was chosen (macOS): like [`activate`], except that the menu bar takes
/// ⌘A, ⌘C, ⌘X, ⌘V, ⌘Z and ⇧⌘Z before the window sees them, so while a text field has keyboard
/// focus, Select All, Copy, Cut, Paste, Undo and Redo act on that field, as in any Mac app.
pub fn activate_native(app: &mut DesignApp, ctx: &egui::Context, id: &str, params: &Value) {
    if params.is_null() && ctx.text_edit_focused() && forward_to_text_edit(app, ctx, id) {
        return;
    }
    // The canvas text editor is not an egui TextEdit, so its clipboard events must pass through
    // egui for the desktop integration to read/write the macOS clipboard.
    if params.is_null() && app.session.wants_text() && matches!(id, "edit.copy" | "edit.cut" | "edit.paste") && forward_to_text_edit(app, ctx, id) {
        return;
    }
    activate(app, id, params);
}

/// Whether a native menu bar item can be chosen: as in the in-window menus, but the editing
/// commands a focused text field takes are always available to it.
pub fn native_menu_enabled(app: &DesignApp, ctx: &egui::Context, id: &str) -> bool {
    (ctx.text_edit_focused() && TEXT_FIELD_COMMANDS.contains(&id)) || menu_enabled(app, id)
}

/// The editing commands a focused text field takes from the native menu bar.
const TEXT_FIELD_COMMANDS: [&str; 6] = ["edit.selectAll", "edit.copy", "edit.cut", "edit.paste", "edit.undo", "edit.redo"];

/// Hand a standard editing command to the focused text field as next frame's input; false when
/// `id` isn't one of them.
fn forward_to_text_edit(app: &mut DesignApp, ctx: &egui::Context, id: &str) -> bool {
    let key = |key: egui::Key, modifiers: egui::Modifiers| {
        [true, false].map(|pressed| egui::Event::Key { key, physical_key: None, pressed, repeat: false, modifiers })
    };
    match id {
        "edit.selectAll" => app.synthetic.extend(key(egui::Key::A, egui::Modifiers::COMMAND)),
        "edit.undo" => app.synthetic.extend(key(egui::Key::Z, egui::Modifiers::COMMAND)),
        "edit.redo" => app.synthetic.extend(key(egui::Key::Z, egui::Modifiers::COMMAND | egui::Modifiers::SHIFT)),
        "edit.copy" => app.synthetic.push(egui::Event::Copy),
        "edit.cut" => app.synthetic.push(egui::Event::Cut),
        // Only the integration reads the system clipboard: it answers with an `Event::Paste`.
        "edit.paste" => ctx.send_viewport_cmd(egui::ViewportCommand::RequestPaste),
        _ => return false,
    }
    ctx.request_repaint();
    true
}

/// The menu bar contents (inside the app bar; macOS uses the native menu instead).
pub fn menu_bar(app: &mut DesignApp, ui: &mut egui::Ui) {
    let lang = app.ui.language.clone();
    let mut menus = menu_tree();
    if crate::i18n::is_rtl(&lang) {
        menus.reverse();
    }
    for (menu, entries) in menus {
        menu_button(ui, crate::rtl::widget(ui, crate::i18n::tr(&lang, menu)), |ui| menu_column(app, ui, &[(menu, entries)]));
    }
}

/// One menu's rows, as the menu bar and the canvas context menus show them: runs of items, each
/// with the menu path its rows are hidden by (Edit › Menus).
fn menu_column<P: AsRef<str>>(app: &mut DesignApp, ui: &mut egui::Ui, runs: &[(P, Vec<Item>)]) {
    let lang = app.ui.language.clone();
    let rtl = crate::i18n::is_rtl(&lang);
    if rtl {
        ui.set_max_width(320.0);
    }
    ui.with_layout(egui::Layout::top_down_justified(if rtl { egui::Align::Max } else { egui::Align::Min }), |ui| {
        ui.set_min_width(240.0);
        let mut hidden = 0;
        for (path, items) in runs {
            hidden += menu_items(app, ui, items, path.as_ref());
        }
        if hidden > 0 && !app.ui.show_full_menus {
            ui.separator();
            if ui.button(crate::rtl::widget(ui, crate::i18n::tr(&lang, "Show All Menu Items"))).clicked() {
                app.ui.show_full_menus = true;
            }
        }
    });
}

/// What a right-click on the canvas opens a menu for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContextMenu {
    /// The selected objects.
    Object,
    /// The text being edited with the Type tool.
    Text,
    /// The page or pasteboard, where nothing was hit.
    Canvas,
}

// Canvas context menus, in the menu bar's entry syntax; `menu:Menu/Submenu` inserts that menu bar
// submenu as it is.
const CONTEXT_OBJECT: &[&str] = &[
    "cmd:edit.undo",
    "cmd:edit.redo",
    "-",
    "cmd:edit.cut",
    "cmd:edit.copy",
    "cmd:edit.paste",
    "cmd:edit.pasteInto",
    "cmd:edit.pasteInPlace",
    "-",
    ">Transform",
    "cmd:transform.rotate|Rotate 90° Clockwise|{\"angle\": -90}",
    "cmd:transform.rotate|Rotate 90° Counterclockwise|{\"angle\": 90}",
    "cmd:transform.rotate|Rotate 180°|{\"angle\": 180}",
    "-",
    "cmd:transform.flip|Flip Horizontal|{\"axis\": \"horizontal\"}",
    "cmd:transform.flip|Flip Vertical|{\"axis\": \"vertical\"}",
    "-",
    "cmd:transform.clear",
    "<",
    "menu:Object/Arrange",
    "menu:Object/Select",
    "-",
    "cmd:object.group",
    "cmd:object.ungroup",
    "cmd:object.lock",
    "cmd:object.unlockAll",
    "cmd:object.hide",
    "cmd:object.showAll",
    "-",
];
/// Added to the object menu when a graphic frame is selected.
const CONTEXT_GRAPHIC_FRAME: &[&str] = &["menu:Object/Fitting"];
const CONTEXT_OBJECT_TAIL: &[&str] = &["menu:Object/Content", "menu:Object/Effects"];
/// Added to the object menu when a text frame is selected.
const CONTEXT_TEXT_FRAME: &[&str] = &["-", "cmd:type.fillWithPlaceholder", "cmd:object.textFrameOptions"];
const CONTEXT_TEXT: &[&str] = &[
    "cmd:edit.cut",
    "cmd:edit.copy",
    "cmd:edit.paste",
    "cmd:edit.pasteWithoutFormatting",
    "-",
    "menu:Type/Insert Special Character",
    "menu:Type/Insert White Space",
    "menu:Type/Insert Break Character",
    "cmd:type.fillWithPlaceholder",
    "-",
    "menu:Type/Change Case",
    "-",
    "ui:view.hiddenCharacters",
    "ui:app.storyEditor",
    "-",
    "cmd:edit.selectAll",
];
const CONTEXT_CANVAS: &[&str] = &[
    "cmd:edit.undo",
    "cmd:edit.redo",
    "-",
    "cmd:edit.paste",
    "cmd:edit.pasteInPlace",
    "-",
    "ui:view.zoomIn",
    "ui:view.zoomOut",
    "ui:view.fitPage",
    "ui:view.fitSpread",
    "ui:view.actualSize",
    "-",
    "ui:view.rulers",
    "menu:View/Grids & Guides",
    "-",
    "cmd:edit.selectAll",
];

/// The menu-bar path context menu rows are keyed by (Edit › Menus doesn't list them); rows
/// inserted with `menu:` keep their menu bar path.
const CONTEXT_PATH: &str = "Context";

/// The entries of a canvas context menu for the current selection.
fn context_entries(app: &DesignApp, kind: ContextMenu) -> Vec<&'static str> {
    match kind {
        ContextMenu::Object => {
            let (graphic, text) = app.session.active().map_or((false, false), |st| {
                let items = || st.selection.items.iter().filter_map(|i| st.doc.item(*i));
                (items().any(|it| matches!(it.content, designcraft_doc::Content::Graphic(_))), items().any(|it| it.is_text_frame()))
            });
            let mut v = CONTEXT_OBJECT.to_vec();
            if graphic {
                v.extend(CONTEXT_GRAPHIC_FRAME);
            }
            v.extend(CONTEXT_OBJECT_TAIL);
            if text {
                v.extend(CONTEXT_TEXT_FRAME);
            }
            v
        }
        ContextMenu::Text => CONTEXT_TEXT.to_vec(),
        ContextMenu::Canvas => CONTEXT_CANVAS.to_vec(),
    }
}

/// Parses context menu entries: the menu bar's syntax plus `menu:Menu/Submenu` (not inside a `>`
/// submenu). Returns runs of items with the menu path they are hidden by: `Context`, or for an
/// inserted submenu its menu bar menu, so hiding a row in the menu bar hides it here too.
fn context_parse(entries: &[&str]) -> Vec<(String, Vec<Item>)> {
    let tree = menu_tree();
    let mut out = Vec::new();
    let mut run: Vec<&str> = Vec::new();
    for e in entries {
        if let Some(path) = e.strip_prefix("menu:") {
            out.push((CONTEXT_PATH.to_owned(), parse_entries(&std::mem::take(&mut run))));
            let menu = path.rsplit_once('/').map_or(path, |(menu, _)| menu);
            out.push((menu.to_owned(), menu_bar_submenu(&tree, path).into_iter().collect()));
        } else {
            run.push(e);
        }
    }
    out.push((CONTEXT_PATH.to_owned(), parse_entries(&run)));
    out
}

/// The menu bar's submenu at `Menu/Submenu`.
fn menu_bar_submenu(tree: &[(&str, Vec<Item>)], path: &str) -> Option<Item> {
    let (menu, sub) = path.split_once('/')?;
    tree.iter().find(|(m, _)| *m == menu)?.1.iter().find(|it| matches!(it, Item::Sub(name, _) if name == sub)).cloned()
}

/// A canvas context menu: the menu bar's rows (labels, shortcuts, check marks, greyed when they
/// can't run), scrolling when taller than the window.
pub fn context_menu(app: &mut DesignApp, ui: &mut egui::Ui, kind: ContextMenu) {
    let runs = context_parse(&context_entries(app, kind));
    let max_height = menu_max_height(ui, None);
    let mut scroll = egui::ScrollArea::vertical().max_height(max_height).min_scrolled_height(1.0);
    if ui.is_sizing_pass() {
        scroll = scroll.vertical_scroll_offset(0.0);
    }
    scroll.show(ui, |ui| menu_column(app, ui, &runs));
}

/// The key a menu item is hidden by (Edit › Menus).
pub fn menu_key(menu: &str, label: &str) -> String {
    format!("{menu}/{label}")
}

/// Draws a menu's items; returns how many were hidden.
fn menu_items(app: &mut DesignApp, ui: &mut egui::Ui, items: &[Item], path: &str) -> usize {
    let mut hidden = 0;
    for it in items {
        match it {
            Item::Sep => {
                ui.separator();
            }
            Item::Sub(name, children) => {
                let sub = format!("{path}/{name}");
                let shown = crate::i18n::tr(&app.ui.language, name).to_owned();
                menu_button(ui, crate::rtl::widget(ui, shown), |ui| {
                    if sub == OPEN_RECENT {
                        recent_menu_items(app, ui);
                    }
                    hidden += menu_items(app, ui, children, &sub);
                });
            }
            Item::Cmd { id, .. } if !shown(app, id) => {}
            Item::Cmd { label, .. } if !app.ui.show_full_menus && app.ui.hidden_menu_items.contains(&menu_key(path, label)) => hidden += 1,
            Item::Cmd { label, id, params, shortcut: _ } => {
                let lang = &app.ui.language;
                let label = match id.as_str() {
                    "window.workspace" => format!("[{}]", crate::i18n::workspace_name(lang, label)),
                    "window.resetWorkspace" => format!("{} {}", crate::i18n::tr(lang, "Reset"), crate::i18n::workspace_name(lang, &app.ui.workspace)),
                    _ => crate::i18n::tr(lang, label).to_owned(),
                };
                let text = match checked(app, id, params) {
                    Some(true) => format!("✓ {label}"),
                    Some(false) => format!("   {label}"),
                    None => label.to_owned(),
                };
                let mut b = egui::Button::new(crate::rtl::widget(ui, text));
                if let Some(sc) = shortcut_label(app, id, params) {
                    b = b.shortcut_text(sc);
                }
                if ui.add_enabled(menu_enabled(app, id), b).clicked() {
                    activate(app, id, params);
                    ui.close();
                }
            }
        }
    }
    hidden
}

/// The File › Open Recent submenu, whose files come before its fixed items.
pub const OPEN_RECENT: &str = "File/Open Recent";

/// A recent file's menu label: its file name.
pub fn recent_label(path: &str) -> String {
    std::path::Path::new(path).file_name().map_or_else(|| path.to_string(), |n| n.to_string_lossy().to_string())
}

/// File › Open Recent's files (`app.openRecent {index}`), then a separator; nothing when empty.
pub fn recent_items(app: &DesignApp) -> Vec<Item> {
    let mut out: Vec<Item> = app
        .session
        .prefs
        .recent_files
        .iter()
        .enumerate()
        .map(|(i, path)| Item::Cmd { label: recent_label(path), id: "app.openRecent".into(), params: json!({"index": i}), shortcut: None })
        .collect();
    if !out.is_empty() {
        out.push(Item::Sep);
    }
    out
}

/// The recent files at the top of File › Open Recent: file names (not translated), the full path
/// as tooltip, dimmed when the file is gone (choosing it then says so).
fn recent_menu_items(app: &mut DesignApp, ui: &mut egui::Ui) {
    let dim = crate::theme::Tokens::get(ui.ctx()).text_dim;
    for it in recent_items(app) {
        match it {
            Item::Cmd { label, id, params, .. } => {
                let path = params["index"].as_u64().and_then(|i| app.session.prefs.recent_files.get(i as usize)).cloned().unwrap_or_default();
                let mut text = egui::RichText::new(label);
                if !crate::chrome::recent_file_stat(ui.ctx(), &path).exists {
                    text = text.color(dim);
                }
                if ui.button(crate::rtl::widget(ui, text)).on_hover_text(&path).clicked() {
                    activate(app, &id, &params);
                    ui.close();
                }
            }
            Item::Sep => {
                ui.separator();
            }
            Item::Sub(..) => {}
        }
    }
}

/// Every menu item as (menu path, label), for Edit › Menus.
pub fn all_menu_items() -> Vec<(String, String)> {
    fn walk(items: &[Item], path: &str, out: &mut Vec<(String, String)>) {
        for it in items {
            match it {
                Item::Sub(name, children) => walk(children, &format!("{path}/{name}"), out),
                Item::Cmd { label, .. } => out.push((path.to_string(), label.clone())),
                Item::Sep => {}
            }
        }
    }
    let mut out = Vec::new();
    for (menu, entries) in menu_tree() {
        walk(&entries, menu, &mut out);
    }
    out
}

/// The command a paste event stands for besides plain Paste. egui-winit turns every ⌘+V chord into
/// `Event::Paste` with the clipboard's text and drops the key press, and the keymap reads the chord
/// from the frame's final modifiers, which miss a chord released within the frame; the modifiers
/// held when it went down tell them apart. `None` when the chord is another one, or the native
/// menu's accelerator takes it.
#[cfg(not(target_arch = "wasm32"))]
pub fn paste_chord_command(app: &DesignApp, modifiers: egui::Modifiers) -> Option<&'static str> {
    let id = "edit.pasteInPlace";
    if app.native_menu && app.keymap.overrides().get(id).is_none() {
        return None;
    }
    let platform = app.keymap.platform();
    let held = craft_keymap::Mods::from_egui(&modifiers, platform);
    let pressed = craft_keymap::Chord::new(held, craft_keymap::Key::V);
    // A shortcut the user set is the menu bar's key equivalent once the bar carries it.
    let on_bar = app
        .native_shortcuts
        .get(id)
        .and_then(|sc| craft_keymap::Binding::parse(sc).ok())
        .is_some_and(|b| !b.trigger.double && b.trigger.chord.effective(platform) == pressed);
    if app.native_menu && on_bar {
        return None;
    }
    let bound =
        app.keymap.bindings(&craft_keymap::CommandRef::new(id)).iter().any(|b| !b.trigger.double && b.trigger.chord.effective(platform) == pressed);
    bound.then_some(id)
}

/// Global keyboard shortcuts: menu commands, panels and tools, through the keymap.
pub fn shortcuts(app: &mut DesignApp, ctx: &egui::Context) {
    app.sync_keymap();
    // Only a focused text field takes the keys: the canvas (or a button) having focus after a click
    // must not swallow tool shortcuts until Esc clears it.
    if ctx.text_edit_focused() || keys_taken(app, ctx) {
        return;
    }
    let typing = app.session.wants_text();
    let keys = ctx.input_mut(|i| take_keys(app, i, typing));
    run_keys(app, keys, typing);
}

/// Keys that a floating panel in its own OS window left over (craft-ui's
/// `DockOutput::forwarded_events`, with the modifiers held in that window): they run the main
/// window's shortcuts, clipboard and tool keys in the frame they arrive, so the keyboard works
/// while a floating window has focus. craft-ui forwards nothing while one of that window's
/// widgets has keyboard focus.
pub fn forwarded_shortcuts(app: &mut DesignApp, ctx: &egui::Context, events: Vec<egui::Event>, modifiers: egui::Modifiers) {
    if events.is_empty() || keys_taken(app, ctx) {
        return;
    }
    app.sync_keymap();
    let typing = app.session.wants_text();
    // The keymap dispatches from an input state: the main window's, with the floating window's
    // keys and modifiers in place of its own.
    let mut input = ctx.input(|i| i.clone());
    input.events = events;
    input.modifiers = modifiers;
    let keys = take_keys(app, &mut input, typing);
    run_keys(app, keys, typing);
    crate::canvas::forwarded_keys(app, input.events, modifiers);
}

/// Whether something other than the shortcuts has the keyboard: a dialog or Quick Apply, an input
/// method composing at the text caret, an open canvas context menu (Escape closes it, as a
/// native menu would) or the menu search.
fn keys_taken(app: &DesignApp, ctx: &egui::Context) -> bool {
    app.ui.dialog.is_some()
        || app.ui.palette.is_some()
        || app.session.tool_composing()
        || crate::canvas::context_menu_open(ctx)
        || crate::chrome::search_had_focus(ctx)
}

/// The key presses [`take_keys`] took from an input, run by [`run_keys`] once the input is
/// released: commands may read the context.
struct Keys {
    /// Presentation keys in order: `None` leaves Presentation, `Some(step)` moves through the spreads.
    presentation: Vec<Option<isize>>,
    hits: Vec<craft_keymap::egui::Hit>,
    /// An Esc press that nothing took.
    escape: bool,
}

/// Takes the presentation keys and the command, panel and tool shortcuts out of `input`'s key
/// presses.
fn take_keys(app: &mut DesignApp, input: &mut egui::InputState, typing: bool) -> Keys {
    let presentation = if app.ui.screen_mode == ScreenMode::Presentation { presentation_keys(input) } else { Vec::new() };
    let contexts: &[&str] = if typing { &[crate::keys::TEXT_CONTEXT] } else { &[] };
    let active = craft_keymap::Active { panel: None, contexts, typing };
    let native = &app.native_shortcuts;
    // Presses the dispatcher leaves alone: Clear (the active tool deletes) and the keys the
    // native menu's accelerators already handled. An item holds one key equivalent, its
    // command's first shortcut in effect, and fires only for it exactly: the Plus key standing
    // in for = never reaches it, and a command's other bindings run here.
    // Copy, Cut and Paste stay too (the dispatcher's default): the canvas and text tools
    // handle them, and Paste carries the text.
    let accept = |hit: &craft_keymap::egui::Hit| {
        let c = &hit.command;
        let on_bar =
            || c.params.is_null() && native.get(&c.id).and_then(|sc| craft_keymap::Binding::parse(sc).ok()).is_some_and(|b| b.trigger == hit.trigger);
        let leave = hit.exact && hit.cycle.is_empty() && (c.id == "edit.clear" || on_bar());
        !leave
    };
    let hits = app.key_dispatcher.dispatch_filtered(input, &app.keymap, &active, accept);
    let escape = input.events.iter().any(
        |e| matches!(e, egui::Event::Key { key: egui::Key::Escape, pressed: true, repeat: false, modifiers, .. } if !modifiers.command && !modifiers.alt),
    );
    Keys { presentation, hits, escape }
}

/// Runs what [`take_keys`] took, then Esc's return to the Selection tool.
fn run_keys(app: &mut DesignApp, keys: Keys, typing: bool) {
    for step in keys.presentation {
        match step {
            None => {
                let _ = app.run("view.togglePresentation", json!({}));
            }
            Some(step) => crate::canvas::step_spread(app, step),
        }
    }
    for hit in keys.hits {
        if !hit.cycle.is_empty() {
            // Tools sharing a key: the press after the group's current tool selects the next one.
            let current = app.session.tool_id();
            let tools: Vec<&str> = hit.cycle.iter().filter_map(|c| c.params.get("tool").and_then(Value::as_str)).collect();
            let next = tools.iter().position(|t| *t == current).and_then(|i| tools.get((i + 1) % tools.len())).or(tools.first());
            if let Some(tool) = next.map(|t| t.to_string()) {
                log::debug!("tool shortcut -> {tool}");
                app.select_tool(&tool);
            }
        } else if hit.command.id == crate::keys::TOOL_COMMAND
            && let Some(tool) = hit.command.params.get("tool").and_then(Value::as_str).map(str::to_string)
        {
            log::debug!("tool shortcut -> {tool}");
            app.select_tool(&tool);
        } else {
            // Like choosing the menu item: "…" commands open their dialog.
            activate(app, &hit.command.id, &hit.command.params);
        }
    }
    // Esc with no shortcut of its own goes back to the Selection tool.
    if keys.escape && !typing && app.session.tool_id() != "selection" && !app.session.tool_busy() && app.ui.flyout.is_none() {
        app.select_tool("selection");
    }
}

/// Takes the Presentation keys out of `input`. Presentation hides every control, so Esc always
/// leaves it, and only that: the canvas must not also see the press and switch tools or deselect.
/// The arrow keys, Page Up/Down, Home and End go through the spreads.
fn presentation_keys(input: &mut egui::InputState) -> Vec<Option<isize>> {
    let mut steps = Vec::new();
    for e in input.events.clone() {
        let egui::Event::Key { key, pressed: true, modifiers, repeat: false, .. } = e else { continue };
        if key == egui::Key::Escape {
            input.consume_key(modifiers, key);
            steps.push(None);
            continue;
        }
        if !modifiers.command && !modifiers.alt {
            let step = match key {
                egui::Key::ArrowRight | egui::Key::ArrowDown | egui::Key::PageDown => Some(1),
                egui::Key::ArrowLeft | egui::Key::ArrowUp | egui::Key::PageUp => Some(-1),
                egui::Key::Home => Some(isize::MIN),
                egui::Key::End => Some(isize::MAX),
                _ => None,
            };
            if let Some(step) = step {
                input.consume_key(modifiers, key);
                steps.push(Some(step));
            }
        }
    }
    steps
}

/// Quick Apply (⌘Return): the document's styles and every command, searched by name.
pub fn palette(app: &mut DesignApp, ctx: &egui::Context) {
    let Some(mut q) = app.ui.palette.clone() else { return };
    let mut close = false;
    let mut run: Option<(String, Value)> = None;
    egui::Modal::new(egui::Id::unique("palette")).show(ctx, |ui| {
        ui.set_width(480.0);
        let r = ui.add(
            egui::TextEdit::singleline(&mut q)
                .hint_text(crate::rtl::widget(ui, crate::i18n::tr(&app.ui.language, "Search styles and commands…")))
                .desired_width(f32::INFINITY),
        );
        r.request_focus();
        let items = quick_apply_items(&app.session, &q, crate::cjk_features(app));
        // Only a fresh Return runs the first entry: ⌘Return pressed again or held down (repeats) is
        // the shortcut, not a choice.
        let enter = ui.input(|i| {
            i.events
                .iter()
                .any(|e| matches!(e, egui::Event::Key { key: egui::Key::Enter, pressed: true, repeat: false, modifiers, .. } if !modifiers.command))
        });
        egui::ScrollArea::vertical().max_height(340.0).show(ui, |ui| {
            for (i, it) in items.iter().enumerate() {
                let mut b = egui::Button::new(it.label.as_str()).frame(false);
                if let Some(sc) = &it.shortcut {
                    b = b.shortcut_text(shortcut_text(sc));
                } else if !it.kind.is_empty() {
                    b = b.shortcut_text(crate::i18n::tr(&app.ui.language, it.kind));
                }
                // The button aligns its label by the layout it's in: left, like the rows with a shortcut.
                let size = egui::vec2(460.0, 22.0);
                let row =
                    ui.allocate_ui_with_layout(size, egui::Layout::left_to_right(egui::Align::Center).with_main_align(egui::Align::Min), |ui| {
                        ui.add(b.min_size(size))
                    });
                if row.inner.clicked() || (i == 0 && enter) {
                    run = Some((it.id.clone(), it.params.clone()));
                }
            }
        });
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            close = true;
        }
    });
    app.ui.palette = if close || run.is_some() { None } else { Some(q) };
    if let Some((id, params)) = run {
        let _ = app.run(&id, params);
    }
}

/// One Quick Apply entry: a style to apply or a command to run.
#[derive(Clone, Debug, PartialEq)]
pub struct QuickItem {
    pub label: String,
    pub id: String,
    pub params: Value,
    pub shortcut: Option<String>,
    /// "Paragraph Style", "Character Style", "Object Style", or "" for commands.
    pub kind: &'static str,
}

/// Quick Apply matches (styles first, as InDesign lists them), at most 14.
pub fn quick_apply_items(session: &designcraft_engine::Session, query: &str, cjk: bool) -> Vec<QuickItem> {
    let ql = query.to_lowercase();
    let hit = |label: &str, id: &str| quick_match(&ql, &[label, id]);
    let mut out = Vec::new();
    if let Some(st) = session.active() {
        let styles = &st.doc.styles;
        let mut add = |name: &str, id: &str, kind: &'static str| {
            if name.starts_with('[') || !hit(name, "") {
                return;
            }
            out.push(QuickItem { label: name.to_string(), id: id.into(), params: json!({"name": name}), shortcut: None, kind });
        };
        for p in &styles.paragraph {
            add(&p.name, "style.paragraph.apply", "Paragraph Style");
        }
        for c in &styles.character {
            add(&c.name, "style.character.apply", "Character Style");
        }
        for o in &styles.object {
            add(&o.name, "style.object.apply", "Object Style");
        }
    }
    // Styles need a query, else they would crowd out the commands.
    if ql.is_empty() {
        out.clear();
    }
    let commands = quick_commands().filter(|(id, l, _)| hit(l, id) && (cjk || !CJK_ONLY.contains(id))).map(|(id, l, sc)| QuickItem {
        label: l.to_string(),
        id: id.to_string(),
        params: json!({}),
        shortcut: sc.map(str::to_string),
        kind: "",
    });
    out.extend(commands);
    out.truncate(14);
    out
}

/// UI commands search leaves out: they only run with a parameter search can't supply and have no
/// dialog to ask for it, or without one they reset a setting. Each stays in the menu, panel or
/// dialog that gives it the value (the workspace menus, Keyboard Shortcuts, Preferences…).
const NOT_SEARCHED: &[&str] = &[
    // The workspace to switch to or delete.
    "window.workspace",
    "window.deleteWorkspace",
    // A command and its shortcut, a menu item, a shortcut set; Reset Shortcuts clears every
    // custom shortcut, and the Keyboard Shortcuts dialog offers it.
    "window.setShortcut",
    "window.resetShortcuts",
    "window.importShortcuts",
    "window.hideMenuItem",
    // Without a value these reset the interface to 100% and the dark theme.
    "window.uiScale",
    "window.brightness",
    // A panel, and where it goes.
    "window.floatPanel",
    "window.dockPanel",
    "window.panel.layout",
    "window.panel.move",
    "window.panel.float",
    "window.panel.dock",
    "window.panel.activate",
    "window.panel.close",
    "window.panel.closeGroup",
    // A window edge, an app page, a recent file, a swatch or color group.
    "window.resize",
    "help.app",
    "app.openRecent",
    "app.swatchOptionsDialog",
    "app.colorGroupOptionsDialog",
    // Without a value these clear the flattener preset, go to Normal screen mode or 100% zoom.
    "app.flattener",
    "view.screenMode",
    "view.zoom",
    // Where to pin the Contextual Task Bar.
    "window.taskBarPin",
    // Shared docking commands DesignCraft doesn't provide, those that need a panel or a name,
    // and older ids that share a label with a searched docking command.
    "window.panel",
    "window.panel.window",
    "window.workspace.lock",
    "window.panel.closeOthers",
    "window.panel.minimize",
    "window.panel.iconic",
    "window.workspace.new",
    "window.workspace.delete",
    "ui.dock",
    "ui.dock.reset",
    "window.resetWorkspace",
];

/// The commands Quick Apply and the application bar's search find: (id, label, default shortcut).
fn quick_commands() -> impl Iterator<Item = (&'static str, &'static str, Option<&'static str>)> {
    designcraft_engine::command_specs()
        .iter()
        .filter(|c| !c.menu.is_empty() || c.shortcut.is_some())
        .map(|c| (c.id, c.label, c.shortcut))
        .chain(UI_COMMANDS.iter().filter(|c| !NOT_SEARCHED.contains(&c.0)).map(|c| (c.0, c.1, c.2)))
}

/// Whether any of `texts` contains the lowercased query (an empty query matches everything).
fn quick_match(query_lower: &str, texts: &[&str]) -> bool {
    query_lower.is_empty() || texts.iter().any(|t| t.to_lowercase().contains(query_lower))
}

/// The application bar's search: the commands whose label in the interface language, English
/// label or id contains `query`, labelled in the interface language with the shortcut in effect.
/// Nothing for an empty query; at most 14.
pub fn command_search(app: &DesignApp, query: &str) -> Vec<QuickItem> {
    let ql = query.trim().to_lowercase();
    if ql.is_empty() {
        return Vec::new();
    }
    let lang = app.ui.language.as_str();
    let mut out: Vec<QuickItem> = Vec::new();
    for (id, label, _) in quick_commands() {
        let shown = crate::i18n::tr(lang, label);
        if out.len() >= 14 || !self::shown(app, id) || out.iter().any(|it| it.id == id) || !quick_match(&ql, &[shown, label, id]) {
            continue;
        }
        out.push(QuickItem { label: shown.to_string(), id: id.to_string(), params: Value::Null, shortcut: shortcut_of(app, id), kind: "" });
    }
    out
}

#[cfg(test)]
mod tests;
