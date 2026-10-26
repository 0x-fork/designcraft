use super::clock::now_ms;
#[cfg(test)]
use super::services::Inbox;
use super::services::Services;
use super::state::{CanvasCache, ImportRequest, ImportedFile, Perf, ScreenMode, UiState, View, second_window_viewport};
use super::*;

impl DesignApp {
    /// Start the background font scan, establish the monotonic timing origin, and initialize document, view, input, screenshot, dialog and control state.
    pub fn new(session: Session, services: Services) -> Self {
        // The font menus and the first file opened need the installed fonts: catalog them now.
        designcraft_fonts::FontDb::global().scan_in_background();
        // The app owns the monotonic origin for recovery timing and frame pacing; `now_ms` itself
        // is process-global so screenshot deadlines stay comparable across hosts.
        let now = now_ms() / 1000.0;
        DesignApp {
            pending_pdf: None,
            session,
            ui: UiState::default(),
            services,
            views: HashMap::new(),
            page_history: HashMap::new(),
            canvas: CanvasCache::new(),
            canvas_rect: None,
            split: false,
            second_window: false,
            pane: 0,
            focus_pane: 0,
            other_pane: None,
            power_zoom: None,
            color_applied: false,
            perf: Perf::default(),
            synthetic: vec![],
            control_synthetic: vec![],
            native_actions: Default::default(),
            native_events_pending: Default::default(),
            control_input_frame: Default::default(),
            clipboard_guard_installed: false,
            story_editor: None,
            control_rx: None,
            pending_shots: vec![],
            queued_shots: vec![],
            shot_token: 0,
            screenshot_geometry: None,
            screenshot_epoch: 0,
            synthetic_injected: false,
            injected_events: Vec::new(),
            injected_private: false,
            injected_generation: InputGeneration::default(),
            injected_pending: Default::default(),
            injected_settled: Default::default(),
            injected_settle_hook: false,
            #[cfg(not(target_arch = "wasm32"))]
            pending_drops: Vec::new(),
            #[cfg(test)]
            logic_tick_ticks: 0,
            #[cfg(test)]
            logic_frame_ticks: 0,
            #[cfg(test)]
            input_ticks: 0,
            styled: false,
            fonts_lang: String::new(),
            fonts_system: false,
            restyle: false,
            fonts_ready: false,
            os_documents: Vec::new(),
            integrated_titlebar: false,
            custom_titlebar: false,
            window_maximized: false,
            pending_window_commands: vec![],
            native_menu: false,
            native_shortcuts: Default::default(),
            keymap: keys::new_keymap(craft_keymap::Overrides::new()),
            key_dispatcher: craft_keymap::egui::Dispatcher::new(),
            shortcut_dialog: None,
            shortcut_answers: Default::default(),
            last_time: now,
            last_recovery: now,
            egui_ctx: None,
            in_control: false,
            object_clip_text: None,
            ime_marked: None,
            ime_discard: false,
            canvas_ime: false,
            ime_tracker: egui::ime_context::ImeContextTracker::new(),
            pending_ime_delete: None,
            ime_target: None,
            ime_interrupt: false,
            ime_canvas: None,
            quit_requested: false,
            close_requested_pending: false,
            quit_without_asking: false,
            quitting: None,
            cursors: cursors::Images::default(),
            live_preview: LivePreview::default(),
            updates_rt: updates_ui::Runtime::default(),
        }
    }

    pub fn with_control(mut self, rx: Receiver<ControlRequest>) -> Self {
        self.control_rx = Some(rx);
        self
    }

    /// The view key of the active document in the current pane.
    pub(crate) fn view_key(&self) -> Option<u64> {
        let uid = self.session.active()?.uid;
        Some(if self.pane == 1 { uid ^ (1 << 63) } else { uid })
    }
    pub fn view(&self) -> Option<&View> {
        self.views.get(&self.view_key()?)
    }
    pub fn view_mut(&mut self) -> Option<&mut View> {
        let k = self.view_key()?;
        Some(self.views.entry(k).or_default())
    }
    pub(crate) fn page_history(&self) -> Option<&page_history::PageHistory> {
        self.page_history.get(&self.view_key()?)
    }
    pub(crate) fn page_history_mut(&mut self) -> Option<&mut page_history::PageHistory> {
        let k = self.view_key()?;
        Some(self.page_history.entry(k).or_default())
    }
    /// Make `pane` current: its canvas cache and rect move in.
    pub fn switch_pane(&mut self, pane: u8) {
        if pane == self.pane {
            return;
        }
        let (c, r) = self.other_pane.take().unwrap_or_default();
        let old = (std::mem::replace(&mut self.canvas, c), std::mem::replace(&mut self.canvas_rect, r));
        self.other_pane = Some(old);
        self.pane = pane;
    }
    pub fn view_info(&self) -> ViewInfo {
        ViewInfo {
            zoom: self.view().map(|v| v.zoom).unwrap_or(1.0),
            snap: self.ui.snap_view(),
            // Tool readouts (sizes, smart-guide distances) measure in the document's ruler units,
            // as its Control and Properties fields do.
            unit: self.session.active().map_or(self.ui.units, |st| st.doc.settings.horizontal_units),
            stroke_in_front: self.ui.stroke_front,
        }
    }

    pub fn status(&mut self, s: impl Into<String>) {
        self.ui.status = s.into();
    }

    /// The host's save dialog for `name`, started beside the active document once it has been
    /// saved, else in the folder of the last file saved or exported (#1653). The answer's folder
    /// is remembered for the next one.
    pub(crate) fn pick_save(&mut self, name: &str) -> Option<String> {
        use std::path::{Path, PathBuf};
        let folder_of = |path: &str| Path::new(path).parent().filter(|f| !f.as_os_str().is_empty()).map(Path::to_path_buf);
        let folder = self
            .session
            .active()
            .and_then(|d| d.path.as_deref())
            .and_then(folder_of)
            .or_else(|| self.ui.last_save_folder.as_ref().map(PathBuf::from));
        // Only the last part of a name goes after the folder: a title may hold a slash, and an
        // empty name would make the folder itself the suggested file.
        let file = Path::new(name).file_name();
        let suggested = match (folder, file) {
            (Some(f), Some(file)) => f.join(file).to_string_lossy().to_string(),
            _ => name.to_string(),
        };
        let picked = self.services.pick_save.as_mut().and_then(|f| f(&suggested))?;
        if let Some(folder) = folder_of(&picked) {
            self.ui.last_save_folder = Some(folder.to_string_lossy().to_string());
        }
        Some(picked)
    }

    /// Did an IME composition end outside the IME this frame (a click, a tool switch)? egui's
    /// `should_interrupt_composition` toggles winit's IME, which doesn't reach the macOS input
    /// context: it keeps the marked text and types it again into the next composition, so the
    /// host discards it there.
    pub fn take_ime_discard(&mut self) -> bool {
        std::mem::take(&mut self.ime_discard)
    }

    /// Did the canvas ask for the system IME in the last frame (a Type tool caret)? The web
    /// build's hidden text input has the keyboard then, so text events hold what the browser
    /// typed (see [`drop_key_name_text`]).
    pub fn canvas_ime(&self) -> bool {
        self.canvas_ime
    }

    /// The second window uses a stable viewport id; forget its IME context when it closes.
    pub(crate) fn end_second_ime_viewport(&mut self) {
        self.ime_tracker.end_viewport(egui::ViewportId::from_hash_of("designcraft_second_window"));
    }

    /// "<feature>: <error>" in the status bar: the feature name is translated, the engine's own
    /// error text stays as the engine wrote it.
    pub fn status_error(&mut self, feature: &str, error: impl std::fmt::Display) {
        self.ui.status = format!("{}: {error}", crate::i18n::tr(&self.ui.language, feature));
    }

    /// THE entry point for every action (menus, shortcuts, palette, panels, control channel).
    pub fn run(&mut self, id: &str, params: Value) -> Result<Value, String> {
        if let Some(r) = menus::run_ui(self, id, &params) {
            return r;
        }
        // Only user-initiated runs touch the system clipboard, never the control channel's.
        let system_clipboard = !self.in_control && !self.control_input_frame.load(std::sync::atomic::Ordering::Relaxed);
        let mut params = if system_clipboard { self.with_system_clipboard(id, params) } else { params };
        // Vector art copied in another app (Illustrator, VectorCraft, Figma) is SVG markup on the
        // system clipboard: with no insertion point it pastes as editable paths where text would go.
        let typing = self.session.active().is_some_and(|d| d.selection.text.is_some());
        let id = match params.as_object_mut() {
            Some(o) if id == "edit.paste" && !typing && o.get("text").and_then(Value::as_str).is_some_and(designcraft_engine::cmd::is_svg_markup) => {
                if let Some(svg) = o.remove("text") {
                    o.insert("svg".into(), svg);
                }
                "edit.pasteSvg"
            }
            _ => id,
        };
        // Copied objects paste on the spread in view.
        if matches!(id, "edit.paste" | "edit.pasteInPlace" | "edit.pasteSvg")
            && let Some(spread) = canvas::current_spread(self)
            && let Some(p) = params.as_object_mut()
        {
            p.entry("spread").or_insert(json!(spread));
        }
        let r = self.session.execute(id, &params).map_err(|e| e.to_string());
        self.after_engine();
        match &r {
            Err(e) => self.status(e.clone()),
            // Copied text goes to the system clipboard too, whichever way Copy was chosen.
            Ok(v) if system_clipboard && matches!(id, "edit.copy" | "edit.cut") => {
                let text = match v.get("text").and_then(Value::as_str) {
                    Some(t) => {
                        self.object_clip_text = None;
                        Some(t.to_string())
                    }
                    None if !self.native_menu => {
                        self.object_clip_text = Some(designcraft_engine::cmd::copied_objects_text(&self.session));
                        self.object_clip_text.clone()
                    }
                    None => {
                        self.object_clip_text = self.services.clipboard_text.as_mut().and_then(|f| f());
                        None
                    }
                };
                if let (Some(t), Some(ctx)) = (text, &self.egui_ctx) {
                    ctx.copy_text(t);
                    ctx.request_repaint();
                }
            }
            Ok(_) => {}
        }
        if r.is_ok() && matches!(id, "file.open" | "file.openIdml" | "file.openBytes") {
            if self.ui.dialog.as_ref().is_some_and(|d| d.id == "findFont") {
                self.ui.dialog = None;
            }
            match self.session.execute("font.list", &json!({})) {
                Ok(fonts) if fonts.as_array().is_some_and(|fonts| fonts.iter().any(|f| f["missing"] == true || f["styleMissing"] == true)) => {
                    self.ui.dialog = Some(dialogs::Dialog::new("findFont", json!({"_fonts": fonts, "onOpen": true})));
                }
                Err(e) => self.status(e.to_string()),
                _ => {}
            }
        }
        if r.is_ok()
            && matches!(id, "font.replace" | "font.reload" | "edit.undo" | "edit.redo")
            && let Some(dialog) = self.ui.dialog.as_mut().filter(|d| d.id == "findFont")
        {
            dialog.fields.remove("_fonts");
        }
        r
    }

    /// Show `command` on the page as a live preview named `label` (the undo step's name): each call
    /// replaces the last, and nothing reaches the undo history until [`Self::end_live_preview`]. A
    /// panel calls this, or [`Self::hold_live_preview`], every frame its control is held; a frame
    /// without either commits the preview.
    pub fn live_preview(&mut self, label: &str, command: &str, params: Value) {
        if self.run("edit.preview", json!({"label": label, "command": command, "params": params})).is_ok() {
            self.live_preview = LivePreview { open: true, held: true };
        }
    }

    /// The control showing the live preview is still held (pressed, though not moved this frame).
    pub fn hold_live_preview(&mut self) {
        self.live_preview.held = self.live_preview.open;
    }

    /// Commit the live preview, if one is open, as one undo step.
    pub fn end_live_preview(&mut self) {
        if std::mem::take(&mut self.live_preview).open {
            let _ = self.run("edit.commitPreview", json!({}));
        }
    }

    /// Start of a frame: a live preview no panel held last frame (its control was let go, or the
    /// panel closed) is committed, so it never swallows later edits' undo steps.
    pub(crate) fn settle_live_preview(&mut self) {
        if !std::mem::take(&mut self.live_preview.held) {
            self.end_live_preview();
        }
    }

