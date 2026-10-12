# DesignCraft roadmap

**Stage: pre-alpha** · next: alpha, ~5–7 h away; beta ~28 points and ~420–750 h beyond that (the core-workflow gate, not the percentage: ready for real work is already ~47%)

> **Last reviewed:** 2026-10-11 · **Last updated:** 2026-10-12 · **Change:** minor (text threads by hand; the alpha gate waits on Primary Text Frame page fixes; Japanese print, import and table controls localized; Live corners (#186); Optical kerning from outlines (#181); window and menu quit confirmation; failed writes preserve live save and collection state) · **Target:** Adobe InDesign 2026

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
| Mainstream practitioner | ~42% | 500–850 (~65% parallel) | INDD import and IDML corpus, composition fidelity, weekly-area depth, PDF/X-1a and printing |
| Essentials user | ~54% | 135–250 (~50% parallel) | start-up robustness, context menus and copy/paste, threading, fonts, Windows printing |

**Why pre-alpha:** it fails one of InDesign's core workflows, the alpha gate in
[docs/roadmap.md](docs/roadmap.md#alpha-gate). Text threads by hand (click a port, then click or
draw a frame; thread and unthread commands), but Primary Text Frame autoflow adds pages users
can't delete (#165). Everything else in the gate works end to end on macOS (document setup,
styles, images, object editing, PDF export, save and reopen), and ready for real work (~47%) is
above the alpha bar, so the Primary Text Frame page fixes are the whole distance to alpha.
Beyond that, beta needs INDD opening, IDML proven on real files, composition compared with
InDesign's, and basic editing fixed on Windows and Linux. Details:
[docs/target-app-parity.md](docs/target-app-parity.md).

## By dimension

| Dimension | Ready | Hours | Doc |
|---|---|---|---|
| Features (depth) | 55% | 250–400 | [target-app-parity.md](docs/target-app-parity.md) |
| Typography and composition fidelity | 45% | 120–200 | [typography-parity.md](docs/typography-parity.md) |
| File formats | 40% | 150–250 + 20–60 (INDD) | [file-format-parity.md](docs/file-format-parity.md) |
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

Also shipped: Czech (99%), Italian (85%), Ukrainian (83%).

## Upcoming

Ranked; detail and estimates in [docs/roadmap.md](docs/roadmap.md), the full list in
[docs/gaps.md](docs/gaps.md).

1. **Alpha gate:** Primary Text Frame page fixes: autoflow adds pages users can't delete (#165) (5–7 h)
2. Basic editing on every platform: copy/paste, context menus, caret formatting (30–50 h)
3. CI running `cargo xtask ci` on pull requests (4–8 h)
4. Open INDD through a clean-room converter, owner decision on #114 (20–60 h)
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
- **2026-10-12** · Save As, library edits and book collection edits persist prospective values before changing live state; failed writes preserve the document and existing collections. Save a Copy rebases data-source paths for its destination without changing the open document. No readiness percentage change.
- **2026-10-12** · Window close and application/menu Quit ask about unsaved documents; later Cancel keeps every tab open, including earlier Don't Save choices. Approvals are rechecked after edits. Cancelable macOS Dock Quit remains open (#544). No readiness percentage change.
- **2026-10-12** · Each running app keeps recovery data in its own locked folder; current folder and lock ownership are checked before writes and cleanup. Failed isolation disables recovery with a warning instead of sharing document IDs. No readiness percentage change.
- **2026-10-12** · Native engine saves and exports replace files atomically; failed writes retain the original, private staging preserves Unix permissions, and valid long filenames work. Desktop Services and CLI raster exports remain separate follow-ups. No readiness percentage change.
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
