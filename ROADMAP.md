# DesignCraft roadmap

**Stage: pre-alpha** · next: alpha, ~5–7 h away; beta ~28 points and ~420–750 h beyond that (the core-workflow gate, not the percentage: ready for real work is already ~47%)

> **Last reviewed:** 2026-10-11 · **Last updated:** 2026-10-13 · **Change:** minor (File ▸ Open Recent; .indd and .indt open through indd-utils; text threads by hand; the alpha gate waits on Primary Text Frame page fixes; Japanese print, import and table controls localized; Live corners (#186); Optical kerning from outlines (#181); window and menu quit confirmation; failed writes preserve live save and collection state; spring-loaded Zoom navigation; orderly native exit recovery; Pages panel parents; cancelable macOS Dock Quit; text stroke behind fill; Based on Parent; object styles carry effects; table anchors keep their own paragraphs; grouped HTML/EPUB reading order; Links source paths; XLSX cached cell values retain numeric precision and text) · **Target:** Adobe InDesign 2026

DesignCraft aims at full Adobe InDesign parity, and to be better: faster, open (documented JSON
format + IDML), scriptable by agents (MCP), and available on the web.

## Headline numbers

| | Value | Kind |
|---|---|---|
| **Feature breadth** | **~80%** (69% of InDesign 21.6's in-scope menu items matched; 99% of our 275-row checklist) | measured bounds, estimate between |
| **Ready for real work** | **~47%** (range 42–52%) | estimated: weighted sum over the dimensions below |
| Mainstream practitioner | **~42%** (weekly core 60.7% × 0.88 interaction × 0.92 stability × 0.85 file exchange) | estimated |
| Essentials user | **~54%** (core features 72% × 0.90 launch × 0.88 discoverability × 0.95 files received) | estimated |
| Remaining to **beta** | **420–750 Opus 5.5 agent-hours** (~120–200 wall-clock hours with 4–5 agents) | estimated |
| Remaining to **full parity** | **1,150–1,950 Opus 5.5 agent-hours** | estimated |

| Audience | Ready | Opus 5.5 agent-hours to ~95% | Work that dominates |
|---|---|---|---|
| Full target (ready for real work) | ~47% | 1,150–1,950 (~70% parallel) | depth across all areas, scripting compatibility, localization, typography fidelity, INDD/IDML |
| Mainstream practitioner | ~42% | 500–850 (~65% parallel) | INDD and IDML real-file corpus, composition fidelity, weekly-area depth, PDF/X-1a and printing |
| Essentials user | ~54% | 135–250 (~50% parallel) | start-up robustness, context menus and copy/paste, threading, fonts, Windows printing |

**Why pre-alpha:** it fails one of InDesign's core workflows, the alpha gate in
[docs/roadmap.md](docs/roadmap.md#alpha-gate). Text threads by hand (click a port, then click or
draw a frame; thread and unthread commands), but Primary Text Frame autoflow adds pages users
can't delete (#165). Everything else in the gate works end to end on macOS (document setup,
styles, images, object editing, PDF export, save and reopen), and ready for real work (~47%) is
above the alpha bar, so the Primary Text Frame page fixes are the whole distance to alpha.
Beyond that, beta needs INDD and IDML proven on real files, composition compared with
InDesign's, and basic editing fixed on Windows and Linux. Details:
[docs/target-app-parity.md](docs/target-app-parity.md).

## By dimension

| Dimension | Ready | Hours | Doc |
|---|---|---|---|
| Features (depth) | 55% | 250–400 | [target-app-parity.md](docs/target-app-parity.md) |
| Typography and composition fidelity | 45% | 120–200 | [typography-parity.md](docs/typography-parity.md) |
| File formats | 40% | 150–250 + 10–30 (INDD corpus) | [file-format-parity.md](docs/file-format-parity.md) |
| UI/UX fidelity | 45% | 120–180 | [ui-parity.md](docs/ui-parity.md) |
| Stability | 45% | 60–100 | [gaps.md](docs/gaps.md) |
| Performance | 65% | 40–80 | [gaps.md](docs/gaps.md) |
| Localization | 45% | 120–180 | [localization-parity.md](docs/localization-parity.md) |
| Platforms | 55% | 40–80 | [hardware-parity.md](docs/hardware-parity.md) |
| Hardware | 45% | 20–40 | [hardware-parity.md](docs/hardware-parity.md) |
| Ecosystem (scripting, plug-ins) | 15% | 150–250 | [gaps.md](docs/gaps.md) |
| AI features | 15% | 60–120 | [gaps.md](docs/gaps.md) |

All "Ready" figures are estimates; the weights and evidence are in target-app-parity.md.

## Features

Breadth from [docs/parity-checklist.md](docs/parity-checklist.md) (`cargo xtask parity`,
measured); Ready estimated.

| Area | Breadth | Ready | Hours |
|---|---|---|---|
| Application shell & workspace | 99% | 55% | 40–60 |
| Documents, pages, spreads | 100% | 65% | 20–35 |
| Layers | 100% | 75% | 5–10 |
| Frames, shapes & paths | 100% | 60% | 25–40 |
| Transform | 100% | 70% | 10–20 |
| Fill, stroke, colour | 100% | 60% | 20–35 |
| Effects & transparency | 100% | 50% | 20–30 |
| Placing & links | 93% | 55% | 20–35 |
| Type & text frames | 99% | 45% | 40–60 |
| Typography | 100% | 45% | see typography-parity.md |
| Styles | 100% | 60% | 15–25 |
| Tables | 100% | 55% | 25–40 |
| Long documents | 100% | 45% | 30–50 |
| Interactivity & digital | 100% | 40% | 30–50 |
| Output & production | 92% | 55% | 40–70 |
| XML & automation | 100% | 35% | see Ecosystem |
| View & navigation | 100% | 65% | 10–20 |
| Undo, history, saving | 100% | 70% | 5–10 |
| Accessibility | 100% | 50% | 15–25 |

## Languages

Interface strings translated, of 1,724 measured. Detail: [docs/localization-parity.md](docs/localization-parity.md).

| Language | Code | Status | Translated |
|---|---|---|---|
| English | en | full | 100% |
| Simplified Chinese | zh | partial (no canvas IME) | 83% |
| Spanish | es | partial | 83% |
| Hindi | hi | none | 0% |
| Arabic | ar | partial (RTL UI and composition) | 85% |
| French | fr | partial | 83% |
| Portuguese (Brazil) | pt-br | partial | 61% |
| Indonesian | id | none | 0% |
| Japanese | ja | partial (no canvas IME) | 95% |
| German | de | partial | 83% |
| Korean | ko | none | 0% |
| Vietnamese | vi | none | 0% |

Also shipped: Czech (99%), Italian (85%), Ukrainian (83%), Polish (99%).

## Upcoming

Ranked; detail and estimates in [docs/roadmap.md](docs/roadmap.md), the full list in
[docs/gaps.md](docs/gaps.md).

1. **Alpha gate:** Primary Text Frame page fixes: autoflow adds pages users can't delete (#165) (5–7 h)
2. Basic editing on every platform: copy/paste, context menus, caret formatting (30–50 h)
3. CI running `cargo xtask ci` on pull requests (4–8 h)
4. INDD real-file corpus: measure what `.indd` files keep through indd-utils (10–30 h)
5. IDML real-file corpus and fixes (60–100 h)
6. Composition oracle against InDesign; optical kerning tuning; overset rule (70–125 h)
7. Start on any GPU; find every installed font (25–50 h)

## Documents

[target-app-parity.md](docs/target-app-parity.md) (assessment) · [gaps.md](docs/gaps.md) (work
list) · [roadmap.md](docs/roadmap.md) (milestones, plan) · [parity-checklist.md](docs/parity-checklist.md)
(presence checklist) · [typography-parity.md](docs/typography-parity.md) ·
[file-format-parity.md](docs/file-format-parity.md) · [ui-parity.md](docs/ui-parity.md) ·
[hardware-parity.md](docs/hardware-parity.md) · [localization-parity.md](docs/localization-parity.md) ·
[architecture.md](docs/architecture.md) · agents: [agents.md](docs/agents.md), [mcp.md](docs/mcp.md),
[control-protocol.md](docs/control-protocol.md) · CJK: [cjk.md](docs/cjk.md),
[cjk-typography.md](docs/cjk-typography.md) · Arabic: [arabic-typography.md](docs/arabic-typography.md)

## Progress log

Newest first.

- **2026-10-13** · Switching between document and parent pages refreshes the canvas immediately, preventing black parent pages and stale document objects when the zoom stays the same. Ordinary edits retain partial repainting. No readiness percentage change.
- **2026-10-13** · Commands refuse parameter keys their params description doesn't name (#776): the control channel, MCP (`-32602`, every `batch` step checked before the first runs) and `designcraft-cli --cmd` answer with an “unknown parameter” error naming the nearest or the accepted keys instead of running with defaults; journals still replay. Descriptions of the table, anchored-object, data-merge and placing commands now name every key they read. No readiness percentage change.
- **2026-10-13** · File ▸ Open Recent lists the documents last opened or saved (newest first, no duplicates; Save a Copy and exports don't count), with Clear Recent Files; the start screen shows the last eight, below the community card, as cards with a picture of the first page: saving puts one in the file (`preview.png`), so the start screen reads it without opening the document. A file that is gone is dimmed and says so when chosen, and stays listed. Preferences ▸ File Handling sets how many are kept (default 20, 0–100). `file.openRecent {index}`, `file.recentFiles`, `file.clearRecent` (#706). No readiness percentage change.
- **2026-10-13** · Dropping an `.ase` file on the window loads its swatches, as Load Swatches does (the drop failed with "Can't Place the File"). No readiness percentage change.
- **2026-10-13** · Text takes gradient swatches: a gradient fill or stroke on characters spans the text frame's box (on a path: the path's), on screen and in PDF. No readiness percentage change.
- **2026-10-13** · Gradient swatches work on strokes: drawn across the path's box on screen and in PDF (they drew nothing), set with `object.gradient {target: stroke}` and in the Gradient panel when the Stroke box is in front; the stroke's gradient vector moves with the shape and round-trips through IDML. No readiness percentage change.
- **2026-10-13** · The Gradient panel follows InDesign's layout (thumbnail, Type, Location, Angle, Reverse, ramp with midpoint diamonds) and takes swatches dragged from the Swatches panel: dropped on a stop they colour it, elsewhere they add a stop. Stops keep their CMYK through panel edits; `object.gradient` stops accept a `swatch`. No readiness percentage change.
- **2026-10-13** · New Swatch works as InDesign's: a gradient made in the Gradient panel becomes "New Gradient Swatch" (still applied), an unnamed colour a listed swatch, a named swatch a "copy" (`swatch.new`); `swatch.create {gradient}` makes a gradient swatch from colour swatches. No readiness percentage change.
- **2026-10-13** · Radial gradients stretch with the object's box, as InDesign draws them: by default an ellipse reaching the frame's sides (it was a circle reaching only the longer side), and a Gradient Swatch tool drag keeps the box's proportions; on screen and in PDF. No readiness percentage change.
- **2026-10-13** · Inner Glow takes a blend mode (Screen by default) and a Technique (Softer or Precise); Basic Feather takes Choke and Corners (Sharp, Rounded, Diffused), in the Effects dialog and `object.effects`. Inner glow and bevel blend modes now mix with the object below them instead of an empty layer. Noise isn't drawn. No readiness percentage change.
- **2026-10-13** · Text Frame Options is laid out as InDesign's: General (Columns: Fixed Number or Fixed Width with Number, Width and Gutter; Balance Columns; Inset Spacing with the link button; Vertical Justification; Ignore Text Wrap), Baseline Options (First Baseline Offset and Min) and Column Rules. `object.textFrameOptions` takes `columnWidth` and `firstBaselineMin`. No readiness percentage change.
- **2026-10-13** · The Window menu follows InDesign's order (Arrange, Workspace, then the panels alphabetically with Color, Editorial, Object & Layout, Styles… submenus), and Window › Workspace lists the workspaces as InDesign does, now with Essentials Classic: the Control panel and an icon column of named panel groups. No readiness percentage change.
- **2026-10-13** · `.indd` and `.indt` documents open through File › Open, drop, the web inbox, `file.open`, `file.openBytes`, `file.openIdml` and the CLI: [storytold/indd-utils](https://github.com/storytold/indd-utils) converts them to IDML in memory and the IDML importer reads the result, on desktop and web. Converter warnings come first in the open result; an unreadable file is an open error (#21, #114). No readiness percentage change until a real-file corpus measures it.
- **2026-10-13** · macOS: a command chosen with the mouse in the menu bar runs at once. It used to wait for the next mouse move or key press over the window, so File › Open, Place and Save As seemed to take 10–20 seconds to open their dialogs. No readiness percentage change.
- **2026-10-13** · An explicit recovery folder (`file.recovery.save` with `dir`) never overwrites or deletes another running app's checkpoints. Document uids start at 1 in every process, so two apps saving recovery to one chosen folder replaced or removed each other's `<uid>.designcraft`. A run replaces or removes only the entries it wrote or recovered there, refuses an occupied uid, and refuses a folder whose run lock another app holds. No readiness percentage change.
- **2026-10-13** · PDF export leaves out form fields, buttons and video or sound that don't print: inside a hidden group, on a hidden or non-printing layer, or marked non-printing. Page content and interactive elements follow one inclusion rule. No readiness percentage change.
- **2026-10-13** · IDML export keeps parent item overrides: each page lists its overridden parent items and the local copy keeps its link to the parent item, so a reopened IDML file shows only the local copy (#725). No readiness percentage change.
- **2026-10-13** · Reflowable EPUB and HTML export include tables (header and footer rows, merged cells), footnotes (a linked note after the paragraph that refers to it) and endnotes (a linked section at the end of the book). No readiness percentage change.
- **2026-10-13** · Word import: table cells read text by the same rules as body text, so deleted text, moved-away text and field codes stay out (#721). Word's Normal comes in as its own paragraph style "Normal" over the document defaults, so text formatted only by Normal keeps its look; it conflicts only with a document's own Normal, and the style map can send it to [Basic Paragraph] (#722). No readiness percentage change.
- **2026-10-13** · Text recomposes when the document baseline grid changes, when a layer holding a text-wrap object is shown or hidden, or when a section's start number, numbering style, prefix or marker changes; the composition cache keyed only on the section count and item identities and kept the old layout. No readiness percentage change.
- **2026-10-13** · Path edits (`path.setAnchor`, `path.moveAnchors`, `path.appendAnchor`, `path.reverse`, `path.close`, anchor add/delete/convert, Erase, Smooth and Scissors) refuse a path that is locked, sits in a locked group or on a locked layer (#735). Properties links to the Stroke, Character, Paragraph, Effects, Text Wrap and Info panels open them in a customized panel layout too (#736). No readiness percentage change.
- **2026-10-13** · The Story Editor stays with the document its story belongs to: it hides while another document is active and closes with its document, so its edits never reach another document's story with the same id (#729). Undo restores the selection from before the command and Redo the one after it (#716). No readiness percentage change.
- **2026-10-13** · Saving never reports success without a file: a document that can't be written is an error from Save, Save As, Save a Copy, Package, snippet export and the web serializer, leaving the existing file, the unsaved state and the recovery checkpoint as they were. Placed files over 4 GiB are written with zip64 sizes. A web Save whose browser download can't start reports the error and keeps the document unsaved (#741, #708). No readiness percentage change.
- **2026-10-13** · Colour management runs on craft-libs' `craft-cms`. The default CMYK working space is its Coated FOGRA39 profile (Generic CMYK and Device CMYK stay selectable); CMYK swatches show relative colorimetric with the settings' black point compensation, and PDF/X-4 output intents embed the working CMYK profile (FOGRA39 by default). Profile names the built-in stands in for resolve to it; other unknown names are an error, and saved settings naming one start with the default. No readiness percentage change.
- **2026-10-13** · Command scripts look up only a well-formed `$N.path` / `$last.path` reference; other strings starting with `$`, such as a GREP replacement of `$1` and a nonbreaking space, stay as written, and `$$` writes a literal `$` (#749). No readiness percentage change.
- **2026-10-13** · Dialogs and the About window move when you drag their title row, so the page behind them can be seen. A dragged dialog may go partly off-screen, but its title row stays reachable; each dialog opens centred, and again after the window is resized. No readiness percentage change.
- **2026-10-12** · Placed Excel tables and Excel data-merge records keep numeric precision, including small values, and preserve numeric-looking cached formula strings as text. No readiness percentage change.
- **2026-10-13** · Links rows show their source paths; selecting a link shows its full path wrapped inside the panel, so same-named files from different folders can be distinguished. No readiness percentage change.
- **2026-10-13** · HTML and reflowable EPUB order grouped stories and graphics by their transformed spread position, including nested groups and rotated frames. Explicit Article order is unchanged. No readiness percentage change.
- **2026-10-13** · A selected placed graphic shows its own brown bounding box with handles: dragging one resizes the graphic inside its frame (Shift keeps proportions, Alt from the centre) and leaves the frame as it is; `transform.set` takes the graphic's new spread `bounds`. Builds on #415 and #550. No readiness percentage change.
- **2026-10-13** · Create Outlines makes one compound path per line (and per text colour within a line), grouped, as InDesign does: Ungroup then leaves each line on its own to be spaced and made into one compound path again. A single line in one colour stays a single compound path. No readiness percentage change.
- **2026-10-13** · Ungroup only takes groups apart: a frame with pasted-in content stays whole (and selected), is named by its shape in the Layers and Properties panels, and offers Convert Shape rather than Ungroup. Non-groups stay selected. No readiness percentage change.
- **2026-10-13** · Copy with a placed graphic selected (Direct Selection) copies just the graphic, uncropped, where it sits on the page and without the frame's stroke, so Paste Into puts the whole image in place; Cut and Delete then remove only the graphic and leave its frame empty. Nested selections copy in their spread coordinates. No readiness percentage change.
- **2026-10-13** · Object effects (Basic Feather, Inner Glow, shadows, glows, bevel, satin, directional and gradient feathers) apply to frames with pasted-in content, on screen and in image exports: the frame, its clipped content and its stroke are one object to the effect. PDF export still has no soft effects for any object. No readiness percentage change.
- **2026-10-13** · Backspace between two consecutive tables, pasting a table next to a table anchor, and pasting several paragraphs right before one keep every table on its own anchor paragraph. Previously a table was dropped or left without its anchor, and the saved file failed to reopen. No readiness percentage change.
- **2026-10-12** · The engine's atomic writer, used by saves and exports (documents, snippets, books, libraries, swatches, XML, interchange, MCP, desktop and CLI writes), delegates to craft-libs/craft-atomic. Failed writes retain the original; new files follow the umask and replaced files keep their permissions. Recovery checkpoints and package output keep their own no-follow writers. No readiness percentage change.
- **2026-10-13** · New Document has Preview (off by default, kept for the session): a document with the window's settings shows in the workspace and follows them once typing pauses. Create keeps it with a clean undo history and refuses invalid fields; Close removes it and shows the previous document again. The preview takes no Untitled number and writes no recovery data; one edited by a menu or script stays open as a document. No readiness percentage change.
- **2026-10-13** · Object styles carry effects (drop shadow, feather, inner and outer shadows and glows, bevel, satin, gradient and directional feather). New styles from a selection keep its effects, `style.object.edit` changes a style's effects and every item that follows them, `style.object.clearOverrides` returns items to their style, and new frames take their default style's effects. IDML carries only the drop shadow and feather, for ObjectStyles (with their effects category and BasedOn chain) as for items (#653). No readiness percentage change.
- **2026-10-13** · A parent can be based on another parent (`layout.parents.options`). Pages draw the base parent's items, then the parent's, then their own, on screen and in PDF and image export; chains work, cycles are refused, overrides reach base items, `document.inspect` reports `basedOn`, and IDML round-trips it. No readiness percentage change.
- **2026-10-12** · The Effects dialog, as in InDesign: Object › Effects (Transparency…, Drop Shadow… and the rest, and their shortcuts) and the Effects panel's fx button open it on the chosen effect, turned on. Transparency and every effect in one window, in the document's units, with X/Y offsets, Preview on the page, Cancel putting things back, and OK as one undo step (`object.effects`). Shadows default to InDesign's 120° light angle. No readiness percentage change.
- **2026-10-12** · Editing type on a path: the Type tool clicks, drags and double-clicks along the curve (the character under the pointer, not one on the straightened line), a click on the letters themselves hits the frame, and the caret and highlight bend with the path. No readiness percentage change.
- **2026-10-12** · Type on a path has start and end brackets, as in InDesign: selected path type shows them across the path, and dragging either with the Selection or Direct Selection tool moves where the text starts or ends (text past the end bracket is overset). `type.pathOptions {end}`; IDML reads and writes EndBracket. No readiness percentage change.
- **2026-10-12** · The Pen tool continues an open path from either end point, as in InDesign: the cursor shows it over an end point, a click there carries on drawing that path, a drag first reshapes the end point's direction lines, and the other end closes it. Object › Paths › Reverse Path (`path.reverse`); `path.setAnchor`; `path.appendAnchor` and `path.close` take a subpath. No readiness percentage change.
- **2026-10-12** · The Eyedropper puts a colour sampled from an image on the stroke when the toolbox's Stroke box is in front, as in InDesign (a line takes it on its stroke). No readiness percentage change.
- **2026-10-12** · Linked text files (#597): placed Word, RTF and text files can stay linked; the Links panel shows them OK / Modified / Missing, and Update imports the file again into the same frames with the saved style map (the Links panel asks before replacing edits made in the layout). No readiness percentage change.
- **2026-10-12** · Text in oval, polygon and other non-rectangular text frames follows the frame's outline: each line is set in the part of its band inside the shape, one inset away from the outline, in every column and in vertical frames; text wrap still applies on top (#646). No readiness percentage change.
- **2026-10-12** · Colour entry: the Color Picker's hex, R/G/B and C/M/Y/K fields and the Color panel's number fields keep typed text until it is complete; panel and picker values are applied on a fixed scale (`{model, …}` colours in 0..1), so M 1 is 1 %. Picking keeps a CMYK colour CMYK; dialogs leave the page behind them undimmed; colours that round to the same swatch name share one swatch (#323). No readiness percentage change.
- **2026-10-12** · Placed photos show in their own colours: embedded ICC profiles (Adobe RGB, ProPhoto, P3, grey) are converted to sRGB on screen, and CMYK JPEGs (including YCCK), CMYK TIFFs and CMYK PSDs go through their embedded CMYK profile or the working CMYK space with the colour settings' intent and black-point compensation, instead of the profile-free formula. Unreadable profiles are ignored. No readiness percentage change.
- **2026-10-12** · Placed Word documents keep their bullets and numbering (#626): `numbering.xml` levels become bullet or numbered paragraphs with Word's number formats, label text, start numbers, restarts and list indents, also when a paragraph style carries the list. No readiness percentage change.
- **2026-10-12** · Text strokes can sit behind the fill: the stroke lies wholly outside the letters at its full weight on screen, in PNG and in PDF, set from the Properties panel or Character Style Options. IDML `StrokeAlignment` on text imports and exports as this setting, so imported text with outside-aligned strokes keeps its letter shapes (#619). No readiness percentage change.
- **2026-10-12** · Type tool: Option/Alt+Left/Right kerns the pair at the caret (manual kerning) or tracks a selection by the Kerning/Tracking increment (Preferences › Units & Increments, 20/1000 em), five times with Cmd/Ctrl; moving by words is Cmd+Left/Right on macOS and Ctrl+Left/Right elsewhere, and Option/Alt+Shift+Left/Right still selects by words (#621). No readiness percentage change.
- **2026-10-12** · Autoflow on Place threads one frame per page column (right to left in a right-to-left story), so text no longer runs across the gutter; pages Smart Text Reflow adds get a primary frame with the page's columns (#625). No readiness percentage change.
- **2026-10-12** · Vector art copied in Illustrator, VectorCraft, Figma or Inkscape pastes as editable paths, as InDesign pastes Illustrator art: the SVG those apps put on the clipboard becomes paths with their fills, strokes (weight, caps, joins, dashes) and opacity, groups stay groups and text comes in as outlines, centred in the view (`edit.pasteSvg`, `ui.paste`). Edit › Paste reads the system clipboard when nothing was copied in DesignCraft. No readiness percentage change.
- **2026-10-12** · Pages panel: double-clicking a parent shows that parent and selects only its row; parent rows show rendered thumbnails. Commands name parents by id, full name, or a prefix only one parent has; an unknown or ambiguous name is an error instead of clearing the page's parent (#624). No readiness percentage change.
- **2026-10-12** · With the Type tool, dragging a table's column or row border resizes it (the double-arrow cursor shows over a border): later columns and rows move along, Shift keeps the table size, and Shift on the right or bottom edge scales every column or row. A dragged row becomes At Least; one undo step per drag, Esc cancels; works in rotated frames and turned spreads. Scripts use `table.resizeBorder`. No readiness percentage change.
- **2026-10-12** · The Tools panel's Fill/Stroke keys work: D (default fill and stroke), X (fill or stroke in front), Shift+X (swap per object, tints included) and / (Apply None), as `color.*` commands; swap and default are one undo step (`object.swapFillStroke`, `object.defaultFillStroke`); the Color panel follows the proxy's fill/stroke choice. No readiness percentage change.
- **2026-10-12** · With a placed graphic selected (Direct Selection), the Transform values act on the graphic inside its frame, as in InDesign: a tilted frame's image can be straightened to 0°. No readiness percentage change.
- **2026-10-11** · Font menu laid out like InDesign's: filters (classification, favourites,
  recently added), families opening to their styles, the selected text as the sample in three
  sizes, Recently Used fonts, Show Similar Fonts and each font's format.
- **2026-10-12** · The Effects panel shows the level its settings apply to ("Group: Color Dodge 100%") and, as in InDesign, Isolate Blending and Knockout Group for a selected group. No readiness percentage change.
- **2026-10-12** · Edit › Step and Repeat has its InDesign dialog: Repeat (Count, or Rows and Columns with Create as a grid), Offset (Vertical, Horizontal, in ruler units) and Preview, which shows the copies while the dialog is open; Cancel takes them away and OK makes them as one undo step. No readiness percentage change.
- **2026-10-12** · Load Styles: the Paragraph Styles and Character Styles panels load styles from another DesignCraft document or an IDML file, with the styles, swatches and lists they name; a dialog picks the styles and what happens to one the document defines differently (`style.load`). No readiness percentage change.
- **2026-10-12** · On screen, blend modes blend with the objects below and not with the paper, as in InDesign and in PDF export: a Color Dodge or Screen object over blank paper shows as it is. No readiness percentage change.
- **2026-10-12** · Text threads by hand: click a selected text frame's in or out port to load
  the text cursor, then click a frame to thread it, click empty space for a column-wide frame or
  drag one (it snaps like frame drawing); Shift-click autoflows, Alt-click stays loaded on the
  frame just added; click the linked frame or double-click a port to break the thread.
  `text.thread`, `text.unthread`, `text.removeFromThread` with undo; locked frames and layers
  refuse threads. The alpha gate now waits only on the Primary Text Frame page fixes (#165). No
  readiness percentage change.
- **2026-10-12** · Properties › Transform shows rotation, shear and X/Y scale, and Rotate 90° and Flip Horizontal/Vertical buttons, as InDesign's does. No readiness percentage change.
- **2026-10-12** · Underlines and strikethroughs are drawn as one bar per stretch of text on screen, as in PDF export: a bar per letter left faint seams between the letters. No readiness percentage change.
- **2026-10-12** · The section marker character shows its section's marker text on document pages (nothing when the section has none) instead of the word "Section", which stays on parent pages; editing the marker recomposes (#556). No readiness percentage change.
- **2026-10-12** · IDML import keeps a font family name exactly as written, trailing spaces included, and font lookup falls back to the name without surrounding spaces; `FontStyle="Nothing"` leaves the style to the paragraph (#558). No readiness percentage change.
- **2026-10-12** · Clipping paths saved with images in IDML (Detect Edges, Alpha Channel, Photoshop Path, User-Modified Path, inverted or not) clip the image on screen and in PDF and survive IDML export; `object.clippingPathOptions` reports, inverts or removes one (#450). No readiness percentage change.
- **2026-10-12** · Anchored objects with text wrap push text aside: other stories' text under them and the lines of their own story after the anchor's line (#557). No readiness percentage change.
- **2026-10-12** · Hyphenation Zone applies to the Single-line Composer only, as in InDesign. With the default Paragraph Composer, ragged text hyphenates where InDesign does (aper-natem), giving the same line breaks. No readiness percentage change.
- **2026-10-12** · Text frame auto-size: Height Only, Width Only, Height and Width (also keeping proportions) frames grow and shrink to fit their text after edits, around their reference point, with minimum sizes and No Line Breaks, in the same undo step (#180). No readiness percentage change.
- **2026-10-12** · Text Frame Options › Balance Columns (`object.textFrameOptions {balanceColumns}`, IDML `VerticalBalanceColumns`) evens out the columns of the frame a story ends in, from its last column break on; frames the text runs on from fill normally (#609). No readiness percentage change.
- **2026-10-13** · Layout ▸ Table of Contents opens a real dialog: title and title style, Include Paragraph Styles and Other Styles lists with Add and Remove, and per style its entry style, page number position (after, before, none), separator and level. Replace Existing regenerates the current table of contents in place; otherwise OK makes a new frame on the page in view. IDML export writes the definition as a TOC style that import reads back, so Update works after a round trip. No readiness percentage change.
- **2026-10-12** · macOS Dock Quit uses the same unsaved-document prompts as window and menu Quit. Repeated requests keep the current answers; Cancel retains every tab and unsaved recovery checkpoint, and completed Save/Don't Save answers exit through the ordinary event loop. No readiness percentage change.
- **2026-10-12** · Orderly native exit writes the latest recovery state after finalizing current quit approvals; incomplete or stale plans keep their documents, and Undo to the saved state clears stale recovery. Unavailable or failed recovery is reported. Cancelable macOS Dock Quit remains open (#544). No readiness percentage change.
- **2026-10-12** · Windows: dragging the window across monitors with different display scales keeps its size instead of growing until it crashes (winit pinned to its 0.30.x backport of the fix). The GPU surface may be as large as the adapter's largest texture (16384 px on DirectX 12, Vulkan and Metal hardware) instead of 8192, and a wgpu error is logged instead of closing the app. No readiness percentage change.
- **2026-10-12** · Recovered documents keep a checkpoint until saved, closed or reverted, including a second crash before autosave. Restarted document IDs and explicit recovery folders preserve other snapshots; failed republishing retains the original. No readiness percentage change.
- **2026-10-12** · Book numbering and style updates prepare every chapter before saving. Only closed native chapters are changed; imported IDML remains a read-only style/PDF source. If a later disk write fails, the error names the completed chapters; the batch is not a whole-book transaction. No readiness percentage change.
- **2026-10-12** · Desktop write callbacks and CLI raster exports use the atomic writer too; open readers retain the original bytes, while private Unix modes and symlink targets survive replacement. Permanent tests cover both actual callers. No readiness percentage change.
- **2026-10-12** · Save As, library edits and book collection edits persist prospective values before changing live state; failed writes preserve the document and existing collections. Save a Copy rebases data-source paths for its destination without changing the open document. No readiness percentage change.
- **2026-10-12** · Window close and application/menu Quit ask about unsaved documents; later Cancel keeps every tab open, including earlier Don't Save choices. Approvals are rechecked after edits. Cancelable macOS Dock Quit remains open (#544). No readiness percentage change.
- **2026-10-12** · Each running app keeps recovery data in its own locked folder; current folder and lock ownership are checked before writes and cleanup. Failed isolation disables recovery with a warning instead of sharing document IDs. No readiness percentage change.
- **2026-10-12** · Native engine saves and exports replace files atomically; failed writes retain the original, private staging preserves Unix permissions, and valid long filenames work. Desktop Services and CLI raster exports remain separate follow-ups. No readiness percentage change.
- **2026-10-11** · Document chapter numbering (#341): style, automatic, start at, same as previous
  document in the book; Update Numbering resolves it in book order.
- **2026-10-11** · Advanced workspace's object Control panel compacted into aligned two-row
  groups (#255): spinners, fill/stroke chips, proportion links, corners, Auto-Fit, alignment.
- **2026-10-12** · A gradient feather on a group fades the group as one, on screen and in PDF. Before, the screen ignored it, and PDF masked the group with a box of zero size because a group has no path of its own. No readiness percentage change.
- **2026-10-12** · The Eyedropper samples a placed image: clicking the photo with text (or objects) selected fills them with the image's colour under the pointer, as an unnamed colour (`color.sample`), as in InDesign. Images on locked layers can be sampled. No readiness percentage change.
- **2026-10-12** · In a CMYK transparency blend space (print documents), blend modes blend in CMYK on screen as in InDesign. Color Dodge with 100% K text turns coloured areas under it white and leaves the paper alone, where RGB blending changed almost nothing. PDF export already did this. No readiness percentage change.
- **2026-10-12** · Double-clicking a frame handle fits the frame to its content as in InDesign (a corner fits both sides, an edge only its own). Object › Fitting › Fit Frame to Content now fits text frames to their text too, growing them for overset text. No readiness percentage change.
- **2026-10-12** · Words in capitals hyphenate like capitalized words (FERRUM COM-MODI), as in InDesign; Hyphenate Capitalized Words off keeps them whole. No readiness percentage change.
- **2026-10-12** · Load Swatches refuses an `.ase` file with more than 100,000 colors, and `swatch.load` merges a file of 100,000 colors in under 0.1 s (it took 9.6 s of CPU in a test build). No readiness percentage change.
- **2026-10-11 (printing):** `file.printers` and `file.print` use craft-libs/craft-print for CUPS and Windows. DesignCraft retains PDF export, page selection, spreads, marks and bleed; dry runs create no temporary PDF or print job. Windows selects matching or default driver paper and shrinks oversized sheets without enlarging smaller ones; queue duplex defaults are preserved. Headless tests exercise exported bytes, job settings, dry runs and refusals; shared native Windows CI passes, including paused-queue submission and cleanup. PostScript, production print controls and physical output validation remain open. No readiness percentage change.
- **2026-10-11** · The View menu follows InDesign's: proofing first, then zoom, Rotate Spread, Screen Mode, Display Performance, rulers, Extras, Grids & Guides and Structure; the view items that sat at the end of the Type menu moved there. No readiness percentage change.
- **2026-10-11** · Properties › Appearance's Fill and Stroke menus gain InDesign's Tint field and, for a text frame, Apply To: Frame / Text. No readiness percentage change.
- **2026-10-11** · `story.replaceRange`, `story.setText`, notes, footnotes, endnotes, type on a path and table cells read `\r` and `\r\n` as paragraph returns, as `frame.create` and `text.insert` already did. No readiness percentage change.
- **2026-10-11** · Corner Options keep curved sides curved and round every corner point, including where an arc meets a line (a Pathfinder quarter circle with rounded corners stayed a triangle). No readiness percentage change.
- **2026-10-11:** Type › Justification… (Cmd+Alt+Shift+J, also in the Paragraph panel menu) sets word and letter spacing, glyph scaling and the composer on the selected paragraphs, sharing the Paragraph Style Options section (#531). No readiness percentage change.
- **2026-10-11** · Properties gains InDesign's Pathfinder section (Add, Subtract, Intersect, Exclude Overlap, Minus Back) with several objects selected; section titles and icon buttons have accessible names. No readiness percentage change.
- **2026-10-11** · `transform.move {copy}`, Edit › Duplicate and Step and Repeat return the copies' ids, so a script can go on with them. No readiness percentage change.
- **2026-10-11** · `object.fill` and `object.stroke` refuse a swatch name the document doesn't have (it painted nothing) and suggest the name meant (`Paper` → `[Paper]`). No readiness percentage change.
- **2026-10-11** · Japanese captions now cover print ranges, imported style handling and table
  row/column movement, including row-height distribution. Existing translation and RTL helpers
  handle the display; command IDs, stored option values and user-defined style names are unchanged.
- **2026-10-11** · Moving a sheared and rotated text frame no longer leaves trails of text behind: canvas repaints cover the frame's whole text box, not just its outline. No readiness percentage change.
- **2026-10-11** · Live corners: the yellow widget on a selected rectangle frame shows corner
  diamonds; dragging one rounds the corners, Alt-click steps the corner shape (#186).
- **2026-10-11** · Optical kerning (#181): pairs are spaced from their outlines (ink gap per band,
  pulled towards the font's stem gap, bounded) instead of silently using the font's kern pairs.
- **2026-10-11** · New Document laid out like InDesign's (#325): Recent, Saved, Print, Web and
  Mobile preset tabs beside the preset's details; saved document presets live in the engine
  (`file.savePreset`, `file.deletePreset`, `file.new {preset}`); `file.new` takes intent, units,
  start page and per-edge bleed and slug.
- **2026-10-11** · macOS: fonts that font services and managers activate (Adobe Fonts,
  RightFont) are listed, read from the files CoreText has available (#261, #327).
- **2026-10-11:** Document pages take their parent page's margins and columns (left or right parent page by side) and follow its changes unless they have their own; parent ruler guides draw and snap on the pages based on it. IDML import marks a page margin preference that differs from its parent as the page's own. No readiness percentage change.
- **2026-10-11** · Document Setup laid out like InDesign's: Page Size, Margins (new: every
  page's, `layout.documentSetup {margins}`), Bleed and Slug with links, Adjust Layout.
- **2026-10-11** · IDML reads and writes a spread's view rotation (Rotate Spread) as the
  rotation part of the spread transform (#484).
- **2026-10-11** · The document grid is drawn (View ▸ Grids & Guides ▸ Show/Hide Document Grid,
  ⌘'), on the lines Snap to Document Grid uses, behind or in front of page items (#530).
- **2026-10-12** · Double-clicking the Fill or Stroke box in the Tools panel opens the Color Picker with the selection's colour, as in InDesign (it opened the Swatches panel). No readiness percentage change.
- **2026-10-11 (DC-1):** Hold Cmd+Space for the Zoom cursor and click/scrubby zoom, with Alt-click to zoom out. Releasing restores the selected tool; text editing and existing tool drags retain their input. Space remains Hand, and navigation changes neither the document nor undo history. No readiness percentage change.
- **2026-10-10 (UI.4–UI.5):** Panel tabs can move into other groups, split the dock, float, resize and return to their saved locations through shared docking transactions. Layouts persist in custom workspaces; UI and control commands share the same validated action path. Shared pane geometry and widget-state theme application retain all five palettes and local fonts. Split panes send keyboard input only to the focused canvas and scope contextual toolbar IDs per pane. No readiness percentage change.
- **2026-10-10** · Version 0.6.0; releases add a Linux riscv64 tar.gz (cross-compiled, CLI
  smoke-tested under QEMU).
- **2026-10-10** · Readiness per audience: full ~47% (method aligned with the standard, was
  42%), mainstream practitioner ~42%, essentials user ~54%, each with hours to ~95%.
- **2026-10-10** · Stage re-normalized to **pre-alpha** under craftrules' core-workflow gate: text
  can't be threaded by hand (no out-port click, no thread command).
- **2026-10-10** · Full re-measure against InDesign 2026 21.6 (menu dump, bundle formats and
  localizations, 124 issues): breadth ~80%, ready ~42% (down from the ~83% "including depth" of
  2026-10-04, on user evidence and the INDD gap); progress docs restructured to the craftrules
  standard (`docs/parity.md` → `docs/parity-checklist.md`; new target-app-parity, gaps, roadmap,
  architecture, typography, file-format, UI, hardware and localization docs). Landed today:
  split columns (#295), PDF export options dialog (#196), column rules (#274), Paragraph Rules
  dialog (#263), Character Style Options (#144), PDF/X-4 RGB handling (#223), Spanish
  hyphenation (#222), Japanese (#272), Italian (#218) and Ukrainian (#236) interfaces, relocated packages relink (#244), graphics backend chosen
  before the window (#297), graphics adapter chosen by the display it drives with a restart on
  the next adapter or backend when one fails at start-up and the reason logged (#273, #334), IDML table styles (#288), warichu (#280), list numbering from IDML
  (#298), security hardening (#153, #155), many text-composition fixes (#282–#293).
- **2026-10-09** · Version 0.5.0; many user reports triaged into issues (#160–#265).
- **2026-10-08** · Version 0.4.0.
- **2026-10-07** · Community PRs: data merge, Brazilian Portuguese, PDF/X-4 flags, Office fonts
  on macOS; CJK 1.x (Korean line breaking, fallback fonts in Package); PrintCraft renamed PdfCraft.
- **2026-10-06** · Snapping to parent items, Windows packaging, ARM64 workflow, control-channel
  hardening.
- **2026-10-05** · FreeBSD build; caret-boundary fixes.
- **2026-10-04** · Never-crash pass across importers and the engine; rotate spread view; kashida
  in styles; previous estimate: breadth 99%, "~83% including depth", ~170 h remaining.
- **2026-10-03** · Release 0.1.1; separations, XML tagging, track changes, PDF/X-4 output
  intent, split window, buttons, menu customization.
- **2026-10-02** · Hyperlinks/Bookmarks panels, conditional text, tagged PDF, lists across
  stories, style groups, Glyphs panel, High Contrast theme.
- **2026-10-01** · First commit; skeleton, IDML, PDF, tables, footnotes, cross-references, index,
  spelling and parallel composition on the first day.

## Revision history

| Date | Change | Summary |
|---|---|---|
| 2026-10-12 | minor | Manual text threading landed; alpha distance 15–25 h → 5–7 h (Primary Text Frame page fixes, #165) |
| 2026-10-11 | minor | Localized Japanese print, import and table controls; readiness estimates unchanged |
| 2026-10-10 | minor | Full number 42% → 47% (method aligned with the standard: plain weighted sum, no new evidence); readiness table with hours per audience |
| 2026-10-10 | minor | Headline adds mainstream practitioner (~42%) and essentials user (~54%); method in docs/target-app-parity.md |
| 2026-10-10 | minor | Stage alpha → pre-alpha under the new core-workflow gate: manual text threading is missing; alpha gate table added to docs/roadmap.md |
| 2026-10-10 | major | Restructured to the progress-docs standard; full re-measure; the "Working today" inventory moved to docs/target-app-parity.md, milestones to docs/roadmap.md, the CLI/MCP section left to docs/agents.md (which already covered it) |
| 2026-10-04 | major | Parity estimate: breadth 99%, depth ~83%, ~170 h |
| 2026-10-01 | major | First roadmap: status, milestones |