    /// Paste without the clipboard's text (a menu, not the paste keys): read it here. With no
    /// insertion point, text that doesn't stand for copied objects becomes a new text frame
    /// centred in the view.
    fn with_system_clipboard(&mut self, id: &str, mut params: Value) -> Value {
        let typing = self.session.active().is_some_and(|d| d.selection.text.is_some());
        if !matches!(id, "edit.paste" | "edit.pasteWithoutFormatting") || !(typing || id == "edit.paste") {
            return params;
        }
        if params.get("text").is_none()
            && let Some(text) = self.services.clipboard_text.as_mut().and_then(|f| f())
            && let Some(o) = params.as_object_mut()
        {
            o.insert("text".into(), Value::String(text));
        }
        // Still what an object copy put there: paste those objects, not the placeholder text.
        let norm = |t: &str| t.replace("\r\n", "\n");
        if let Some(copied) = &self.object_clip_text
            && params.get("text").and_then(Value::as_str).is_some_and(|t| norm(t) == norm(copied))
            && let Some(o) = params.as_object_mut()
        {
            o.remove("text");
        }
        if !typing
            && params.get("text").and_then(Value::as_str).is_some_and(|t| !t.is_empty())
            && let Some((spread, at)) = canvas::view_centre(self)
            && let Ok(spread) = serde_json::to_value(spread)
            && let Some(o) = params.as_object_mut()
        {
            o.entry("spread").or_insert(spread);
            o.entry("at").or_insert(json!([at.x, at.y]));
        }
        params
    }

    /// Handle requests produced by tools/commands (dialogs, view changes, file pickers).
    pub fn after_engine(&mut self) {
        let reqs = std::mem::take(&mut self.session.ui_requests);
        for r in reqs {
            let was_find = self.ui.dialog.as_ref().is_some_and(|d| d.id == "findChange");
            match r {
                UiRequest::Dialog { id, params } if id == "colorTheme" => dialogs::open_color_theme(self, params),
                UiRequest::Dialog { id, params } => self.ui.dialog = Some(dialogs::Dialog::new(&id, params)),
                UiRequest::View { params } => canvas::apply_view_request(self, &params),
                UiRequest::Pick { purpose, params } => {
                    let _ = params;
                    let _ = self.pick_and_open(&purpose);
                }
            }
            // Release only when the request actually replaces Find, including a native PDF picker.
            if was_find && self.ui.dialog.as_ref().is_none_or(|d| d.id != "findChange") {
                self.session.reset_find_scope();
            }
        }
        // New documents get a fitted view.
        if let Some(st) = self.session.active() {
            let uid = st.uid;
            self.views.entry(uid).or_default();
        }
    }

    /// Ask the host for a file to open (`open`) or place (`place`). Synchronous pickers run the
    /// command right away; asynchronous ones (web) deliver the file through the inbox.
    pub fn pick_and_open(&mut self, purpose: &str) -> Result<Value, String> {
        let target = self.session.active().map(|d| d.uid);
        if let Some(pick) = self.services.pick_open.as_mut() {
            return match pick(purpose) {
                // Image Import Options: a multi-page PDF asks which page.
                Some(path) if purpose == "place" && path.to_ascii_lowercase().ends_with(".pdf") => {
                    let pages =
                        self.services.read.as_mut().and_then(|r| r(&path).ok()).and_then(|b| designcraft_render::pdf_page_count(&b)).unwrap_or(1);
                    if pages > 1 {
                        self.ui.dialog =
                            Some(dialogs::Dialog::new("pdfImport", json!({"path": path, "page": "1", "pages": pages, "target": target})));
                        Ok(Value::Null)
                    } else {
                        self.place_file(json!({"path": path}))
                    }
                }
                Some(path) if purpose == "place" => self.place_file(json!({"path": path})),
                Some(path) => self.open_file("file.open", json!({"path": path})),
                None => Ok(Value::Null),
            };
        }
        let request = self.import_request(purpose);
        if let Some(open) = self.services.open_async.as_mut() {
            open(request);
        }
        Ok(Value::Null)
    }

    /// Load Styles: ask the host for a document whose styles to load; `kind` is `paragraph`,
    /// `character` or empty for both. A synchronous picker opens the dialog that chooses among
    /// the file's styles; an asynchronous one (web) loads them all when the file arrives.
    pub fn pick_styles(&mut self, kind: &str) -> Result<Value, String> {
        designcraft_engine::cmd::has_doc(&self.session)?;
        if let Some(pick) = self.services.pick_open.as_mut() {
            let Some(path) = pick("styles") else { return Ok(Value::Null) };
            let found = self.run("style.load.list", json!({"path": path}))?;
            let of = |k: &str| if kind.is_empty() || kind == k { found[k].clone() } else { json!([]) };
            self.ui.dialog = Some(dialogs::Dialog::new(
                "loadStyles",
                json!({"path": path, "paragraph": of("paragraph"), "character": of("character"), "conflicts": "redefine"}),
            ));
            return Ok(Value::Null);
        }
        let request = self.import_request(&if kind.is_empty() { "styles".to_string() } else { format!("styles:{kind}") });
        if let Some(open) = self.services.open_async.as_mut() {
            open(request);
        }
        Ok(Value::Null)
    }

    pub fn import_request(&self, purpose: &str) -> ImportRequest {
        ImportRequest { purpose: purpose.into(), target: self.session.active().map(|d| d.uid) }
    }

    pub(crate) fn activate_import_target(&mut self, request: &ImportRequest) -> Result<(), String> {
        let Some(index) = request.target.and_then(|uid| self.session.documents().iter().position(|d| d.uid == uid)) else {
            let message = "Import cancelled: the originating document has closed";
            self.status(crate::i18n::tr(&self.ui.language, message));
            return Err(message.into());
        };
        self.run("file.activate", json!({"index": index}))?;
        Ok(())
    }

    pub(crate) fn cancel_pdf_import(&mut self) {
        self.pending_pdf = None;
    }

    pub(crate) fn confirm_pdf_import(&mut self, page: u64, crop: String) -> Result<Value, String> {
        let file = self.pending_pdf.take().ok_or("PDF import cancelled: no pending file")?;
        self.activate_import_target(&file.request)?;
        let params = json!({"name": file.name, "base64": designcraft_engine::cmd::base64_encode(&file.bytes), "pdfPage": page, "pdfCrop": crop});
        if file.request.purpose == "place" { self.place_file(params) } else { self.open_file("file.place", params) }
    }

    /// A single unresolved PDF owns its bytes. Later deliveries cancel explicitly rather than
    /// replacing its options. Cancellation, replacement and target closure release its payload.
    fn drain_inbox(&mut self) {
        if let Some(file) = &self.pending_pdf {
            let target_exists = self.session.documents().iter().any(|d| Some(d.uid) == file.request.target);
            let owns_dialog =
                self.ui.dialog.as_ref().is_some_and(|d| d.id == "pdfImport" && d.fields.get("async").and_then(Value::as_bool) == Some(true));
            if !target_exists || !owns_dialog {
                self.pending_pdf = None;
                if !target_exists {
                    if owns_dialog {
                        self.ui.dialog = None;
                    }
                    self.status(crate::i18n::tr(&self.ui.language, "PDF import cancelled: the originating document has closed"));
                }
            }
        }
        let Some(inbox) = self.services.inbox.clone() else { return };
        let files = std::mem::take(&mut *inbox.lock().unwrap_or_else(std::sync::PoisonError::into_inner));
        for file in files {
            if file.request.purpose == "shortcuts" {
                shortcut_import::import_delivered(self, &file.bytes);
                continue;
            }
            if self.ui.dialog.is_some() {
                self.status(
                    crate::i18n::tr(&self.ui.language, "{name}: import cancelled while a dialog is open; choose the file again")
                        .replace("{name}", &file.name),
                );
                continue;
            }
            let name = file.name.clone();
            if let Err(e) = self.import_file(file) {
                self.status(format!("{name}: {e}"));
            }
        }
    }

    fn import_file(&mut self, file: ImportedFile) -> Result<Value, String> {
        let lower = file.name.to_ascii_lowercase();
        if let Some(kind) = file.request.purpose.strip_prefix("styles") {
            self.activate_import_target(&file.request)?;
            let mut params = json!({"name": file.name, "base64": designcraft_engine::cmd::base64_encode(&file.bytes)});
            match kind {
                ":paragraph" => params["character"] = json!([]),
                ":character" => params["paragraph"] = json!([]),
                _ => {}
            }
            return self.run("style.load", params);
        }
        let layout = opens_as_document(&file.name)
            || (std::path::Path::new(&file.name).extension().is_none() && designcraft_engine::cmd::is_document(&file.bytes));
        let place =
            file.request.purpose == "place" || (matches!(file.request.purpose.as_str(), "drop" | "open") && !layout && !lower.ends_with(".ase"));
        if place || lower.ends_with(".ase") || file.request.purpose == "swatches" {
            self.activate_import_target(&file.request)?;
        }
        if place && lower.ends_with(".pdf") {
            let pages = designcraft_render::pdf_page_count(&file.bytes).unwrap_or(1);
            if pages > 1 {
                self.ui.dialog =
                    Some(dialogs::Dialog::new("pdfImport", json!({"async": true, "name": file.name.clone(), "page": "1", "pages": pages})));
                self.pending_pdf = Some(file);
                return Ok(Value::Null);
            }
        }
        let b64 = designcraft_engine::cmd::base64_encode(&file.bytes);
        if lower.ends_with(".ase") || file.request.purpose == "swatches" {
            self.run("swatch.load", json!({"base64": b64}))
        } else if file.request.purpose == "place" {
            self.place_file(json!({"name": file.name, "base64": b64}))
        } else if place {
            self.open_file("file.place", json!({"name": file.name, "base64": b64}))
        } else {
            let title = file.name.rsplit_once('.').map_or(file.name.as_str(), |(stem, _)| stem);
            self.open_file("file.openBytes", json!({"name": title, "base64": b64}))
        }
    }

    /// Applies `ui.shortcuts` to the keymap when they differ, e.g. after preferences were loaded,
    /// and rebuilds it when the shortcut platform changed (the web build learns the browser's
    /// operating system on the first frame). Outside the Keyboard Shortcuts dialog the active
    /// shortcut set takes `ui.shortcuts` too: an earlier version may have changed them.
    pub fn sync_keymap(&mut self) {
        if self.shortcut_dialog.is_none() {
            keys::store_in_set(&mut self.ui);
        }
        if self.keymap.platform() != keys::platform() {
            self.keymap = keys::new_keymap(self.ui.shortcuts.clone());
        } else if *self.keymap.overrides() != self.ui.shortcuts {
            self.keymap.set_overrides(self.ui.shortcuts.clone());
        }
    }

    /// Whether the Keyboard Shortcuts dialog is recording keys: a native menu bar pauses its key
    /// equivalents meanwhile, so the keys reach the dialog.
    pub fn shortcut_recording(&self) -> bool {
        self.shortcut_dialog.as_ref().is_some_and(|d| d.recording())
    }

    pub fn select_tool(&mut self, id: &str) {
        let _ = self.run("tool.select", json!({"tool": id}));
        self.ui.flyout = None;
    }

    /// First frame only: load the saved colour profiles and make the saved Color Settings active.
    fn apply_saved_color_settings(&mut self) {
        if self.color_applied {
            return;
        }
        self.color_applied = true;
        for path in self.ui.color_profiles.clone() {
            if let Err(e) = self.session.execute("color.loadProfile", &json!({"path": path})) {
                log::warn!("Color Settings: {e}");
                self.status_error("Color Settings", e);
            }
        }
        if let Some(cs) = self.ui.color_settings.clone() {
            let known = cs.with_known_profiles();
            for (saved, used) in [(&cs.rgb, &known.rgb), (&cs.cmyk, &known.cmyk)] {
                if saved != used {
                    let e = format!("saved profile {saved} is not available; using {used}");
                    log::warn!("Color Settings: {e}");
                    self.status_error("Color Settings", e);
                }
            }
            let _ = designcraft_color::cms::set_active(&known);
        }
    }

    /// Install the UI fonts on the first frame, and again when the language or the system fonts change.
    fn update_fonts(&mut self, ctx: &egui::Context) {
        if !self.styled {
            // The UI scale is a preference (applied above); egui's own Cmd+Plus/Minus/0 would rescale the
            // whole window instead of the canvas, only to be reset next frame.
            ctx.options_mut(|o| o.zoom_with_keyboard = false);
            self.fonts_system = theme::install_fonts(ctx, &self.ui.language);
            self.fonts_lang = self.ui.language.clone();
            self.styled = true;
            self.restyle = true;
        } else {
            if self.fonts_lang != self.ui.language || (!self.fonts_system && theme::system_fonts_ready()) {
                self.fonts_system = theme::install_fonts(ctx, &self.ui.language);
                self.fonts_lang = self.ui.language.clone();
            }
            self.fonts_ready = true;
        }
    }

    /// Queue and deadline work, run once per app callback.
    ///
    /// `logic_tick` runs on every call (a first pass, a later discarded pass, or a hidden
    /// `run_logic` tick) and consumes queues and deadlines once. Work that is genuinely once per
    /// presented layout frame lives in [`Self::ui_setup`] / [`Self::ui_frame_end`], which only a
    /// real `run_ui` pass calls. A hidden `run_logic` tick also reports pass index zero, so the
    /// pass index alone cannot identify a UI pass.
    pub fn logic(&mut self, ctx: &egui::Context) {
        #[cfg(test)]
        {
            self.logic_tick_ticks += 1;
        }
        self.logic_tick(ctx);
    }

    /// Work whose identity is a consumed queue item, revision or deadline.
    fn logic_tick(&mut self, ctx: &egui::Context) {
        self.update_screenshot_epoch(ctx);
        self.drain_control(ctx);
        updates_ui::tick(self, ctx);
        if self.fonts_ready && !self.host_order_pending() {
            self.drain_inbox();
            self.open_os_documents();
        }
        if self.host_order_pending() {
            ctx.request_repaint();
        }
        self.collect_screenshots(ctx);
        // Ending a stale preview is state cleanup after a control/menu action, not a frame effect:
        // it stays here so a hidden tick still closes a preview whose dialog has gone.
        if !self.host_order_pending() {
            new_document::end_stale_preview(self);
        }
        self.request_hidden_paint_for_pending_input(ctx);
        self.handle_quit(ctx);
        self.flush_window_commands(ctx);
    }

    /// The first pass of a real `run_ui`: fonts, theme, scale and window state, before any widget
    /// is laid out. It must run before the font gate in [`Self::ui`], or the app would wait for
    /// fonts that this function is what installs.
    pub(crate) fn ui_setup(&mut self, ctx: &egui::Context) {
        #[cfg(test)]
        {
            self.logic_frame_ticks += 1;
        }
        if self.egui_ctx.is_none() {
            self.egui_ctx = Some(ctx.clone());
        }
        keys::note_platform(ctx);
        self.sync_keymap();
        self.window_maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
        self.apply_saved_color_settings();
        let scale = self.ui.ui_scale.clamp(0.5, 3.0);
        if (ctx.zoom_factor() - scale).abs() > 1e-3 {
            ctx.set_zoom_factor(scale);
        }
        self.update_fonts(ctx);
        if !self.fonts_system {
            // The font scan runs in the background: look again shortly.
            ctx.request_repaint_after(std::time::Duration::from_millis(250));
        }
        if self.restyle {
            theme::apply(ctx, &theme::Tokens::for_brightness(self.ui.brightness));
            self.restyle = false;
        }
    }

    /// The end of a settled `run_ui` pass: pacing, recovery and screenshot settlement. Recovery
    /// uses the monotonic clock, so a wall-clock/NTP jump cannot postpone or advance it.
    fn ui_frame_end(&mut self, ctx: &egui::Context) {
        let now = now_ms() / 1000.0;
        // Crash recovery: unsaved documents are written to the recovery folder every 30 s.
        #[cfg(not(target_arch = "wasm32"))]
        if self.session.recovery_dir.is_some() && now - self.last_recovery > (self.session.prefs.recovery_minutes * 60.0).max(5.0) {
            self.last_recovery = now;
            if self.session.documents().iter().any(|d| d.is_dirty())
                && let Err(e) = self.session.execute("file.recovery.save", &json!({}))
            {
                self.status(crate::i18n::tr(&self.ui.language, "Couldn't write recovery data: {error}").replace("{error}", &e.to_string()));
            }
        }
        let dt = now - self.last_time;
        if dt > 0.0 {
            self.perf.fps = self.perf.fps * 0.9 + (1.0 / dt).min(240.0) * 0.1;
        }
        self.last_time = now;
        self.issue_screenshots(ctx);
    }

    /// Keep pending synthetic input and screenshot settlement repainting while the window may be
    /// hidden.
    ///
    /// `visible() == None` means unknown and is treated like hidden. The command holds for one
    /// frame and is sent once per tick until a settled UI pass has taken the injected step or
    /// issued the queued capture.
    fn request_hidden_paint_for_pending_input(&self, ctx: &egui::Context) {
        let pending = self.host_order_pending() || !self.queued_shots.is_empty();
        if !pending || ctx.input(|i| i.viewport().visible()) == Some(true) {
            return;
        }
        // The 0.37 web backend ignores `RequestPaintWhileHidden` until the fork carries it. Do not
        // pretend a hidden-tab capture or gesture will run there; the request times out instead.
        #[cfg(not(target_arch = "wasm32"))]
        ctx.send_viewport_cmd(egui::ViewportCommand::RequestPaintWhileHidden);
    }

    /// Advance the screenshot geometry epoch whenever size, DPI, zoom or surface change.
    fn update_screenshot_epoch(&mut self, ctx: &egui::Context) {
        let Some(mut geometry) = screenshot_geometry(ctx) else { return };
        geometry.document = self.active_document_epoch();
        if self.screenshot_geometry != Some(geometry) {
            self.screenshot_geometry = Some(geometry);
            self.screenshot_epoch = self.screenshot_epoch.wrapping_add(1);
        }
    }

    /// Quit: close the window the way its close button does, so unsaved documents ask first.
    pub fn request_quit(&mut self) {
        self.quit_requested = true;
    }

    /// Ask about unsaved documents without closing any of them yet. A later Cancel must also
    /// keep documents whose earlier answer was Don't Save, including their history and recovery.
    pub(crate) fn handle_quit(&mut self, ctx: &egui::Context) {
        let close_requested = ctx.input(|i| i.viewport().close_requested());
        if self.host_order_pending() {
            if close_requested {
                self.close_requested_pending = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            }
            ctx.request_repaint();
            return;
        }
        let replay_close = std::mem::take(&mut self.close_requested_pending);
        if std::mem::take(&mut self.quit_requested) || replay_close {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        if close_requested || replay_close {
            if self.quit_without_asking {
                self.quitting = None;
                return;
            }
            if self.quitting.is_none() && !self.session.documents().iter().any(|d| d.is_dirty()) {
                return;
            }
            let approved =
                self.quitting.as_ref().is_some_and(|q| q.pending.is_none()) && self.ui.dialog.is_none() && self.next_unapproved_document().is_none();
            if approved {
                // Commit only when the backend receives the final close, after checking for
                // edits that arrived since the last answer. Closing also discards recovery data.
                if let Err(e) = self.finish_confirmed_quit() {
                    self.status(e);
                    self.cancel_quit();
                    ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                }
                return;
            }
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            // Dock Quit can arrive while the app is hidden or minimized. Make its question
            // visible through the normal safe window commands before asking about documents.
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            if self.quitting.is_none() {
                self.quitting = Some(QuitState { original_active: self.session.active().map(|d| d.uid), accepted: HashMap::new(), pending: None });
            }
        }
        let Some(q) = self.quitting.as_ref() else { return };
        if let Some(uid) = q.pending {
            let asking = self.ui.dialog.as_ref().is_some_and(|d| {
                d.id == "closeDocument"
                    && d.fields.get("quitting").and_then(Value::as_bool) == Some(true)
                    && d.fields.get("uid").and_then(Value::as_u64) == Some(uid)
            });
            if !asking {
                self.cancel_quit();
            }
            return;
        }
        if self.ui.dialog.is_some() {
            self.cancel_quit();
            return;
        }
        self.ask_next_unsaved(ctx);
    }

    /// Finalize a complete, still-current quit plan at an orderly host exit. The host can
    /// terminate before egui receives its queued Close; an unfinished/stale plan leaves every
    /// document live so the host can preserve its recovery data instead.
    pub fn finish_confirmed_quit(&mut self) -> Result<(), String> {
        let approved =
            self.quitting.as_ref().is_some_and(|q| q.pending.is_none()) && self.ui.dialog.is_none() && self.next_unapproved_document().is_none();
        if !approved {
            return Ok(());
        }
        while !self.session.documents().is_empty() {
            self.run("file.close", json!({"index": 0}))?;
        }
        self.quitting = None;
        Ok(())
    }

    fn next_unapproved_document(&self) -> Option<(usize, u64)> {
        let q = self.quitting.as_ref()?;
        self.session
            .documents()
            .iter()
            .enumerate()
            .find(|(_, d)| d.is_dirty() && q.accepted.get(&d.uid) != Some(&d.revision))
            .map(|(i, d)| (i, d.uid))
    }

    fn cancel_quit(&mut self) {
        let Some(q) = self.quitting.take() else { return };
        if let Some(i) = q.original_active.and_then(|uid| self.session.documents().iter().position(|d| d.uid == uid)) {
            let _ = self.run("file.activate", json!({"index": i}));
        }
    }

    /// Save/Don't Save during Quit approves this UID's current revision, without destroying it.
    pub(crate) fn confirm_quit_document(&mut self, uid: u64, discard: bool) -> Result<Value, String> {
        if self.quitting.as_ref().and_then(|q| q.pending) != Some(uid) {
            return Err("Quit confirmation is no longer active".into());
        }
        let Some(i) = self.session.documents().iter().position(|d| d.uid == uid) else {
            if let Some(q) = self.quitting.as_mut() {
                q.pending = None;
            }
            return Ok(Value::Null);
        };
        if !discard {
            let saved = self.run("file.activate", json!({"index": i})).and_then(|_| self.run("app.save", json!({})));
            if let Err(e) = saved {
                self.cancel_quit();
                return Err(e);
            }
            if self.session.documents().iter().find(|d| d.uid == uid).is_none_or(|d| d.is_dirty()) {
                // No file chosen (or an asynchronous host save has not completed): stop quitting.
                self.cancel_quit();
                return Ok(Value::Null);
            }
        }
        if let Some(q) = self.quitting.as_mut() {
            if let Some(d) = self.session.documents().iter().find(|d| d.uid == uid) {
                q.accepted.insert(uid, d.revision);
            }
            q.pending = None;
        }
        Ok(Value::Null)
    }

    /// Show the next unapproved document, or request a final close that revalidates all approvals.
    fn ask_next_unsaved(&mut self, ctx: &egui::Context) {
        let Some((i, uid)) = self.next_unapproved_document() else {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        };
        if self.run("file.activate", json!({"index": i})).is_err() {
            self.cancel_quit();
            return;
        }
        menus::close_document(self, Some(i));
        if let Some(d) = self.ui.dialog.as_mut() {
            d.fields.insert("quitting".into(), json!(true));
        }
        if let Some(q) = self.quitting.as_mut() {
            q.pending = Some(uid);
        }
    }

    /// A file dropped on the window: documents open, the swatches in a swatch exchange (.ase) file
    /// are loaded, and anything else is placed on the spread in view (a parent spread while parents
    /// are being edited). A file without an extension opens when its content is a DesignCraft or
    /// IDML document.
    pub fn open_dropped(&mut self, path: &str) -> Result<Value, String> {
        self.open_dropped_at(path, None)
    }

    /// Open or place each file dropped on a floating panel's own window. `pointer` is in the main
    /// window's coordinates.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn open_dropped_files(&mut self, files: &[egui::DroppedFileHandle], pointer: Option<egui::Pos2>) {
        for f in files {
            let p = f.path().to_string_lossy().to_string();
            if !p.is_empty() {
                let _ = self.open_dropped_at(&p, pointer);
            }
        }
    }

    /// [`Self::open_dropped`] at the pointer: a graphic's top left lands where the file was
    /// dropped on the canvas, text goes to a new frame on the document page there.
    pub fn open_dropped_at(&mut self, path: &str, pointer: Option<egui::Pos2>) -> Result<Value, String> {
        if path.to_ascii_lowercase().ends_with(".ase") {
            return self.run("swatch.load", json!({"path": path}));
        }
        let document = opens_as_document(path)
            || (std::path::Path::new(path).extension().is_none()
                && designcraft_engine::io::read_capped(path, designcraft_engine::cmd::DOCUMENT_SNIFF_BYTES)
                    .is_ok_and(|b| designcraft_engine::cmd::is_document(&b)));
        if document {
            return self.open_file("file.open", json!({"path": path}));
        }
        let mut params = json!({"path": path});
        if let (Some((sr, at)), Some(o)) = (canvas::drop_point(self, pointer), params.as_object_mut()) {
            o.insert("spread".into(), serde_json::to_value(sr).unwrap_or(Value::Null));
            o.insert("x".into(), json!(at.x));
            o.insert("y".into(), json!(at.y));
            if let (designcraft_doc::SpreadRef::Doc(si), Some(st)) = (sr, self.session.active()) {
                let pi = st.doc.spreads.get(si).and_then(|s| s.page_at_x(at.x)).unwrap_or(0);
                o.insert("page".into(), json!(st.doc.first_page_of_spread(si) + pi + 1));
            }
        }
        self.open_file("file.place", params)
    }

    /// Queue files the operating system asked the app to open. They can arrive before the window
    /// exists (the event that launched the app); the first ready frame opens them, each as a new
    /// document through `file.open`, as File › Open does.
    pub fn open_documents_later(&mut self, paths: impl IntoIterator<Item = String>) {
        self.os_documents.extend(paths);
    }

    pub(crate) fn open_os_documents(&mut self) {
        for path in std::mem::take(&mut self.os_documents) {
            if let Err(e) = self.open_file("file.open", json!({"path": path})) {
                log::warn!("{path}: {e}");
            }
        }
    }

    /// File › Place: the file fills the selected empty or graphic frame; with none, an image loads
    /// into the place cursor (click to place at its natural size, drag to draw the frame).
    pub(crate) fn place_file(&mut self, mut params: Value) -> Result<Value, String> {
        if let Some(o) = params.as_object_mut() {
            o.insert("loadCursor".into(), json!(true));
        }
        self.open_file("file.place", params)
    }

    /// Open (`file.open`, `file.openBytes`) or place (`file.place`) a file the user chose. A file
    /// that can't be opened or placed gets an alert with the reason, as well as the status line.
    /// A successful import with warnings keeps the document and shows the warnings in an alert.
    pub fn open_file(&mut self, cmd: &str, params: Value) -> Result<Value, String> {
        let file = match params.get("path").and_then(Value::as_str) {
            Some(path) => std::path::Path::new(path).file_name().map_or_else(|| path.to_string(), |n| n.to_string_lossy().to_string()),
            None => params.get("name").and_then(Value::as_str).unwrap_or_default().to_string(),
        };
        self.open_named_file(cmd, params, file)
    }

    /// [`Self::open_file`] for a command whose params don't name the file: `file` is shown in the
    /// alert or warnings.
    pub fn open_named_file(&mut self, cmd: &str, params: Value, file: String) -> Result<Value, String> {
        let r = self.run(cmd, params);
        if let Err(e) = &r {
            let title = if cmd == "file.place" { "Can't Place the File" } else { "Can't Open the File" };
            if self.ui.dialog.as_ref().is_some_and(|d| d.id == "findChange") {
                self.session.reset_find_scope();
            }
            self.ui.dialog = Some(dialogs::Dialog::new("alert", json!({"title": title, "file": file, "message": e})));
        } else if let Ok(result) = &r {
            let warnings: Vec<&str> = result
                .get("warnings")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .filter(|warning| !warning.is_empty())
                .collect();
            if !warnings.is_empty() {
                self.status(format!("{file}: {}", warnings.join("; ")));
                // Find Font, offered for the document's missing fonts, stays up: the status line
                // keeps the warnings.
                if self.ui.dialog.as_ref().is_none_or(|d| d.id != "findFont") {
                    self.ui.dialog =
                        Some(dialogs::Dialog::new("alert", json!({"title": "Import Warnings", "file": file, "message": warnings.join("\n")})));
                }
            }
        }
        r
    }

    /// Deliver window actions to the host without persisting them in UI preferences.
    pub(crate) fn flush_window_commands(&mut self, ctx: &egui::Context) {
        for command in std::mem::take(&mut self.pending_window_commands) {
            ctx.send_viewport_cmd(command);
        }
    }

    /// Keep widget clipboard output private while a queued control event is running.
    pub(crate) fn install_clipboard_guard(&mut self, ctx: &egui::Context) {
        if self.clipboard_guard_installed {
            return;
        }
        let private = self.control_input_frame.clone();
        ctx.on_end_pass(
            "control clipboard",
            std::sync::Arc::new(move |ui| {
                if private.load(std::sync::atomic::Ordering::Relaxed) {
                    ui.ctx().output_mut(|o| {
                        o.commands.retain(|cmd| !matches!(cmd, egui::OutputCommand::CopyText(_) | egui::OutputCommand::CopyImage(_)))
                    });
                }
            }),
        );
        self.clipboard_guard_installed = true;
    }

    /// Inject synthetic events (one pointer event per host iteration), retaining control
    /// provenance until a settled UI pass has actually taken the batch.
    pub fn raw_input_hook(&mut self, ctx: &egui::Context, raw: &mut egui::RawInput) {
        if !self.injected_settle_hook {
            let pending = self.injected_pending.clone();
            let settled = self.injected_settled.clone();
            ctx.on_end_pass(
                "injected input settlement",
                std::sync::Arc::new(move |ui| {
                    if ui.ctx().viewport_id() == egui::ViewportId::ROOT
                        && pending.load(std::sync::atomic::Ordering::Relaxed)
                        && !ui.ctx().will_discard()
                    {
                        settled.store(true, std::sync::atomic::Ordering::Relaxed);
                    }
                }),
            );
            self.injected_settle_hook = true;
        }
        // A settled pass that did not go through `DesignApp::ui` (a harness consuming raw input
        // directly) still consumed the batch; acknowledge it before considering the next step.
        if self.injected_settled.swap(false, std::sync::atomic::Ordering::Relaxed) {
            self.synthetic_injected = false;
            self.injected_events.clear();
            self.injected_pending.store(false, std::sync::atomic::Ordering::Relaxed);
            self.injected_private = false;
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let pointer = raw.events.iter().rev().find_map(|event| match event {
                egui::Event::PointerMoved(pos) | egui::Event::PointerButton { pos, .. } => Some(*pos),
                _ => None,
            });
            let generation = self.input_generation();
            for file in std::mem::take(&mut raw.dropped_files) {
                let path = file.path().to_string_lossy().to_string();
                if !path.is_empty() {
                    self.pending_drops.push(PendingDrop { path, pointer, generation });
                }
            }
        }
        self.discard_stale_injected_batch(raw);
        // A batch acknowledged by the previous pass keeps its private provenance through that
        // pass's end hook (the clipboard guard), and is cleared at the next host iteration.
        if !self.synthetic_injected {
            self.injected_private = false;
        }
        let private = self.injected_private || !self.control_synthetic.is_empty();
        self.control_input_frame.store(private, std::sync::atomic::Ordering::Relaxed);
        // While the previous batch is still unapplied (it lives in eframe's pending raw input),
        // inject nothing more: one pointer step per UI pass, not per hidden logic tick.
        if self.synthetic_injected {
            return;
        }
        let private = !self.control_synthetic.is_empty();
        let events = if private { &mut self.control_synthetic } else { &mut self.synthetic };
        let Some(first) = events.first() else { return };
        let n = match first {
            egui::Event::PointerMoved(_) | egui::Event::PointerButton { .. } => 1,
            _ => events.iter().position(|e| matches!(e, egui::Event::Key { pressed: false, .. })).map_or(events.len(), |i| i + 1),
        };
        let batch: Vec<egui::Event> = events.drain(..n).collect();
        self.injected_generation = self.input_generation();
        self.injected_private = private;
        self.injected_events = batch.clone();
        self.injected_pending.store(true, std::sync::atomic::Ordering::Relaxed);
        raw.events.extend(batch);
        // Held until a settled UI pass runs with the fonts installed; a hidden logic tick, a
        // discarded multipass first pass or a font-gated early return must not acknowledge it.
        self.synthetic_injected = true;
    }

    /// Notify after enqueuing a native menu event, before waking the host. The returned callback
    /// owns its state so the event handler never borrows the app.
    pub fn native_action_notifier(&self, ctx: egui::Context) -> Box<dyn Fn() + Send + Sync> {
        let pending = self.native_events_pending.clone();
        Box::new(move || {
            pending.store(true, std::sync::atomic::Ordering::Release);
            ctx.request_repaint();
        })
    }

    /// Poll from the first real UI pass, under the host's normal UI/watchdog guard. Resolve menu
    /// choices before refreshing their IDs, then execute them once synthetic input has settled.
    /// Event arrival already blocks logic deliveries, so a deferred action keeps its ordering
    /// without executing native dialogs or menu commands from a hidden logic-only tick.
    pub fn poll_native_actions(&mut self, ctx: &egui::Context, mut next: impl FnMut() -> Option<(String, Value)>) {
        // Clear before draining: a notification arriving during the drain stays set for the
        // next pass, even when its channel send races the final empty receive.
        self.native_events_pending.swap(false, std::sync::atomic::Ordering::AcqRel);
        while let Some((id, params)) = next() {
            self.native_actions.push_back((id, params));
        }
        self.apply_native_actions(ctx);
    }

    /// True while an injected batch, control synthetic event or ordinary synthetic event remains queued.
    fn synthetic_input_pending(&self) -> bool {
        self.synthetic_injected || !self.control_synthetic.is_empty() || !self.synthetic.is_empty()
    }

    /// True when synthetic input, a queued native action or a pending native-event notification remains.
    fn host_order_pending(&self) -> bool {
        self.synthetic_input_pending() || !self.native_actions.is_empty() || self.native_events_pending.load(std::sync::atomic::Ordering::Acquire)
    }

    /// Execute queued native actions only after synthetic input settles, and request another repaint while host work remains.
    fn apply_native_actions(&mut self, ctx: &egui::Context) {
        while !self.synthetic_input_pending() {
            let Some((id, params)) = self.native_actions.pop_front() else { break };
            menus::activate_native(self, ctx, &id, &params);
        }
        if self.host_order_pending() {
            ctx.request_repaint();
        }
    }

    /// The current state an injected batch belongs to.
    pub(crate) fn input_generation(&self) -> InputGeneration {
        InputGeneration {
            document: self.active_document_epoch(),
            dialog: self.ui.dialog.as_ref().map(|dialog| egui::Id::unique(&dialog.id)),
            focus: self.egui_ctx.as_ref().and_then(|ctx| ctx.memory(|memory| memory.focused())),
        }
    }

    /// Drop a batch whose document, dialog or focus changed before a UI pass could take it.
    /// The batch is still in the host's pending raw input; remove exactly the events we appended
    /// (contiguously, in order) so a subsequent forced paint cannot replay stale input.
    fn discard_stale_injected_batch(&mut self, raw: &mut egui::RawInput) {
        if !self.synthetic_injected || self.input_generation() == self.injected_generation {
            return;
        }
        let events = std::mem::take(&mut self.injected_events);
        if !events.is_empty()
            && let Some(start) = raw.events.windows(events.len()).rposition(|window| window == events.as_slice())
        {
            raw.events.drain(start..start + events.len());
        }
        self.synthetic_injected = false;
        self.injected_pending.store(false, std::sync::atomic::Ordering::Relaxed);
        self.injected_private = false;
        self.control_input_frame.store(false, std::sync::atomic::Ordering::Relaxed);
    }

    /// A font-gated first pass saw the events before any widget did. Put the same batch back at
    /// the front of its queue (with the original provenance) so a later UI pass consumes it.
    fn requeue_injected_batch(&mut self) {
        if !std::mem::take(&mut self.synthetic_injected) {
            return;
        }
        let events = std::mem::take(&mut self.injected_events);
        if !events.is_empty() {
            if self.injected_private {
                self.control_synthetic.splice(0..0, events);
            } else {
                self.synthetic.splice(0..0, events);
            }
        }
        self.injected_pending.store(false, std::sync::atomic::Ordering::Relaxed);
        self.injected_private = false;
        self.control_input_frame.store(false, std::sync::atomic::Ordering::Relaxed);
    }

    /// The document area: the start screen, or the document tabs over one or two canvas panes.
    pub(crate) fn canvas_area(&mut self, ui: &mut egui::Ui, presenting: bool) {
        let t = theme::Tokens::get(ui.ctx());
        if self.session.active().is_none() {
            chrome::start_screen(self, ui);
            return;
        }
        if !presenting {
            chrome::doc_tabs(self, ui);
        }
        let split_layout = (self.split && !presenting)
            .then(|| {
                craft_ui::layout::split_rect(
                    ui.available_rect_before_wrap(),
                    craft_ui::layout::SplitAxis::Horizontal,
                    craft_ui::layout::SplitSize::Ratio(0.5),
                    2.0,
                    0.0,
                )
            })
            .flatten();
        if let Some(layout) = split_layout {
            // Two panes with a divider; each has its own view (zoom, scroll) and render cache.
            ui.painter().rect_filled(layout.divider, 0.0, t.border);
            // Resolve the clicked pane before either canvas handles this frame's keys.
            if let Some(p) = ui.input(|i| i.pointer.any_pressed().then(|| i.pointer.interact_pos()).flatten()) {
                if layout.first.contains(p) {
                    self.focus_pane = 0;
                } else if layout.second.contains(p) {
                    self.focus_pane = 1;
                }
            }
            for (k, half) in [layout.first, layout.second].into_iter().enumerate() {
                self.switch_pane(k as u8);
                // A new pane starts at the other pane's view.
                if k == 1 && self.view().is_none() {
                    self.switch_pane(0);
                    let v = self.view().copied();
                    self.switch_pane(1);
                    if let (Some(mut v), Some(m)) = (v, self.view_mut()) {
                        v.fitted = false;
                        *m = v;
                    }
                }
                ui.scope_builder(egui::UiBuilder::new().max_rect(half), |ui| canvas::show(self, ui));
            }
            self.switch_pane(self.focus_pane);
        } else {
            self.switch_pane(0);
            // With a second window, the main one takes the focus back when clicked.
            if !self.second_window || (ui.input(|i| i.pointer.any_pressed()) && ui.ui_contains_pointer()) {
                self.focus_pane = 0;
            }
            canvas::show(self, ui);
        }
    }

    /// Lay out the whole window.
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        #[cfg(test)]
        {
            self.input_ticks += 1;
        }
        let ctx = ui.ctx().clone();
        self.canvas_ime = false;
        self.ime_canvas = None;
        self.ime_interrupt = false;
        // Fonts/theme must be installed from the real UI pass, before the font gate below: doing
        // this only from a hidden logic tick would make the first shown pass wait forever.
        if ctx.current_pass_index() == 0 {
            self.ui_setup(&ctx);
        }
        if !self.fonts_ready {
            // The first pass installs fonts and returns before any widget sees this frame's input.
            // Put an injected step back rather than acknowledge it as consumed.
            self.requeue_injected_batch();
            ctx.request_repaint();
            return;
        }
        self.settle_live_preview();
        // Input consumers run once, in the live first UI pass, before layout. Hidden logic ticks
        // must not replay shortcuts or dropped files from stale input.
        if ctx.current_pass_index() == 0 {
            #[cfg(not(target_arch = "wasm32"))]
            {
                // Sample the generation once for the whole batch: placing the first valid file
                // must not suppress later files that belong to the same captured batch.
                let current = self.input_generation();
                for drop in std::mem::take(&mut self.pending_drops) {
                    if drop.generation != current {
                        continue;
                    }
                    let _ = self.open_dropped_at(&drop.path, drop.pointer);
                }
            }
            menus::shortcuts(self, &ctx);
        }
        let t0 = now_ms();
        let t = theme::Tokens::get(&ctx);
        let presenting = self.ui.screen_mode == ScreenMode::Presentation;
        if presenting || self.ui.hidden_panels != 0 {
            panel_docking::cancel_gesture(&ctx);
        }
        canvas::begin_frame(&ctx);
        let dbg = std::env::var_os("DESIGNCRAFT_DEBUG_LAYOUT").is_some();
        if dbg {
            eprintln!("root start {:?}", ui.available_rect_before_wrap());
        }
        if !presenting {
            chrome::app_bar(self, ui);
            if dbg {
                eprintln!("after app bar {:?}", ui.available_rect_before_wrap());
            }
            if self.ui.control_bar && self.session.active().is_some() && self.ui.hidden_panels == 0 {
                chrome::control_bar(self, ui);
            }
            chrome::status_bar(self, ui);
            if dbg {
                eprintln!("after status {:?}", ui.available_rect_before_wrap());
            }
            if self.ui.hidden_panels != 1 {
                toolbar::show(self, ui);
            }
        }
        egui::CentralPanel::default().frame(egui::Frame::NONE.fill(t.pasteboard)).show(ui, |ui| {
            // The panel layout holds the canvas as its protected `canvas` panel.
            if presenting || self.ui.hidden_panels != 0 {
                self.canvas_area(ui, presenting);
            } else {
                panel_docking::show(self, ui);
            }
        });
        // New Window: pane 1 in a window of its own.
        if self.second_window
            && !presenting
            && let Some(title) = self.session.active().map(|d| format!("{} — 2", d.doc.title))
        {
            let fill = t.pasteboard;
            let ctx = ui.ctx().clone();
            ctx.show_viewport_immediate(
                second_window_viewport(),
                egui::ViewportBuilder::default().with_title(title).with_inner_size([960.0, 720.0]),
                |ui, _class| {
                    egui::CentralPanel::default().frame(egui::Frame::NONE.fill(fill)).show(ui, |ui| {
                        self.switch_pane(1);
                        if self.view().is_none() {
                            self.switch_pane(0);
                            let v = self.view().copied();
                            self.switch_pane(1);
                            if let (Some(mut v), Some(m)) = (v, self.view_mut()) {
                                v.fitted = false;
                                *m = v;
                            }
                        }
                        if ui.input(|i| i.pointer.any_pressed()) {
                            self.focus_pane = 1;
                        }
                        canvas::show(self, ui);
                    });
                    if ui.input(|i| i.viewport().close_requested()) {
                        self.end_second_ime_viewport();
                        self.second_window = false;
                        self.focus_pane = 0;
                    }
                },
            );
            self.switch_pane(self.focus_pane);
        } else if self.second_window {
            // The child viewport does not run this pass (Presentation, or no document for its
            // title): its pass counter stops advancing, so treat it as gone and let the tracker
            // start a fresh context when the window is shown again.
            self.end_second_ime_viewport();
        }
        // Tab, Shift+Tab and Presentation hide the floating panels with the docked ones.
        if !presenting && self.ui.hidden_panels == 0 {
            dock::float_only_windows(self, &ctx);
        }
        story_editor::show(self, &ctx);
        dialogs::show(self, &ctx);
        about::show(self, &ctx);
        updates_ui::show(self, &ctx);
        menus::palette(self, &ctx);
        // The IME is observed once, here: after the canvas, the floating panels, the story editor,
        // dialogs and Quick Apply have all applied this pass's commands and focus changes.
        canvas::ime_observe(self, &ctx);
        if self.custom_titlebar {
            chrome::resize_borders(self, ui);
        }
        for url in std::mem::take(&mut self.ui.pending_urls) {
            ctx.open_url(egui::OpenUrl::new_tab(url));
        }
        if !ctx.will_discard() {
            // A settled UI pass has taken the injected step (a discarded pass has not). Keep
            // `injected_private` until the next raw-input hook, so this pass's end-pass clipboard
            // guard still sees the control provenance.
            self.synthetic_injected = false;
            self.injected_events.clear();
            self.injected_pending.store(false, std::sync::atomic::Ordering::Relaxed);
            self.injected_settled.store(false, std::sync::atomic::Ordering::Relaxed);
            self.ui_frame_end(&ctx);
        }
        self.perf.frame_ms = now_ms() - t0;
    }

    /// Defer control commands while host-order input is pending; otherwise handle replies and screenshot admission.
    fn drain_control(&mut self, ctx: &egui::Context) {
        // An injected batch may still be waiting in the host's pending raw input. Running another
        // control command now could switch documents or dialogs under it, so keep the channel
        // queued until a settled UI pass has taken the batch.
        if self.host_order_pending() {
            return;
        }
        let Some(rx) = self.control_rx.take() else { return };
        while !self.host_order_pending() {
            let Ok(req) = rx.try_recv() else { break };
            let reply = req.reply.clone();
            match control::handle(self, ctx, &req) {
                control::Outcome::Done(v) => {
                    let _ = reply.send(v);
                }
                control::Outcome::Screenshot { path } => {
                    if let Err(e) = self.admit_screenshot(path, reply.clone()) {
                        let _ = reply.send(e);
                    }
                }
            }
        }
        self.control_rx = Some(rx);
    }

    /// Admit one `ui.screenshot` request against the app's count and byte budgets.
    ///
    /// Admission happens before any capture is issued: the checked, current viewport dimensions
    /// reserve their RGBA bytes, and queued plus completed images are counted together.
    pub(crate) fn admit_screenshot(&mut self, path: Option<String>, reply: Sender<ControlResponse>) -> Result<u64, serde_json::Value> {
        let Some(mut geometry) = self.screenshot_geometry else {
            return Err(json!({"ok": false, "error": "the window geometry is not known yet"}));
        };
        // The cached geometry may predate a document switch handled earlier in this same logic
        // tick. Admit for the current document, not for the one the cache last saw.
        geometry.document = self.active_document_epoch();
        if self.pending_shots.len() >= SCREENSHOT_MAX_REQUESTS {
            return Err(json!({"ok": false, "error": format!("at most {SCREENSHOT_MAX_REQUESTS} screenshot requests may be outstanding")}));
        }
        let reserved_bytes = geometry
            .width
            .checked_mul(geometry.height)
            .and_then(|pixels| pixels.checked_mul(4))
            .ok_or_else(|| json!({"ok": false, "error": "the screenshot dimensions overflow the addressable size"}))?;
        if reserved_bytes > SCREENSHOT_MAX_IMAGE_BYTES {
            return Err(json!({
                "ok": false,
                "error": format!(
                    "a screenshot of {}×{} exceeds the {SCREENSHOT_MAX_IMAGE_BYTES}-byte image limit",
                    geometry.width, geometry.height
                )
            }));
        }
        let used = self.pending_shots.iter().try_fold(0usize, |total, shot| total.checked_add(shot.reserved_bytes));
        let over_budget = used.and_then(|used| used.checked_add(reserved_bytes)).is_none_or(|total| total > SCREENSHOT_MAX_AGGREGATE_BYTES);
        if over_budget {
            return Err(json!({"ok": false, "error": format!("the {SCREENSHOT_MAX_AGGREGATE_BYTES}-byte screenshot budget is exhausted")}));
        }
        self.shot_token += 1;
        let token = self.shot_token;
        let now = now_ms();
        self.queued_shots.push((token, now + 120.0, 0));
        self.pending_shots.push(ShotRequest {
            token,
            path,
            reply,
            deadline: now + 8000.0,
            viewport: egui::ViewportId::ROOT,
            epoch: self.screenshot_epoch,
            geometry,
            reserved_bytes,
            slot: None,
        });
        Ok(token)
    }

    /// The current document state (`0` when none is open). Revert preserves the UID but
    /// gives the replacement state a new epoch, invalidating old captures and input.
    fn active_document_epoch(&self) -> u64 {
        self.session.active().map_or(0, |document| document.identity_epoch)
    }

    /// Reject one admitted request: remove its admission and queued token and cancel any live
    /// capture slot before replying with the error.
    fn reject_screenshot(&mut self, index: usize, message: &str) {
        let shot = self.pending_shots.remove(index);
        self.queued_shots.retain(|(token, ..)| *token != shot.token);
        if let Some(slot) = &shot.slot {
            slot.cancel();
        }
        let _ = shot.reply.send(json!({"ok": false, "error": message}));
    }

    /// Settle and issue queued captures. Called only at the end of a settled `run_ui` pass, so
    /// `frames` counts actual layout passes; hidden logic ticks never satisfy the three-frame
    /// settling policy on their own.
    fn issue_screenshots(&mut self, ctx: &egui::Context) {
        let now = now_ms();
        let mut due = Vec::new();
        self.queued_shots.retain_mut(|(token, at, frames)| {
            *frames += 1;
            if now >= *at && *frames >= 3 {
                due.push(*token);
                false
            } else {
                true
            }
        });
        for token in due {
            let Some(index) = self.pending_shots.iter().position(|shot| shot.token == token) else { continue };
            if self.pending_shots[index].geometry.document != self.active_document_epoch() {
                self.reject_screenshot(index, "the active document changed before the screenshot was issued");
                continue;
            }
            let Some(current) = self.screenshot_geometry else {
                let shot = self.pending_shots.remove(index);
                let _ = shot.reply.send(json!({"ok": false, "error": "the window geometry is not known yet"}));
                continue;
            };
            // Re-check the current dimensions before issuing: a request admitted before a resize,
            // DPI or zoom change must renew its checked reservation (or fail, never overrun it).
            if self.pending_shots[index].epoch != self.screenshot_epoch {
                let bytes = current.width.checked_mul(current.height).and_then(|pixels| pixels.checked_mul(4));
                let others = self
                    .pending_shots
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| *i != index)
                    .try_fold(0usize, |total, (_, shot)| total.checked_add(shot.reserved_bytes));
                let reserved = bytes.zip(others).and_then(|(bytes, others)| others.checked_add(bytes).map(|total| (bytes, total)));
                let fits = reserved.is_some_and(|(bytes, total)| bytes <= SCREENSHOT_MAX_IMAGE_BYTES && total <= SCREENSHOT_MAX_AGGREGATE_BYTES);
                let Some((bytes, _)) = reserved else {
                    let shot = self.pending_shots.remove(index);
                    let _ = shot.reply.send(json!({
                        "ok": false,
                        "error": "the window changed and the screenshot no longer fits the admitted budget"
                    }));
                    continue;
                };
                if !fits {
                    let shot = self.pending_shots.remove(index);
                    let _ = shot.reply.send(json!({
                        "ok": false,
                        "error": "the window changed and the screenshot no longer fits the admitted budget"
                    }));
                    continue;
                }
                let shot = &mut self.pending_shots[index];
                shot.epoch = self.screenshot_epoch;
                shot.geometry = current;
                shot.reserved_bytes = bytes;
            }
            let shot = &mut self.pending_shots[index];
            let (slot, callback) = craft_ui::capture::CaptureSlot::new(ctx, shot.viewport, SCREENSHOT_MAX_IMAGE_BYTES);
            shot.slot = Some(slot);
            // The callback holds only a weak slot: it never captures the app, writes a file, runs a
            // command or wakes the context itself (the slot's live request does that).
            ctx.send_viewport_cmd_to(shot.viewport, egui::ViewportCommand::Screenshot(callback));
        }
        if !self.queued_shots.is_empty() || !self.pending_shots.is_empty() {
            ctx.request_repaint_after(std::time::Duration::from_millis(16));
        }
        // The screenshot command itself forces one hidden paint; the settling frames before it do
        // not, so ask for them while the window may be hidden. The 0.37 web backend does not yet
        // honor this command (fork follow-up): hidden-tab captures time out there rather than
        // pretending to settle.
        if !self.queued_shots.is_empty() && ctx.input(|i| i.viewport().visible()) != Some(true) {
            #[cfg(not(target_arch = "wasm32"))]
            ctx.send_viewport_cmd(egui::ViewportCommand::RequestPaintWhileHidden);
        }
    }

    /// Take completed captures out of their slots and write files only on this (UI) thread.
    fn collect_screenshots(&mut self, ctx: &egui::Context) {
        if self.pending_shots.is_empty() {
            return;
        }
        let now = now_ms();
        let current_document = self.active_document_epoch();
        let mut i = 0;
        while i < self.pending_shots.len() {
            // Check before taking the callback: a request whose document changed must not write,
            // retry, or wait for its slot, even when no frame has completed yet.
            if self.pending_shots.get(i).is_some_and(|shot| shot.geometry.document != current_document) {
                self.reject_screenshot(i, "the active document changed while the screenshot was pending");
                continue;
            }
            let taken = self.pending_shots.get(i).and_then(|shot| shot.slot.as_ref()).and_then(|slot| slot.take());
            let Some(result) = taken else {
                if now >= self.pending_shots[i].deadline {
                    let shot = self.pending_shots.remove(i);
                    self.queued_shots.retain(|(token, ..)| *token != shot.token);
                    if let Some(slot) = &shot.slot {
                        slot.cancel();
                    }
                    let _ = shot.reply.send(json!({"ok": false, "error": "no frame was presented (screen locked or window hidden); use ui.render"}));
                } else {
                    i += 1;
                }
                continue;
            };
            let (epoch_changed, within_reserved) = {
                let shot = &self.pending_shots[i];
                let within = match &result {
                    Ok(image) => {
                        let [width, height] = image.size;
                        width <= shot.geometry.width && height <= shot.geometry.height
                    }
                    Err(_) => true,
                };
                (shot.epoch != self.screenshot_epoch, within)
            };
            if !epoch_changed && within_reserved {
                let shot = self.pending_shots.remove(i);
                self.queued_shots.retain(|(token, ..)| *token != shot.token);
                let value = match result {
                    Ok(image) => control::save_screenshot(self, image.as_ref(), shot.path.as_deref()),
                    Err(e) => json!({"ok": false, "error": e.to_string()}),
                };
                let _ = shot.reply.send(value);
                continue;
            }
            let (retry, retry_token) = {
                let shot = &self.pending_shots[i];
                (now < shot.deadline, shot.token)
            };
            if let Some(slot) = self.pending_shots[i].slot.take() {
                slot.cancel();
            }
            if !retry {
                let shot = self.pending_shots.remove(i);
                self.queued_shots.retain(|(token, ..)| *token != shot.token);
                let message = match result {
                    Err(e) => e.to_string(),
                    Ok(_) if epoch_changed => "the window changed while the screenshot was captured".to_string(),
                    Ok(_) => "the screenshot is larger than the admitted viewport".to_string(),
                };
                let _ = shot.reply.send(json!({"ok": false, "error": message}));
                continue;
            }
            // Reject the stale pixels and retry within the same deadline with a fresh epoch.
            self.queued_shots.push((retry_token, now + 120.0, 0));
            i += 1;
        }
        if !self.queued_shots.is_empty() || !self.pending_shots.is_empty() {
            ctx.request_repaint_after(std::time::Duration::from_millis(16));
        }
    }
}

/// The physical capture surface of the root viewport, from its logical rect and DPI.
fn screenshot_geometry(ctx: &egui::Context) -> Option<ScreenshotGeometry> {
    let zoom = ctx.zoom_factor();
    let fallback = ctx.pixels_per_point();
    let (rect, native, fullscreen) = ctx.input(|i| {
        let viewport = i.viewport();
        // Web (eframe's web backend) and egui_kittest set `screen_rect` but not `inner_rect`;
        // both describe the same logical content rect for this viewport.
        (viewport.inner_rect.or(i.raw.screen_rect), viewport.native_pixels_per_point, viewport.fullscreen)
    });
    let rect = rect?;
    let pixels_per_point = match native {
        // `native_pixels_per_point` is before egui's zoom
        // (`Context::pixels_per_point() == zoom_factor * native_pixels_per_point`), so apply the
        // zoom exactly once. The fallback already includes it.
        Some(native) => native * zoom,
        None => fallback,
    };
    if !pixels_per_point.is_finite() || pixels_per_point <= 0.0 {
        return None;
    }
    let width = (rect.width() * pixels_per_point).ceil();
    let height = (rect.height() * pixels_per_point).ceil();
    if !width.is_finite() || !height.is_finite() || width < 1.0 || height < 1.0 {
        return None;
    }
    Some(ScreenshotGeometry {
        width: width as usize,
        height: height as usize,
        document: 0,
        signature: [
            rect.width().to_bits(),
            rect.height().to_bits(),
            pixels_per_point.to_bits(),
            u32::from(fullscreen.unwrap_or(false)),
            zoom.to_bits(),
            // Reserved for a surface-generation counter once a backend exposes one.
            0,
        ],
    })
}

/// Files that open as documents (when dropped or picked) rather than being placed: DesignCraft,
/// IDML, and InDesign documents and templates (converted to IDML on open).
pub fn opens_as_document(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    [".designcraft", ".idml", ".indd", ".indt"].iter().any(|ext| n.ends_with(ext))
}

#[cfg(test)]
mod import_warning_tests {
    use super::*;
    use std::io::Write;

    fn idml(missing_parts: bool) -> Vec<u8> {
        let missing = if missing_parts { r#"<idPkg:Spread src="Spreads/missing.xml"/><idPkg:Story src="Stories/missing.xml"/>"# } else { "" };
        let map = format!(
            r#"<Document xmlns:idPkg="http://ns.adobe.com/AdobeInDesign/idml/1.0/packaging" Self="d"><idPkg:Spread src="Spreads/one.xml"/>{missing}</Document>"#
        );
        let spread = r#"<idPkg:Spread xmlns:idPkg="http://ns.adobe.com/AdobeInDesign/idml/1.0/packaging"><Spread Self="s"><Page Self="p" GeometricBounds="0 0 100 100" ItemTransform="1 0 0 1 0 0"/><Rectangle Self="r" ItemTransform="1 0 0 1 0 0"/></Spread></idPkg:Spread>"#;
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        let options = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
        for (name, text) in [("mimetype", designcraft_idml::MIMETYPE), ("designmap.xml", &map), ("Spreads/one.xml", spread)] {
            zip.start_file(name, options).unwrap();
            zip.write_all(text.as_bytes()).unwrap();
        }
        zip.finish().unwrap().into_inner()
    }

    fn app() -> DesignApp {
        let mut app = DesignApp::new(Session::new(), Services { inbox: Some(Inbox::default()), ..Default::default() });
        app.run("file.new", json!({"title": "Existing work"})).unwrap();
        app.run("frame.create", json!({"rect": [10, 10, 50, 50]})).unwrap();
        app
    }

    fn assert_partial_import_warning(app: &mut DesignApp, file: &str) {
        let dialog = app.ui.dialog.as_ref().expect("partial import must show an alert");
        assert_eq!(dialog.id, "alert");
        assert_eq!(dialog.fields["title"], "Import Warnings");
        assert_eq!(dialog.fields["file"], file);
        for part in ["Spreads/missing.xml", "Stories/missing.xml"] {
            assert!(dialog.fields["message"].as_str().unwrap().contains(part), "every missing part is named");
            assert!(app.ui.status.contains(part), "the status line retains the warning");
        }
        assert_eq!(app.session.documents().len(), 2);
        assert_eq!(app.session.documents()[0].doc.title, "Existing work");
        assert!(app.session.documents()[0].is_dirty(), "existing unsaved work stays intact");
        assert_eq!(app.session.documents()[0].doc.spreads[0].items.len(), 1);
        assert_eq!(app.session.doc().unwrap().doc.spreads.len(), 1);
        assert_eq!(app.session.doc().unwrap().doc.spreads[0].items.len(), 1, "available document content is kept");
        dialogs::confirm(app).unwrap();
        assert!(app.ui.dialog.is_none(), "OK dismisses the warning");
        assert_eq!(app.session.documents().len(), 2, "dismissing the warning keeps the imported document");
    }

    #[test]
    fn byte_open_reports_partial_import_warnings_without_losing_documents() {
        let mut app = app();
        let result =
            app.open_file("file.openBytes", json!({"name": "partial.idml", "base64": designcraft_engine::cmd::base64_encode(&idml(true))})).unwrap();
        assert_eq!(result["warnings"].as_array().unwrap().len(), 2);
        assert_partial_import_warning(&mut app, "partial.idml");
    }

    #[test]
    fn native_open_reports_partial_import_warnings_without_losing_documents() {
        let path = std::env::temp_dir().join(format!("designcraft-partial-import-{}", std::process::id())).join("partial.idml");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, idml(true)).unwrap();
        let picked = path.to_string_lossy().to_string();
        let mut app = app();
        app.services.pick_open = Some(Box::new(move |_| Some(picked.clone())));
        let result = app.run("app.openDialog", json!({}));
        std::fs::remove_file(&path).unwrap();
        std::fs::remove_dir(path.parent().unwrap()).unwrap();
        assert_eq!(result.unwrap()["warnings"].as_array().unwrap().len(), 2);
        assert_partial_import_warning(&mut app, "partial.idml");
    }

    #[test]
    fn inbox_open_reports_partial_import_warnings_without_losing_documents() {
        let mut app = app();
        let request = app.import_request("open");
        app.services.inbox.as_ref().unwrap().lock().unwrap().push(ImportedFile { request, name: "partial.idml".into(), bytes: idml(true) });
        app.drain_inbox();
        assert_partial_import_warning(&mut app, "partial");
    }

    #[test]
    fn find_font_on_open_outranks_the_warning_alert() {
        let mut app = app();
        app.run("frame.create", json!({"rect": [0, 0, 200, 200], "content": "text", "text": "Sample"})).unwrap();
        let d = std::sync::Arc::make_mut(&mut app.session.doc_mut().unwrap().doc);
        let sid = *d.stories.keys().next().unwrap();
        let story = d.story_mut(sid).unwrap();
        story.format_chars(0..6, |f| f.over.font_family = Some("Unavailable Test Family".into()));
        story.paras[0].para.grep_styles = Some(vec![designcraft_doc::GrepStyle { style: "Keep".into(), pattern: "[a-".into() }]);
        let path = std::env::temp_dir().join(format!("designcraft-font-warning-{}.designcraft", std::process::id()));
        app.run("file.save", json!({"path": path})).unwrap();
        let result = app.open_file("file.open", json!({"path": path}));
        std::fs::remove_file(&path).unwrap();
        assert_eq!(result.unwrap()["warnings"].as_array().unwrap().len(), 1);
        assert_eq!(app.ui.dialog.as_ref().unwrap().id, "findFont");
        assert!(app.ui.status.contains("GREP style not applied"), "{}", app.ui.status);
    }

    #[test]
    fn complete_import_does_not_show_a_warning() {
        let mut app = app();
        let result = app
            .open_file("file.openBytes", json!({"name": "complete.idml", "base64": designcraft_engine::cmd::base64_encode(&idml(false))}))
            .unwrap();
        assert_eq!(result["warnings"], json!([]));
        assert!(app.ui.dialog.is_none());
        assert!(app.ui.status.is_empty());
        assert_eq!(app.session.documents().len(), 2);
    }
}

#[cfg(test)]
pub(crate) mod browser_import_tests {
    use super::*;

    fn app() -> DesignApp {
        let mut app = DesignApp::new(Session::new(), Services { inbox: Some(Inbox::default()), ..Default::default() });
        app.run("file.new", json!({"title": "A"})).unwrap();
        app
    }
    fn deliver(app: &mut DesignApp, request: ImportRequest, name: &str, bytes: Vec<u8>) {
        app.services.inbox.as_ref().unwrap().lock().unwrap().push(ImportedFile { request, name: name.into(), bytes });
        app.drain_inbox();
    }
    fn image() -> Vec<u8> {
        br##"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><rect width="10" height="10" fill="#ff0000"/></svg>"##.to_vec()
    }
    fn pdf_pages(count: usize) -> Vec<u8> {
        let kids = (0..count).map(|i| format!("{} 0 R", 3 + i * 2)).collect::<Vec<_>>().join(" ");
        let mut objects = vec!["<< /Type /Catalog /Pages 2 0 R >>".to_string(), format!("<< /Type /Pages /Kids [{kids}] /Count {count} >>")];
        for (i, color) in ["1 0 0", "0 1 0", "0 0 1"].iter().take(count).enumerate() {
            let content = format!("{color} rg 0 0 200 200 re f\n");
            objects.push(format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << >> /Contents {} 0 R >>", 4 + i * 2));
            objects.push(format!("<< /Length {} >>\nstream\n{content}endstream", content.len()));
        }
        let mut result = "%PDF-1.4\n".to_string();
        let mut offsets = Vec::new();
        for (i, object) in objects.iter().enumerate() {
            offsets.push(result.len());
            result.push_str(&format!("{} 0 obj\n{object}\nendobj\n", i + 1));
        }
        let xref = result.len();
        result.push_str(&format!("xref\n0 {}\n0000000000 65535 f \n", offsets.len() + 1));
        for offset in offsets {
            result.push_str(&format!("{offset:010} 00000 n \n"));
        }
        result.push_str(&format!("trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n", objects.len() + 1));
        result.into_bytes()
    }
    pub(crate) fn pdf() -> Vec<u8> {
        pdf_pages(3)
    }

    #[test]
    fn picker_receives_context_before_returning_and_cancel_does_not_edit() {
        let captured = std::sync::Arc::new(std::sync::Mutex::new(None));
        let copy = captured.clone();
        let mut app = app();
        app.services.open_async = Some(Box::new(move |request| *copy.lock().unwrap() = Some(request)));
        let uid = app.session.doc().unwrap().uid;
        app.pick_and_open("place").unwrap();
        app.run("file.new", json!({"title": "B"})).unwrap();
        let request = captured.lock().unwrap().take().unwrap();
        assert_eq!(request.purpose, "place");
        assert_eq!(request.target, Some(uid));
        assert!(app.session.documents().iter().all(|d| d.doc.assets.is_empty()));
        deliver(&mut app, request, "broken.pdf", b"not a PDF".to_vec());
        assert!(app.session.documents().iter().all(|d| d.doc.assets.is_empty()));
        assert!(app.pending_pdf.is_none());
    }

    #[test]
    fn delayed_place_activates_original_document_and_undo_belongs_to_it() {
        let mut app = app();
        let request = app.import_request("place");
        app.run("file.new", json!({"title": "B"})).unwrap();
        deliver(&mut app, request, "red.svg", image());
        assert_eq!(app.session.active_index(), Some(0));
        assert_eq!(app.session.documents()[0].doc.assets.len(), 1);
        assert!(app.session.documents()[1].doc.assets.is_empty());
        app.run("edit.undo", json!({})).unwrap();
        assert!(app.session.doc().unwrap().doc.assets.is_empty());
        app.run("edit.redo", json!({})).unwrap();
        assert_eq!(app.session.doc().unwrap().doc.assets.len(), 1);
    }

    #[test]
    fn closed_target_is_not_replaced_by_a_reused_tab_index() {
        let mut app = app();
        let request = app.import_request("place");
        app.run("file.close", json!({})).unwrap();
        app.run("file.new", json!({"title": "replacement"})).unwrap();
        deliver(&mut app, request, "red.svg", image());
        assert!(app.session.doc().unwrap().doc.assets.is_empty());
    }

    #[test]
    fn closing_earlier_tab_does_not_shift_import_target() {
        let mut app = app();
        app.run("file.new", json!({"title": "target"})).unwrap();
        let request = app.import_request("place");
        app.run("file.new", json!({"title": "other"})).unwrap();
        app.run("file.close", json!({"index": 0})).unwrap();
        deliver(&mut app, request, "red.svg", image());
        assert_eq!(app.session.doc().unwrap().doc.title, "target");
        assert_eq!(app.session.documents()[0].doc.assets.len(), 1);
        assert!(app.session.documents()[1].doc.assets.is_empty());
    }

    #[test]
    fn layout_place_and_open_preserve_explicit_intent() {
        let mut source = Session::new();
        source.execute("file.new", &json!({"title": "source"})).unwrap();
        source.execute("frame.create", &json!({"rect": [10, 10, 50, 50], "content": "text", "text": "placed"})).unwrap();
        let native = designcraft_engine::cmd::to_bytes(&source.doc().unwrap().doc).unwrap();
        let idml = designcraft_idml::export_idml(&source.doc().unwrap().doc);
        for (name, bytes) in [("source.designcraft", native), ("source.idml", idml)] {
            let mut app = app();
            let request = app.import_request("place");
            deliver(&mut app, request, name, bytes.clone());
            assert_eq!(app.session.documents().len(), 1);
            assert_eq!(app.session.doc().unwrap().doc.title, "A");
            assert!(app.session.doc().unwrap().doc.stories.values().any(|s| s.text == "placed"));
            app.run("edit.undo", json!({})).unwrap();
            assert!(app.session.doc().unwrap().doc.stories.is_empty());
            app.run("edit.redo", json!({})).unwrap();
            assert!(!app.session.doc().unwrap().doc.stories.is_empty());
            let request = app.import_request("open");
            deliver(&mut app, request, name, bytes);
            assert_eq!(app.session.documents().len(), 2);
        }
    }

    #[test]
    fn pdf_options_keep_bytes_target_and_selected_page() {
        let mut app = app();
        let bytes = pdf();
        let request = app.import_request("place");
        deliver(&mut app, request, "three.pdf", bytes.clone());
        assert!(app.session.doc().unwrap().doc.assets.is_empty());
        assert_eq!(app.ui.dialog.as_ref().unwrap().id, "pdfImport");
        app.run("file.new", json!({"title": "B"})).unwrap();
        app.ui.dialog.as_mut().unwrap().fields.insert("page".into(), json!("2"));
        dialogs::confirm(&mut app).unwrap();
        assert_eq!(app.session.active_index(), Some(0));
        let asset = app.session.doc().unwrap().doc.assets.values().next().unwrap();
        assert_eq!(asset.page, 1);
        assert_eq!(*asset.data, bytes);
        assert!(app.pending_pdf.is_none());
        assert!(app.session.documents()[1].doc.assets.is_empty());
        app.run("place.drop", json!({"x": 36, "y": 36})).unwrap();
        let exported = app.run("file.exportPdf", json!({})).unwrap();
        let exported = designcraft_engine::cmd::base64_decode(exported["base64"].as_str().unwrap());
        let pixels = designcraft_render::render_pdf_page(&exported, 0, 400).unwrap();
        assert!(pixels.data().iter().filter(|p| p.g > 240 && p.r < 10 && p.b < 10).count() > 1000);
        assert_eq!(pixels.data().iter().filter(|p| p.r > 240 && p.g < 10 && p.b < 10).count(), 0);
    }

    #[test]
    fn pdf_options_refuse_a_page_range_instead_of_placing_page_one() {
        for spec in ["4-5", "2,5", "2-3", "2.5", "-3", "0", "nan", "inf"] {
            let mut app = app();
            let request = app.import_request("place");
            deliver(&mut app, request, "three.pdf", pdf());
            app.ui.dialog.as_mut().unwrap().fields.insert("page".into(), json!(spec));
            assert!(dialogs::confirm(&mut app).is_err(), "{spec}");
            assert!(app.session.doc().unwrap().doc.assets.is_empty(), "{spec}");
            assert_eq!(app.ui.dialog.as_ref().unwrap().id, "pdfImport", "{spec}");
        }
    }

    #[test]
    fn place_loads_the_cursor_unless_a_frame_can_take_the_graphic() {
        let mut app = app();
        let items = |app: &DesignApp| app.session.doc().unwrap().doc.spreads[0].items.len();
        // Nothing selected: the graphic waits in the place cursor; Escape unloads it.
        let request = app.import_request("place");
        deliver(&mut app, request, "red.svg", image());
        assert_eq!(app.session.tool_id(), "placeGun");
        assert!(app.session.loaded.is_some());
        assert_eq!(items(&app), 0);
        let mods = designcraft_tools::Mods::default();
        app.session.tool_key(designcraft_tools::ToolKey::Escape, mods, designcraft_engine::ViewInfo::at_zoom(1.0)).unwrap();
        assert_eq!(app.session.tool_id(), "selection");
        assert!(app.session.loaded.is_none());
        // A click with the loaded cursor places the graphic at its natural size.
        let request = app.import_request("place");
        deliver(&mut app, request, "red.svg", image());
        app.run("place.drop", json!({"x": 100, "y": 100})).unwrap();
        let it = app.session.doc().unwrap().doc.spreads[0].items.last().unwrap();
        assert_eq!(it.bounds(), designcraft_geom::Rect::new(100.0, 100.0, 107.5, 107.5), "10 CSS px = 7.5 pt");
        // An empty graphic frame selected: the graphic fills it.
        let f = app.run("frame.create", json!({"rect": [200, 200, 300, 300], "content": "graphic"})).unwrap()["id"].as_u64().unwrap();
        app.run("selection.set", json!({"ids": [f]})).unwrap();
        let request = app.import_request("place");
        deliver(&mut app, request, "red.svg", image());
        assert_eq!(app.session.tool_id(), "selection");
        assert!(app.session.loaded.is_none());
        assert_eq!(items(&app), 2);
        let frame = app.session.doc().unwrap().doc.item(designcraft_doc::ItemId(f)).unwrap();
        assert!(matches!(frame.content, designcraft_doc::Content::Graphic(_)));
    }

    #[test]
    fn pdf_options_reject_oversized_page_without_editing() {
        let mut app = app();
        for page in [0_u64, 4, 4_294_967_296, 4_294_967_297, u64::MAX] {
            let request = app.import_request("place");
            deliver(&mut app, request, "three.pdf", pdf());
            app.ui.dialog.as_mut().unwrap().fields.insert("page".into(), json!(page.to_string()));
            // The native dialog clamps zero to one; positive invalid pages must fail.
            if page == 0 {
                assert!(app.confirm_pdf_import(page, "crop".into()).is_err());
            } else {
                assert!(dialogs::confirm(&mut app).is_err());
            }
            app.ui.dialog = None;
            assert!(app.session.doc().unwrap().doc.assets.is_empty());
            assert!(app.pending_pdf.is_none());
        }
        let request = app.import_request("place");
        deliver(&mut app, request, "three.pdf", pdf());
        app.ui.dialog.as_mut().unwrap().fields.insert("page".into(), json!("2"));
        dialogs::confirm(&mut app).unwrap();
        assert_eq!(app.session.doc().unwrap().doc.assets.values().next().unwrap().page, 1);
    }

    #[test]
    fn drop_routing_and_nonlayout_open_remain_compatible() {
        let mut app = app();
        let native = designcraft_engine::cmd::to_bytes(&app.session.doc().unwrap().doc).unwrap();
        let request = app.import_request("drop");
        deliver(&mut app, request, "document.designcraft", native);
        assert_eq!(app.session.documents().len(), 2);
        let request = app.import_request("drop");
        app.run("file.activate", json!({"index": 0})).unwrap();
        deliver(&mut app, request, "red.svg", image());
        assert_eq!(app.session.active_index(), Some(1));
        assert_eq!(app.session.doc().unwrap().doc.assets.len(), 1);
        let request = app.import_request("open");
        deliver(&mut app, request, "single.pdf", pdf_pages(1));
        assert_eq!(app.session.documents().len(), 2);
        assert!(app.pending_pdf.is_none());
        assert!(app.ui.dialog.is_none());
    }

    #[test]
    fn ase_swatches_keep_purpose_and_target() {
        let mut source = Session::new();
        source.execute("file.new", &json!({})).unwrap();
        source.execute("swatch.create", &json!({"name": "Import Teal", "color": "#108080"})).unwrap();
        let data = source.execute("swatch.save", &json!({"names": ["Import Teal"]})).unwrap();
        let bytes = designcraft_engine::cmd::base64_decode(data["base64"].as_str().unwrap());
        for (purpose, name) in [("swatches", "picker-file"), ("drop", "palette.ase")] {
            let mut app = app();
            let request = app.import_request(purpose);
            app.run("file.new", json!({"title": "B"})).unwrap();
            deliver(&mut app, request, name, bytes.clone());
            assert_eq!(app.session.active_index(), Some(0));
            assert!(app.session.documents()[0].doc.swatches.iter().any(|s| s.name == "Import Teal"));
            assert!(!app.session.documents()[1].doc.swatches.iter().any(|s| s.name == "Import Teal"));
        }
    }

    #[test]
    fn native_pdf_dialog_keeps_path_and_uid_at_confirmation() {
        let mut app = app();
        let path = std::env::temp_dir().join(format!("designcraft-native-import-{}-{}.pdf", std::process::id(), app.session.doc().unwrap().uid));
        std::fs::write(&path, pdf()).unwrap();
        let pick_path = path.to_string_lossy().to_string();
        app.services.pick_open = Some(Box::new(move |_| Some(pick_path.clone())));
        app.services.read = Some(Box::new(|path| std::fs::read(path).map_err(|e| e.to_string())));
        app.pick_and_open("place").unwrap();
        assert_eq!(app.ui.dialog.as_ref().unwrap().id, "pdfImport");
        app.run("file.new", json!({"title": "B"})).unwrap();
        app.ui.dialog.as_mut().unwrap().fields.insert("page".into(), json!("2"));
        dialogs::confirm(&mut app).unwrap();
        assert_eq!(app.session.active_index(), Some(0));
        assert_eq!(app.session.doc().unwrap().doc.assets.values().next().unwrap().page, 1);
        assert!(app.session.documents()[1].doc.assets.is_empty());
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn pending_pdf_cancel_close_supersession_and_second_delivery_release_bytes() {
        let mut app = app();
        let request = app.import_request("place");
        deliver(&mut app, request.clone(), "first.pdf", pdf());
        deliver(&mut app, request.clone(), "second.svg", image());
        assert_eq!(app.pending_pdf.as_ref().unwrap().name, "first.pdf");
        assert!(app.session.doc().unwrap().doc.assets.is_empty());
        app.ui.dialog = None;
        app.drain_inbox();
        assert!(app.pending_pdf.is_none());
        deliver(&mut app, request.clone(), "first.pdf", pdf());
        app.ui.dialog = Some(dialogs::Dialog::new("newDocument", json!({})));
        app.drain_inbox();
        assert!(app.pending_pdf.is_none());
        app.ui.dialog = None;
        deliver(&mut app, request, "first.pdf", pdf());
        app.run("file.close", json!({})).unwrap();
        app.run("file.new", json!({})).unwrap();
        assert!(dialogs::confirm(&mut app).is_err());
        assert!(app.pending_pdf.is_none());
        assert!(app.session.doc().unwrap().doc.assets.is_empty());
    }
}

#[cfg(test)]
mod load_styles_tests {
    use super::*;

    /// A client's document on disk: Head based on Body, and a Lead character style.
    fn client_bytes() -> Vec<u8> {
        let mut s = Session::new();
        s.execute("file.new", &json!({})).unwrap();
        s.execute("style.paragraph.create", &json!({"name": "Body", "chars": {"size": 10}})).unwrap();
        s.execute("style.paragraph.create", &json!({"name": "Head", "basedOn": "Body", "chars": {"size": 20}})).unwrap();
        s.execute("style.character.create", &json!({"name": "Lead", "chars": {"size": 9}})).unwrap();
        designcraft_engine::cmd::base64_decode(s.execute("file.serialize", &json!({})).unwrap()["base64"].as_str().unwrap())
    }

    /// An app with a document that has its own 8 pt Body, and a picker that answers with the client's file.
    fn app_and_file(test: &str) -> (DesignApp, std::path::PathBuf) {
        let path = std::env::temp_dir().join(format!("designcraft-load-styles-{}-{test}.designcraft", std::process::id()));
        std::fs::write(&path, client_bytes()).unwrap();
        let picked = path.to_string_lossy().to_string();
        let services = Services { pick_open: Some(Box::new(move |purpose| (purpose == "styles").then(|| picked.clone()))), ..Default::default() };
        let mut app = DesignApp::new(Session::new(), services);
        app.run("file.new", json!({})).unwrap();
        app.run("style.paragraph.create", json!({"name": "Body", "chars": {"size": 8}})).unwrap();
        (app, path)
    }

    fn names(app: &DesignApp) -> (Vec<String>, Vec<String>) {
        let st = &app.session.doc().unwrap().doc.styles;
        (st.paragraph.iter().skip(2).map(|s| s.name.clone()).collect(), st.character.iter().skip(1).map(|s| s.name.clone()).collect())
    }

    #[test]
    fn load_all_text_styles_asks_which_styles_of_the_picked_file_to_load() {
        let (mut app, path) = app_and_file("all");
        app.run("app.loadTextStyles", json!({})).unwrap();
        let d = app.ui.dialog.as_mut().expect("the Load Styles dialog");
        let row = |name: &str, conflict: bool| json!({"name": name, "conflict": conflict});
        assert_eq!(d.id, "loadStyles");
        assert_eq!(d.fields["paragraph"], json!([row("[Basic Paragraph]", false), row("Body", true), row("Head", false)]));
        assert_eq!(d.fields["character"], json!([row("Lead", false)]));
        // Lead is unchecked; the rest loads, and Body takes the file's definition.
        d.fields["character"][0]["on"] = json!(false);
        dialogs::confirm(&mut app).unwrap();
        std::fs::remove_file(&path).unwrap();
        assert!(app.ui.dialog.is_none());
        assert_eq!(names(&app), (vec!["Body".to_string(), "Head".to_string()], vec![]));
        assert_eq!(app.session.doc().unwrap().doc.styles.para("Body").unwrap().chars.size, Some(10.0));
    }

    #[test]
    fn the_dialog_applies_the_chosen_conflict_rule() {
        let (mut app, path) = app_and_file("rename");
        app.run("app.loadParagraphStyles", json!({})).unwrap();
        let d = app.ui.dialog.as_mut().unwrap();
        assert_eq!(d.fields["character"], json!([]), "Load Paragraph Styles offers no character styles");
        assert_eq!(d.fields["conflicts"], "redefine");
        d.fields.insert("conflicts".into(), json!("autoRename"));
        dialogs::confirm(&mut app).unwrap();
        std::fs::remove_file(&path).unwrap();
        assert_eq!(names(&app), (vec!["Body".to_string(), "Body 2".to_string(), "Head".to_string()], vec![]));
        assert_eq!(app.session.doc().unwrap().doc.styles.para("Body").unwrap().chars.size, Some(8.0));
    }

    #[test]
    fn load_character_styles_offers_only_character_styles() {
        let (mut app, path) = app_and_file("character");
        app.run("app.loadCharacterStyles", json!({})).unwrap();
        assert_eq!(app.ui.dialog.as_ref().unwrap().fields["paragraph"], json!([]));
        dialogs::confirm(&mut app).unwrap();
        std::fs::remove_file(&path).unwrap();
        assert_eq!(names(&app), (vec!["Body".to_string()], vec!["Lead".to_string()]));
    }

    /// A script can put anything into the dialog's rows: drawing and ticking them must not panic.
    #[test]
    fn the_dialog_draws_and_ticks_rows_that_are_no_style_rows() {
        use egui_kittest::kittest::Queryable as _;
        let mut h = test_window::open(DesignApp::new(Session::new(), Services::default()), egui::vec2(1200.0, 800.0));
        h.state_mut().app.ui.dialog = Some(dialogs::Dialog::new(
            "loadStyles",
            json!({"path": "x", "conflicts": 3, "paragraph": [1, "Body", null, {"name": 7}, {"name": "Head", "conflict": true}], "character": "none"}),
        ));
        h.run_steps(3);
        let button = h.get_by_label("Deselect All").rect().center();
        test_window::click_at(&mut h, button);
        let d = h.state().app.ui.dialog.as_ref().unwrap();
        assert_eq!(d.fields["paragraph"][4]["on"], false);
        assert_eq!(d.fields["paragraph"][0], 1, "what is no style row stays as it is");
        assert!(dialogs::confirm(&mut h.state_mut().app).is_err(), "there is no file `x`");
    }

    #[test]
    fn no_file_or_no_document_opens_no_dialog() {
        let (mut app, path) = app_and_file("none");
        std::fs::remove_file(&path).unwrap();
        // The picked file can't be read.
        assert!(app.run("app.loadTextStyles", json!({})).is_err());
        assert!(app.ui.dialog.as_ref().is_none_or(|d| d.id != "loadStyles"));
        // The picker is cancelled.
        app.ui.dialog = None;
        app.services.pick_open = Some(Box::new(|_| None));
        app.run("app.loadTextStyles", json!({})).unwrap();
        assert!(app.ui.dialog.is_none());
        // No document: the picker isn't shown at all.
        app.run("file.close", json!({})).unwrap();
        app.services.pick_open = Some(Box::new(|_| panic!("asked for a file without a document")));
        assert!(app.run("app.loadTextStyles", json!({})).is_err());
    }

    #[test]
    fn styles_from_an_asynchronous_picker_load_into_the_document_that_asked() {
        let asked = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let seen = asked.clone();
        let services = Services {
            inbox: Some(Inbox::default()),
            open_async: Some(Box::new(move |request| seen.lock().unwrap().push(request))),
            ..Default::default()
        };
        let mut app = DesignApp::new(Session::new(), services);
        app.run("file.new", json!({"title": "A"})).unwrap();
        app.run("app.loadCharacterStyles", json!({})).unwrap();
        let request = asked.lock().unwrap().pop().expect("the picker was asked");
        assert_eq!(request.purpose, "styles:character");
        // Another document is in front when the file arrives.
        app.run("file.new", json!({"title": "B"})).unwrap();
        app.services.inbox.as_ref().unwrap().lock().unwrap().push(ImportedFile { request, name: "client.designcraft".into(), bytes: client_bytes() });
        app.drain_inbox();
        assert_eq!(app.session.active_index(), Some(0));
        assert_eq!(names(&app), (vec![], vec!["Lead".to_string()]));
        assert!(app.session.documents()[1].doc.styles.char_style("Lead").is_none());
    }
}
