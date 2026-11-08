//! Application shell and controller.
//!
//! The controller owns the evening, the audio engine and the show machine. It
//! converts machine [`Effect`]s into audio/video actions, keeps autosave and
//! recovery current, drives the native menu and draws the editor workspace.
//! Page layouts live in [`crate::pages`].

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::time::{Duration, Instant};

use eframe::egui::{self, Color32, RichText, Stroke, TextureHandle};
use nerd_audio::{AudioEngine, AudioEvent, DecodedAudio, Levels};
use nerd_core::library::{Library, MediaAsset, MediaKind, AUDIO_EXTENSIONS, VIDEO_EXTENSIONS};
use nerd_core::model::{PlaybackMode, Project, ResumeMode, Song};
use nerd_core::persistence::{self, MissingMedia};
use nerd_core::prefs::{Preferences, UpdateChannel};
use nerd_core::{Effect, GoOutcome, Phase, Show, ShowConfig};

use crate::media::{self, ImportOutcome};
use crate::menu::MenuBar;
use crate::output::{self, ProjectorSong, ProjectorState};
use crate::paths::AppPaths;
use crate::platform::ShowAssertion;
use crate::state::{Checklist, Page, Snackbar, Toast, ToastKind, UndoStack};
use crate::theme::Palette;
use crate::update::{UpdateStatus, Updater};
use crate::video_player::VideoPlayer;
use nerd_ui_kit::SystemAppearance;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LibrarySort {
    Title,
    Duration,
    Recent,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct OrderDrag {
    pub from: usize,
    pub start_y: f32,
    pub current_y: f32,
}

struct BackgroundLoad {
    key: String,
    looping: bool,
    result: Result<DecodedAudio, String>,
}

#[derive(Debug, Clone)]
pub struct LaunchOptions {
    pub initial: Option<PathBuf>,
    pub demo: bool,
    pub gallery: bool,
    pub page: Option<Page>,
    pub appearance: Option<nerd_core::prefs::ThemeMode>,
    pub screenshot: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub(crate) struct ChromeState {
    pub library_sidebar: bool,
    pub library_filter: usize,
    pub inspector_open: bool,
    pub help: bool,
    pub timeline_list: bool,
    pub timeline_zoom: f32,
    pub timeline_snap: bool,
    pub titles_sidebar: bool,
    pub live_checklist: bool,
}

impl Default for ChromeState {
    fn default() -> Self {
        Self {
            library_sidebar: true,
            library_filter: 0,
            inspector_open: true,
            help: false,
            timeline_list: false,
            timeline_zoom: 1.0,
            timeline_snap: true,
            titles_sidebar: true,
            live_checklist: true,
        }
    }
}

pub struct NerdApp {
    pub(crate) paths: AppPaths,
    pub(crate) prefs: Preferences,
    pub(crate) project: Project,
    pub(crate) project_path: Option<PathBuf>,
    pub(crate) library: Library,
    pub(crate) engine: AudioEngine,
    pub(crate) show: Show,
    pub(crate) page: Page,
    pub(crate) palette: Palette,
    pub(crate) system: SystemAppearance,
    pub(crate) chrome: ChromeState,
    pub(crate) menu: MenuBar,
    pub(crate) snackbar: Option<Snackbar>,
    pub(crate) undo: UndoStack,
    pub(crate) dirty: bool,
    pub(crate) prefs_dirty: bool,
    pub(crate) selected_song: Option<String>,
    pub(crate) selected_asset: Option<String>,
    pub(crate) search: String,
    pub(crate) sort: LibrarySort,
    pub(crate) confirm_go: bool,
    pub(crate) confirm_quit: bool,
    pub(crate) quit_confirmed: bool,
    pub(crate) show_shortcuts: bool,
    pub(crate) about_open: bool,
    pub(crate) update_dialog: bool,
    pub(crate) recovery: Option<Project>,
    pub(crate) recovery_prompt: bool,
    pub(crate) missing: Vec<MissingMedia>,
    pub(crate) output_open: bool,
    pub(crate) checklist: Checklist,
    import_tx: Sender<ImportOutcome>,
    import_rx: Receiver<ImportOutcome>,
    pub(crate) imports_in_flight: usize,
    pub(crate) thumbnails: HashMap<String, TextureHandle>,
    thumbnail_failures: HashSet<String>,
    logo: Option<TextureHandle>,
    logo_for: Option<PathBuf>,
    logo_missing: bool,
    background_rx: Option<Receiver<BackgroundLoad>>,
    background_key: String,
    background_pending_start: bool,
    pub(crate) background_error: Option<String>,
    pub(crate) updater: Updater,
    startup_update_at: Option<Instant>,
    update_notified: bool,
    pub(crate) displays: Vec<nerd_video::DisplayInfo>,
    last_display_poll: Instant,
    pub(crate) levels: Levels,
    pub(crate) song_meter: f32,
    pub(crate) bg_meter: f32,
    last_tick: Instant,
    last_autosave: Instant,
    last_snapshot: Instant,
    last_prefs_save: Instant,
    autosave_failed: bool,
    show_assertion: Option<ShowAssertion>,
    pub(crate) video: VideoPlayer,
    pub(crate) video_texture: Option<TextureHandle>,
    pub(crate) video_aspect: Option<f32>,
    video_error_shown: bool,
    window_title: String,
    pub(crate) order_drag: Option<OrderDrag>,
    pub(crate) preview_anchor: Instant,
    pub(crate) started: Instant,
    screenshot: Option<PathBuf>,
    screenshot_at: Instant,
    screenshot_requested: bool,
    output_close_signal: Arc<AtomicBool>,
}

impl NerdApp {
    pub fn new(cc: &eframe::CreationContext<'_>, paths: AppPaths, options: LaunchOptions) -> Self {
        crate::update::cleanup_stale(&paths);
        let mut prefs = persistence::load_preferences(&paths.prefs_file).unwrap_or_default();
        if let Some(appearance) = options.appearance {
            prefs.theme = appearance;
        }
        let initial = options.initial.clone();
        let system = nerd_ui_kit::system::detect_with_egui(&cc.egui_ctx);
        let palette = crate::theme::install_with_system(&cc.egui_ctx, prefs.theme, system);
        let library = persistence::load_library(&paths.library_file).unwrap_or_default();
        // The demo is a clean-room fixture: never restore or overwrite user data.
        let recovery = if options.demo {
            None
        } else {
            persistence::load_recovery(&paths.support)
        };

        let mut startup_note = None;
        let mut project = Project::new("Nuova serata");
        let mut project_path = None;
        let candidate = if options.demo {
            None
        } else {
            initial.or_else(|| prefs.recent_projects.first().cloned())
        };
        if let Some(candidate) = candidate {
            match persistence::load_project(&candidate) {
                Ok(mut loaded) => {
                    persistence::normalize_loaded_project(&mut loaded);
                    loaded.reset_statuses();
                    project = loaded;
                    project_path = Some(candidate);
                }
                Err(err) => {
                    startup_note = Some(Toast::new(
                        format!("Impossibile aprire la serata: {err}"),
                        ToastKind::Error,
                    ));
                }
            }
        } else if !options.demo && paths.autosave_file.is_file() {
            if let Ok(mut loaded) = persistence::load_project(&paths.autosave_file) {
                persistence::normalize_loaded_project(&mut loaded);
                loaded.reset_statuses();
                project = loaded;
                startup_note = Some(Toast::new(
                    "Sessione precedente ripristinata. Salva con nome per conservarla.",
                    ToastKind::Info,
                ));
            }
        }
        if project.date.trim().is_empty() {
            project.date = today();
        }
        project.output.blackout = false;

        let mut engine = AudioEngine::new(prefs.audio_device.clone());
        let _ = engine.set_master_volume(prefs.master_volume);
        let show = Show::from_project(&project);
        let selected_song = project.songs.first().map(|song| song.id.clone());
        let selected_asset = library.assets.first().map(|asset| asset.id.clone());
        let missing = persistence::missing_media(&project);
        let (import_tx, import_rx) = mpsc::channel();
        let displays = load_displays();
        let now = Instant::now();
        let update_at = if prefs.check_updates_on_startup && !prefs.update_repo.trim().is_empty() {
            Some(now + Duration::from_secs(4))
        } else {
            None
        };
        let page = if options.gallery {
            Page::Gallery
        } else if let Some(page) = options.page {
            page
        } else if project.songs.is_empty() {
            Page::Library
        } else {
            Page::Live
        };
        let mut app = Self {
            paths,
            prefs,
            project,
            project_path,
            library,
            engine,
            show,
            page,
            palette,
            system,
            chrome: ChromeState::default(),
            menu: MenuBar::install(),
            snackbar: startup_note.map(|toast| Snackbar { toast }),
            undo: UndoStack::default(),
            dirty: false,
            prefs_dirty: false,
            selected_song,
            selected_asset,
            search: String::new(),
            sort: LibrarySort::Title,
            confirm_go: false,
            confirm_quit: false,
            quit_confirmed: false,
            show_shortcuts: false,
            about_open: false,
            update_dialog: false,
            recovery_prompt: recovery.is_some(),
            recovery,
            missing,
            output_open: false,
            checklist: Checklist::default(),
            import_tx,
            import_rx,
            imports_in_flight: 0,
            thumbnails: HashMap::new(),
            thumbnail_failures: HashSet::new(),
            logo: None,
            logo_for: None,
            logo_missing: false,
            background_rx: None,
            background_key: String::new(),
            background_pending_start: false,
            background_error: None,
            updater: Updater::new(),
            startup_update_at: update_at,
            update_notified: false,
            displays,
            last_display_poll: now,
            levels: Levels::default(),
            song_meter: 0.0,
            bg_meter: 0.0,
            last_tick: now,
            last_autosave: now,
            last_snapshot: now,
            last_prefs_save: now,
            autosave_failed: false,
            show_assertion: None,
            video: VideoPlayer::new(),
            video_texture: None,
            video_aspect: None,
            video_error_shown: false,
            window_title: String::new(),
            order_drag: None,
            preview_anchor: now,
            started: now,
            screenshot: options.screenshot,
            screenshot_at: now + Duration::from_millis(2_200),
            screenshot_requested: false,
            output_close_signal: Arc::new(AtomicBool::new(false)),
        };
        if let Some(path) = app.project_path.clone() {
            app.prefs.remember_project(path);
            app.prefs_dirty = true;
        }
        if app.project.date.is_empty() {
            app.project.date = today();
        }
        if let Some(asset) = app.library.assets.iter().find(|a| !a.exists()) {
            startup_note = Some(Toast::new(
                format!(
                    "{} in libreria: {}",
                    crate::strings::MISSING,
                    asset.display_title()
                ),
                ToastKind::Warning,
            ));
            app.snackbar = startup_note.map(|toast| Snackbar { toast });
        }
        if options.demo {
            crate::demo::populate(&mut app);
        }
        if options.gallery {
            app.page = Page::Gallery;
        }
        app.ensure_background_loaded();
        app.ensure_preparations();
        tracing::info!(
            songs = app.project.songs.len(),
            assets = app.library.assets.len(),
            "application ready"
        );
        app
    }

    // ---------------------------------------------------------------------
    // Shared state helpers
    // ---------------------------------------------------------------------

    pub(crate) fn toast(&mut self, message: impl Into<String>, kind: ToastKind) {
        self.snackbar = Some(Snackbar {
            toast: Toast::new(message, kind),
        });
    }

    pub(crate) fn mark_dirty(&mut self) {
        self.dirty = true;
        crash::set_snapshot(&self.project);
    }

    /// Call after editing, passing the project as it was before the edit.
    pub(crate) fn commit_edit(&mut self, before: &Project) {
        self.undo.checkpoint(before);
        self.mark_dirty();
    }

    pub(crate) fn checkpoint(&mut self) {
        let snapshot = self.project.clone();
        self.undo.checkpoint(&snapshot);
        self.mark_dirty();
    }

    pub(crate) fn show_is_active(&self) -> bool {
        self.show.phase() != Phase::Idle || self.show.background_running()
    }

    pub(crate) fn selected_song(&self) -> Option<&Song> {
        let id = self.selected_song.as_deref()?;
        self.project.find_song(id)
    }

    pub(crate) fn selected_song_index(&self) -> Option<usize> {
        let id = self.selected_song.as_deref()?;
        self.project.songs.iter().position(|song| song.id == id)
    }

    pub(crate) fn live_song(&self) -> Option<&Song> {
        self.show
            .live_index()
            .and_then(|index| self.project.songs.get(index))
    }

    pub(crate) fn next_song(&self) -> Option<&Song> {
        self.show
            .cursor_index()
            .and_then(|index| self.project.songs.get(index))
    }

    pub(crate) fn can_edit_structure(&mut self) -> bool {
        if self.show.phase() != Phase::Idle || self.show.background_running() {
            self.toast(
                "Ferma lo spettacolo e il sottofondo prima di modificare la scaletta.",
                ToastKind::Warning,
            );
            false
        } else {
            true
        }
    }

    pub(crate) fn refresh_missing(&mut self) {
        self.missing = persistence::missing_media(&self.project);
    }

    pub(crate) fn display_name(&self) -> String {
        self.displays
            .iter()
            .find(|display| display.index == self.project.output.display_index)
            .map(|display| display.name.clone())
            .unwrap_or_else(|| "Schermo principale".to_string())
    }

    // ---------------------------------------------------------------------
    // Project lifecycle
    // ---------------------------------------------------------------------

    fn rebuild_show(&mut self) {
        let effects = self.show.reset();
        self.apply_effects(effects);
        self.show = Show::from_project(&self.project);
        self.show.set_mode(self.project.show.mode);
        self.show.sync_statuses(&mut self.project);
        self.ensure_preparations();
        self.sync_show_assertion();
    }

    fn new_project(&mut self) {
        if self.show_is_active() {
            self.toast(
                "Ferma lo spettacolo prima di creare una nuova serata.",
                ToastKind::Warning,
            );
            return;
        }
        self.autosave();
        let mut next = Project::new("Nuova serata");
        next.date = today();
        next.background = self.project.background.clone();
        next.title_template = self.project.title_template.clone();
        next.output = self.project.output.clone();
        next.output.blackout = false;
        next.show = self.project.show.clone();
        self.project = next;
        self.project_path = None;
        self.selected_song = None;
        self.undo.reset();
        self.dirty = true;
        self.rebuild_show();
        self.refresh_missing();
        self.close_output_viewport();
        self.page = Page::Timeline;
        self.toast("Nuova serata pronta.", ToastKind::Info);
    }

    pub(crate) fn open_project_dialog(&mut self) {
        if self.show_is_active() {
            self.toast(
                "Ferma lo spettacolo prima di aprire un'altra serata.",
                ToastKind::Warning,
            );
            return;
        }
        let Some(path) = rfd::FileDialog::new()
            .set_title("Apri serata")
            .add_filter("Serata Project Nerd", &[nerd_core::PROJECT_EXTENSION])
            .pick_file()
        else {
            return;
        };
        self.open_project_path(path);
    }

    pub(crate) fn open_project_path(&mut self, path: PathBuf) {
        match persistence::load_project(&path) {
            Ok(mut loaded) => {
                self.autosave();
                persistence::normalize_loaded_project(&mut loaded);
                loaded.reset_statuses();
                loaded.output.blackout = false;
                if loaded.date.trim().is_empty() {
                    loaded.date = today();
                }
                self.project = loaded;
                self.project_path = Some(path.clone());
                self.prefs.remember_project(path);
                self.prefs_dirty = true;
                self.selected_song = self.project.songs.first().map(|song| song.id.clone());
                self.undo.reset();
                self.dirty = false;
                self.close_output_viewport();
                let _ = self.engine.stop_song(0);
                let _ = self.engine.stop_background(0);
                self.video.stop();
                self.video_texture = None;
                self.background_key.clear();
                self.rebuild_show();
                self.refresh_missing();
                self.ensure_background_loaded();
                self.page = Page::Live;
                self.toast(
                    format!("Aperta «{}»", self.project.name),
                    ToastKind::Success,
                );
            }
            Err(err) => {
                self.toast(format!("Apertura non riuscita: {err}"), ToastKind::Error);
            }
        }
    }

    pub(crate) fn save(&mut self, save_as: bool) -> bool {
        if save_as || self.project_path.is_none() {
            let suggested = self.project.suggested_file_name();
            let Some(path) = rfd::FileDialog::new()
                .set_title("Salva serata")
                .add_filter("Serata Project Nerd", &[nerd_core::PROJECT_EXTENSION])
                .set_file_name(&suggested)
                .save_file()
            else {
                return false;
            };
            let path = persistence::ensure_project_path(path);
            self.project_path = Some(path);
        }
        let Some(path) = self.project_path.clone() else {
            return false;
        };
        self.project.validate();
        match persistence::save_project(&self.project, &path) {
            Ok(()) => {
                self.dirty = false;
                self.autosave_failed = false;
                self.last_autosave = Instant::now();
                self.prefs.remember_project(path.clone());
                self.prefs_dirty = true;
                self.save_prefs_now();
                crash::set_snapshot(&self.project);
                self.toast(
                    format!(
                        "Salvato: {}",
                        path.file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("serata")
                    ),
                    ToastKind::Success,
                );
                true
            }
            Err(err) => {
                self.toast(format!("Salvataggio non riuscito: {err}"), ToastKind::Error);
                false
            }
        }
    }

    fn autosave(&mut self) {
        let (path, is_real_save) = match &self.project_path {
            Some(path) => (path.clone(), true),
            None => (self.paths.autosave_file.clone(), false),
        };
        self.project.validate();
        match persistence::save_project(&self.project, &path) {
            Ok(()) => {
                self.autosave_failed = false;
                self.last_autosave = Instant::now();
                if is_real_save {
                    self.dirty = false;
                }
                crash::set_snapshot(&self.project);
            }
            Err(err) => {
                if !self.autosave_failed {
                    self.autosave_failed = true;
                    self.toast(format!("Autosave non riuscito: {err}"), ToastKind::Error);
                }
            }
        }
    }

    pub(crate) fn save_prefs_now(&mut self) {
        self.prefs.clamp();
        match persistence::save_preferences(&self.prefs, &self.paths.prefs_file) {
            Ok(()) => {
                self.prefs_dirty = false;
                self.last_prefs_save = Instant::now();
            }
            Err(err) => tracing::warn!(%err, "could not save preferences"),
        }
    }

    pub(crate) fn save_library(&mut self) {
        if let Err(err) = persistence::save_library(&self.library, &self.paths.library_file) {
            tracing::warn!(%err, "could not save media library");
        }
    }

    // ---------------------------------------------------------------------
    // Show commands
    // ---------------------------------------------------------------------

    pub(crate) fn request_go(&mut self) {
        if self.show.is_empty() {
            self.toast(
                "La scaletta è vuota. Aggiungi almeno un brano.",
                ToastKind::Warning,
            );
            return;
        }
        match self.show.go() {
            GoOutcome::Started(effects) => {
                self.confirm_go = false;
                self.apply_effects(effects);
                self.ensure_preparations();
            }
            GoOutcome::ConfirmRequired => self.confirm_go = true,
            GoOutcome::Ignored => {}
        }
    }

    pub(crate) fn confirm_go_now(&mut self) {
        self.confirm_go = false;
        let effects = self.show.confirm_go();
        self.apply_effects(effects);
        self.ensure_preparations();
    }

    pub(crate) fn toggle_pause(&mut self) {
        let effects = self.show.toggle_pause();
        if effects.is_empty() {
            return;
        }
        self.apply_effects(effects);
    }

    pub(crate) fn stop_show(&mut self) {
        let effects = self.show.stop(1_200);
        if effects.is_empty() {
            return;
        }
        self.apply_effects(effects);
        self.toast("Stop con fade.", ToastKind::Info);
    }

    pub(crate) fn skip_song(&mut self) {
        let effects = self.show.skip();
        if effects.is_empty() {
            self.toast("Nessun brano da saltare.", ToastKind::Info);
            return;
        }
        self.apply_effects(effects);
    }

    pub(crate) fn previous_song(&mut self) {
        let effects = self.show.previous();
        if effects.is_empty() {
            return;
        }
        self.apply_effects(effects);
        self.page = Page::Live;
    }

    pub(crate) fn panic(&mut self) {
        let effects = self.show.panic();
        if effects.is_empty() {
            self.toast("Niente in onda.", ToastKind::Info);
            return;
        }
        self.apply_effects(effects);
        self.toast("PANIC: uscite in fade rapido.", ToastKind::Warning);
    }

    pub(crate) fn set_show_mode(&mut self, mode: PlaybackMode) {
        if self.project.show.mode == mode {
            return;
        }
        self.checkpoint();
        self.project.show.mode = mode;
        self.show.set_mode(mode);
        self.mark_dirty();
        self.toast(
            format!("Modalità {}", crate::widgets::mode_label(mode)),
            ToastKind::Info,
        );
    }

    /// Pushes timing settings edited in the UI into the running show.
    pub(crate) fn sync_show_config(&mut self) {
        self.show.set_config(ShowConfig {
            mode: self.project.show.mode,
            default_gap_ms: self.project.show.default_gap_ms,
            panic_fade_ms: self.project.show.panic_fade_ms,
            background_fade_in_ms: self.project.background.fade_in_ms,
            background_fade_out_ms: self.project.background.fade_out_ms,
        });
    }

    pub(crate) fn toggle_auto_suspend(&mut self) {
        self.show.toggle_auto_suspend();
        if self.show.auto_suspended() {
            self.toast(
                "Automatico sospeso: il prossimo brano attende il GO.",
                ToastKind::Info,
            );
        } else {
            self.toast("Automatico riattivato.", ToastKind::Info);
        }
    }

    pub(crate) fn start_background_manual(&mut self) {
        let effects = self.show.start_background();
        if effects.is_empty() {
            if self.show.background_running() {
                self.toast("Il sottofondo è già in onda.", ToastKind::Info);
            }
            return;
        }
        self.apply_effects(effects);
    }

    pub(crate) fn stop_background_manual(&mut self) {
        let effects = self.show.stop_background();
        if effects.is_empty() {
            return;
        }
        self.apply_effects(effects);
    }

    pub(crate) fn toggle_blackout(&mut self) {
        self.project.output.blackout = !self.project.output.blackout;
        self.mark_dirty();
        if self.project.output.blackout {
            self.toast("Blackout attivo sull'uscita video.", ToastKind::Warning);
        } else {
            self.toast("Blackout disattivato.", ToastKind::Info);
        }
    }

    pub(crate) fn toggle_output(&mut self, ctx: &egui::Context) {
        if self.output_open {
            self.close_output_viewport();
            ctx.send_viewport_cmd_to(output_viewport_id(), egui::ViewportCommand::Close);
            self.toast("Uscita video chiusa.", ToastKind::Info);
        } else {
            self.output_open = true;
            self.toast(
                format!("Uscita video su: {}", self.display_name()),
                ToastKind::Success,
            );
        }
    }

    pub(crate) fn close_output_viewport(&mut self) {
        self.output_open = false;
    }

    fn apply_effects(&mut self, effects: Vec<Effect>) {
        for effect in effects {
            match effect {
                Effect::PlaySong { index } => self.play_song(index),
                Effect::StopSong { fade_ms } => {
                    if let Err(err) = self.engine.stop_song(fade_ms) {
                        tracing::warn!(%err, "stop song");
                    }
                    self.video.stop();
                    self.video_texture = None;
                }
                Effect::StartBackground { fade_in_ms } => {
                    self.ensure_background_loaded();
                    if !self.engine.has_background_program() {
                        self.background_pending_start = true;
                        if self.project.background.tracks.is_empty() {
                            self.toast(
                                "Nessun brano di sottofondo configurato: aggiungilo in Impostazioni.",
                                ToastKind::Warning,
                            );
                        }
                    } else {
                        self.start_background_audio(fade_in_ms);
                    }
                }
                Effect::StopBackground { fade_ms } => {
                    if let Err(err) = self.engine.stop_background(fade_ms) {
                        tracing::warn!(%err, "stop background");
                    }
                    self.background_pending_start = false;
                }
                Effect::PauseSong => {
                    let _ = self.engine.pause();
                    self.video.pause();
                }
                Effect::ResumeSong => {
                    let _ = self.engine.resume();
                    self.video.resume();
                }
                Effect::PauseBackground => {
                    let _ = self.engine.pause_background();
                }
                Effect::ResumeBackground => {
                    let _ = self.engine.resume_background();
                }
            }
        }
        self.sync_show_assertion();
    }

    fn play_song(&mut self, index: usize) {
        let Some(song) = self.project.songs.get(index).cloned() else {
            return;
        };
        let Some(path) = song.audio.clone() else {
            self.fail_song(&song, "Il brano non ha un file audio.".to_string());
            return;
        };
        if !path.is_file() {
            self.fail_song(&song, format!("File audio mancante: {}", path.display()));
            return;
        }
        let key = song_key(&song);
        let start = song.effective_start_ms();
        let end = Some(song.effective_end_ms());
        if !self.engine.prepared(&key) {
            if let Err(err) = self.engine.prepare_now(key.clone(), &path, start, end) {
                self.fail_song(&song, format!("Decodifica non riuscita: {err}"));
                return;
            }
        }
        if let Err(err) =
            self.engine
                .play_song(&key, song.volume, song.fade_in_ms, song.fade_out_ms)
        {
            self.fail_song(&song, format!("Riproduzione non riuscita: {err}"));
            return;
        }
        self.selected_song = Some(song.id.clone());
        self.play_video_for(&song);
        tracing::info!(
            title = %song.display_title(),
            start_ms = start,
            end_ms = ?end,
            "song started"
        );
    }

    fn fail_song(&mut self, song: &Song, message: String) {
        tracing::error!(title = %song.display_title(), %message, "cannot play song");
        self.toast(
            format!("{}: {message}", song.display_title()),
            ToastKind::Error,
        );
        let effects = self.show.skip();
        self.apply_effects(effects);
    }

    fn play_video_for(&mut self, song: &Song) {
        self.video_texture = None;
        self.video_aspect = None;
        self.video_error_shown = false;
        if let Some(path) = song.video.as_ref().filter(|path| path.is_file()) {
            self.video.start(path, song.effective_start_ms());
        } else {
            self.video.stop();
        }
    }

    pub(crate) fn ensure_preparations(&mut self) {
        let mut indices = Vec::new();
        if let Some(index) = self.show.live_index() {
            indices.push(index);
        }
        if let Some(index) = self.show.cursor_index() {
            if !indices.contains(&index) {
                indices.push(index);
            }
        }
        for index in indices {
            let Some(song) = self.project.songs.get(index) else {
                continue;
            };
            let Some(path) = &song.audio else {
                continue;
            };
            if !path.is_file() {
                continue;
            }
            let key = song_key(song);
            if self.engine.prepared(&key) {
                continue;
            }
            if let Err(err) = self.engine.prepare(
                key,
                path.clone(),
                song.effective_start_ms(),
                Some(song.effective_end_ms()),
            ) {
                tracing::warn!(%err, title = %song.display_title(), "preload failed");
            }
        }
    }

    fn start_background_audio(&mut self, fade_in_ms: u64) {
        self.background_pending_start = false;
        let resume = self.project.background.resume_mode == ResumeMode::Resume;
        if let Err(err) = self.engine.start_background(fade_in_ms, resume) {
            self.toast(format!("Sottofondo: {err}"), ToastKind::Warning);
        }
    }

    pub(crate) fn reload_background(&mut self) {
        self.background_key.clear();
        if self.show.background_running() {
            self.background_pending_start = true;
            let _ = self.engine.stop_background(200);
        }
        self.ensure_background_loaded();
    }

    fn desired_background_key(&self) -> String {
        if self.project.background.tracks.is_empty() {
            return "none".to_string();
        }
        let mut key = format!(
            "{}|{}|{}",
            self.engine.sample_rate(),
            self.project.background.shuffle,
            self.project.background.looping
        );
        for path in &self.project.background.tracks {
            key.push('|');
            key.push_str(&path.to_string_lossy());
        }
        key
    }

    pub(crate) fn ensure_background_loaded(&mut self) {
        let key = self.desired_background_key();
        if key == "none" {
            if self.engine.has_background_program() {
                let _ = self.engine.stop_background(0);
                self.engine.clear_background_program();
            }
            self.background_key = key;
            self.background_pending_start = false;
            self.background_error = None;
            return;
        }
        if key == self.background_key && self.engine.has_background_program() {
            return;
        }
        if self.background_rx.is_some() {
            self.background_key = key;
            return;
        }
        let paths = self.project.background.tracks.clone();
        let shuffle = self.project.background.shuffle;
        let looping = self.project.background.looping;
        let rate = self.engine.sample_rate();
        let (tx, rx) = mpsc::channel();
        self.background_rx = Some(rx);
        self.background_key = key.clone();
        self.background_error = None;
        let key_for_thread = key.clone();
        let spawn = std::thread::Builder::new()
            .name("nerd-background".into())
            .spawn(move || {
                let result = nerd_audio::decode_playlist(&paths, shuffle, rate)
                    .map_err(|err| err.to_string());
                let _ = tx.send(BackgroundLoad {
                    key: key_for_thread,
                    looping,
                    result,
                });
            });
        if let Err(err) = spawn {
            self.background_rx = None;
            self.background_error = Some(format!("Impossibile avviare il decoder: {err}"));
        }
    }

    fn poll_background(&mut self) {
        let message = if let Some(rx) = &self.background_rx {
            match rx.try_recv() {
                Ok(message) => Some(Ok(message)),
                Err(mpsc::TryRecvError::Empty) => None,
                Err(mpsc::TryRecvError::Disconnected) => Some(Err(())),
            }
        } else {
            None
        };
        match message {
            Some(Ok(load)) => {
                self.background_rx = None;
                if load.key != self.desired_background_key() {
                    return;
                }
                match load.result {
                    Ok(audio) => {
                        if audio.frames == 0 {
                            self.background_error =
                                Some("Il sottofondo decodificato è vuoto.".to_string());
                            return;
                        }
                        self.engine.set_background_program(
                            Arc::new(audio),
                            load.looping,
                            self.project.background.volume,
                        );
                        self.background_key = load.key;
                        self.background_error = None;
                        if self.background_pending_start && self.show.background_running() {
                            let fade = self.project.background.fade_in_ms;
                            self.start_background_audio(fade);
                        }
                    }
                    Err(err) => {
                        self.background_error = Some(err.clone());
                        self.toast(
                            format!("Sottofondo non disponibile: {err}"),
                            ToastKind::Warning,
                        );
                    }
                }
            }
            Some(Err(())) => {
                self.background_rx = None;
            }
            None => {}
        }
    }

    fn sync_show_assertion(&mut self) {
        let needed = self.show.phase() != Phase::Idle || self.show.background_running();
        if needed && self.show_assertion.is_none() {
            self.show_assertion = Some(ShowAssertion::begin());
        } else if !needed {
            self.show_assertion = None;
        }
    }

    fn handle_audio_events(&mut self) {
        for event in self.engine.poll() {
            match event {
                AudioEvent::Prepared { .. } => {}
                AudioEvent::PrepareFailed { message, .. } => {
                    self.toast(
                        format!("Preparazione fallita: {message}"),
                        ToastKind::Warning,
                    );
                }
                AudioEvent::OutputLost => {
                    self.toast(
                        "Uscita audio scollegata. L'app tenta la riconnessione…",
                        ToastKind::Error,
                    );
                }
                AudioEvent::OutputRestored => {
                    self.toast("Uscita audio riconnessa.", ToastKind::Success);
                    self.background_key.clear();
                    let _ = self.engine.stop_background(0);
                    self.background_pending_start = self.show.background_running();
                    self.ensure_background_loaded();
                    self.ensure_preparations();
                }
            }
        }
    }

    // ---------------------------------------------------------------------
    // Library / running order edits
    // ---------------------------------------------------------------------

    pub(crate) fn import_dialog(&mut self) {
        let Some(paths) = rfd::FileDialog::new()
            .set_title("Importa media")
            .add_filter(
                "Audio e video",
                &[
                    "mp3", "m4a", "wav", "aiff", "aif", "flac", "mp4", "mov", "m4v",
                ],
            )
            .add_filter("Audio", AUDIO_EXTENSIONS)
            .add_filter("Video", VIDEO_EXTENSIONS)
            .pick_files()
        else {
            return;
        };
        self.import_paths(paths);
    }

    pub(crate) fn import_paths(&mut self, paths: Vec<PathBuf>) {
        for path in paths {
            if nerd_core::library::media_kind_for_path(&path).is_none() {
                self.toast(
                    format!("Formato non supportato: {}", path.display()),
                    ToastKind::Warning,
                );
                continue;
            }
            if !path.is_file() {
                self.toast(
                    format!("File non trovato: {}", path.display()),
                    ToastKind::Error,
                );
                continue;
            }
            let tx = self.import_tx.clone();
            let thumbnail_dir = self.paths.thumbnails.clone();
            self.imports_in_flight += 1;
            let spawn = std::thread::Builder::new()
                .name("nerd-import".into())
                .spawn(move || {
                    let outcome = media::import_file(path, thumbnail_dir);
                    let _ = tx.send(outcome);
                });
            if let Err(err) = spawn {
                self.imports_in_flight = self.imports_in_flight.saturating_sub(1);
                self.toast(format!("Importazione non avviata: {err}"), ToastKind::Error);
            }
        }
    }

    fn poll_imports(&mut self) {
        while let Ok(outcome) = self.import_rx.try_recv() {
            self.imports_in_flight = self.imports_in_flight.saturating_sub(1);
            let path = outcome.path.clone();
            if let Some(asset) = outcome.asset {
                let id = asset.id.clone();
                let title = asset.display_title().to_string();
                let is_new = self.library.upsert(asset);
                self.selected_asset = Some(id);
                self.save_library();
                if let Some(error) = outcome.error {
                    self.toast(error, ToastKind::Warning);
                }
                self.toast(
                    if is_new {
                        format!("Importato: {title}")
                    } else {
                        format!("Aggiornato: {title}")
                    },
                    ToastKind::Success,
                );
            } else {
                self.toast(
                    format!(
                        "{}: {}",
                        path.file_name().and_then(|n| n.to_str()).unwrap_or("File"),
                        outcome
                            .error
                            .unwrap_or_else(|| "importazione non riuscita".to_string())
                    ),
                    ToastKind::Error,
                );
            }
        }
    }

    pub(crate) fn add_asset_to_show(&mut self, asset_id: &str) {
        let Some(asset) = self.library.find(asset_id).cloned() else {
            return;
        };
        if !self.can_edit_structure() {
            return;
        }
        if !asset.exists() {
            self.toast("Il file dell'asset non esiste più.", ToastKind::Error);
            return;
        }
        self.checkpoint();
        let mut song = Song::new(
            asset.title.clone(),
            Some(asset.path.clone()),
            asset.duration_ms,
        );
        song.artist = asset.artist.clone();
        if asset.kind == MediaKind::Video {
            song.video = Some(asset.path.clone());
        }
        let insert_at = self
            .selected_song_index()
            .map(|index| index + 1)
            .unwrap_or(self.project.songs.len());
        self.project.insert_song(insert_at, song.clone());
        self.selected_song = Some(song.id.clone());
        self.page = Page::Timeline;
        self.rebuild_show();
        self.refresh_missing();
        self.mark_dirty();
        self.toast(
            format!("Aggiunto alla scaletta: {}", song.display_title()),
            ToastKind::Success,
        );
    }

    pub(crate) fn remove_song(&mut self, index: usize) {
        if !self.can_edit_structure() {
            return;
        }
        if index >= self.project.songs.len() {
            return;
        }
        self.checkpoint();
        let removed = self.project.remove_song(index);
        self.selected_song = self
            .project
            .songs
            .get(index.min(self.project.songs.len().saturating_sub(1)))
            .map(|song| song.id.clone());
        self.rebuild_show();
        self.refresh_missing();
        self.mark_dirty();
        if let Some(song) = removed {
            self.toast(
                format!("Rimosso: {}", song.display_title()),
                ToastKind::Info,
            );
        }
    }

    pub(crate) fn move_song(&mut self, from: usize, to: usize) {
        if !self.can_edit_structure() || from == to {
            return;
        }
        if from >= self.project.songs.len() || to >= self.project.songs.len() {
            return;
        }
        self.checkpoint();
        let moved_id = self.project.songs[from].id.clone();
        self.project.move_song(from, to);
        self.selected_song = Some(moved_id);
        self.rebuild_show();
        self.mark_dirty();
    }

    pub(crate) fn relink_dialog(&mut self) {
        if self.show_is_active() {
            self.toast(
                "Ferma lo spettacolo prima di ricollegare i file.",
                ToastKind::Warning,
            );
            return;
        }
        let Some(folder) = rfd::FileDialog::new()
            .set_title("Scegli la cartella che contiene i file")
            .pick_folder()
        else {
            return;
        };
        let report = persistence::relink_media(&mut self.project, &folder);
        if report.relinked > 0 {
            self.mark_dirty();
            self.refresh_missing();
            self.background_key.clear();
            self.ensure_background_loaded();
            self.rebuild_show();
            self.toast(
                format!("Ricollegati {} file.", report.relinked),
                ToastKind::Success,
            );
        } else {
            self.toast(
                "Nessun file corrispondente trovato in quella cartella.",
                ToastKind::Warning,
            );
        }
    }

    pub(crate) fn undo(&mut self) {
        if self.show_is_active() {
            self.toast(
                "Ferma lo spettacolo per annullare una modifica.",
                ToastKind::Warning,
            );
            return;
        }
        if let Some(project) = self.undo.undo(&self.project) {
            self.project = project;
            self.selected_song = self.project.songs.first().map(|song| song.id.clone());
            self.rebuild_show();
            self.refresh_missing();
            self.dirty = true;
            self.background_key.clear();
            self.ensure_background_loaded();
            self.toast("Modifica annullata.", ToastKind::Info);
        }
    }

    pub(crate) fn redo(&mut self) {
        if self.show_is_active() {
            self.toast(
                "Ferma lo spettacolo per ripetere una modifica.",
                ToastKind::Warning,
            );
            return;
        }
        if let Some(project) = self.undo.redo(&self.project) {
            self.project = project;
            self.selected_song = self.project.songs.first().map(|song| song.id.clone());
            self.rebuild_show();
            self.refresh_missing();
            self.dirty = true;
            self.background_key.clear();
            self.ensure_background_loaded();
            self.toast("Modifica ripristinata.", ToastKind::Info);
        }
    }

    pub(crate) fn reveal_path(&self, path: &Path) {
        let _ = std::process::Command::new("/usr/bin/open")
            .arg("-R")
            .arg(path)
            .spawn();
    }

    pub(crate) fn open_folder(&self, path: &Path) {
        let _ = std::process::Command::new("/usr/bin/open")
            .arg(path)
            .spawn();
    }

    // ---------------------------------------------------------------------
    // Texture caches
    // ---------------------------------------------------------------------

    pub(crate) fn thumbnail_for(
        &mut self,
        ctx: &egui::Context,
        asset: &MediaAsset,
    ) -> Option<TextureHandle> {
        if let Some(texture) = self.thumbnails.get(&asset.id) {
            return Some(texture.clone());
        }
        if !asset.has_thumbnail || self.thumbnail_failures.contains(&asset.id) {
            return None;
        }
        let path = media::thumbnail_path(&self.paths.thumbnails, asset);
        let image = match image::open(&path) {
            Ok(image) => image.to_rgba8(),
            Err(err) => {
                tracing::warn!(path = %path.display(), %err, "thumbnail unreadable");
                self.thumbnail_failures.insert(asset.id.clone());
                return None;
            }
        };
        let size = [image.width() as usize, image.height() as usize];
        let color_image = egui::ColorImage::from_rgba_unmultiplied(size, image.as_raw());
        let texture = ctx.load_texture(
            format!("thumbnail-{}", asset.id),
            color_image,
            egui::TextureOptions::LINEAR,
        );
        self.thumbnails.insert(asset.id.clone(), texture.clone());
        Some(texture)
    }

    fn logo_texture(&mut self, ctx: &egui::Context) -> Option<TextureHandle> {
        let Some(path) = self.project.output.logo_path.clone() else {
            self.logo = None;
            self.logo_for = None;
            self.logo_missing = false;
            return None;
        };
        if self.logo_for.as_ref() == Some(&path) {
            return if self.logo_missing {
                None
            } else {
                self.logo.clone()
            };
        }
        self.logo_for = Some(path.clone());
        self.logo_missing = false;
        match image::open(&path) {
            Ok(image) => {
                let image = image.to_rgba8();
                let size = [image.width() as usize, image.height() as usize];
                let color_image = egui::ColorImage::from_rgba_unmultiplied(size, image.as_raw());
                let texture =
                    ctx.load_texture("project-logo", color_image, egui::TextureOptions::LINEAR);
                self.logo = Some(texture.clone());
                Some(texture)
            }
            Err(err) => {
                tracing::warn!(path = %path.display(), %err, "logo unreadable");
                self.logo_missing = true;
                self.logo = None;
                None
            }
        }
    }

    // ---------------------------------------------------------------------
    // Projector
    // ---------------------------------------------------------------------

    pub(crate) fn projector_state(&mut self, ctx: &egui::Context) -> ProjectorState {
        let logo = self.logo_texture(ctx);
        let live_song = self
            .show
            .live_index()
            .and_then(|index| self.project.songs.get(index))
            .cloned();
        let next_title = self
            .show
            .cursor_index()
            .and_then(|index| self.project.songs.get(index))
            .map(|song| song.display_title());
        let song = live_song.map(|song| ProjectorSong {
            title: song.display_title(),
            artist: song.artist.clone(),
            info: song.info.clone(),
            has_video: song.has_video(),
        });
        let has_video = song.as_ref().is_some_and(|song| song.has_video);
        ProjectorState {
            template: self.project.title_template.clone(),
            title_elapsed_ms: self.show.elapsed_ms(),
            title_active: matches!(self.show.phase(), Phase::Playing | Phase::Paused),
            blackout: self.project.output.blackout,
            show_titles: self.project.output.show_titles,
            idle_screen: self.project.output.idle_screen,
            courtesy_message: self.project.output.courtesy_message.clone(),
            next_title,
            phase: self.show.phase(),
            video_frame: if has_video {
                self.video_texture.clone()
            } else {
                None
            },
            video_aspect: if has_video { self.video_aspect } else { None },
            logo,
            project_name: self.project.name.clone(),
            venue: self.project.venue.clone(),
            preview_backdrop: false,
            song,
        }
    }

    pub(crate) fn projector_preview(
        &mut self,
        ctx: &egui::Context,
        song: &Song,
        elapsed_ms: u64,
    ) -> ProjectorState {
        let logo = self.logo_texture(ctx);
        ProjectorState {
            template: self.project.title_template.clone(),
            title_elapsed_ms: elapsed_ms,
            title_active: true,
            blackout: false,
            show_titles: true,
            idle_screen: self.project.output.idle_screen,
            courtesy_message: self.project.output.courtesy_message.clone(),
            next_title: None,
            phase: Phase::Playing,
            video_frame: None,
            video_aspect: None,
            logo,
            project_name: self.project.name.clone(),
            venue: self.project.venue.clone(),
            preview_backdrop: true,
            song: Some(ProjectorSong {
                title: song.display_title(),
                artist: song.artist.clone(),
                info: song.info.clone(),
                has_video: song.has_video(),
            }),
        }
    }

    fn output_viewport(&mut self, ctx: &egui::Context) {
        if !self.output_open {
            return;
        }
        let state = Arc::new(self.projector_state(ctx));
        let display = if self
            .displays
            .iter()
            .any(|display| display.index == self.project.output.display_index)
        {
            self.project.output.display_index
        } else {
            0
        };
        let signal = self.output_close_signal.clone();
        let root_ctx = ctx.clone();
        ctx.show_viewport_deferred(
            output_viewport_id(),
            egui::ViewportBuilder::default()
                .with_title(format!("{} — Uscita video", nerd_core::APP_NAME))
                .with_fullscreen(true)
                .with_decorations(false)
                .with_monitor(display),
            move |ui, _class| {
                if ui.input(|input| input.key_pressed(egui::Key::Escape)) {
                    signal.store(true, Ordering::SeqCst);
                    root_ctx.request_repaint();
                }
                let rect = ui.max_rect();
                ui.painter()
                    .rect_filled(rect, egui::CornerRadius::ZERO, Color32::BLACK);
                output::paint_output(ui.painter(), rect, &state);
            },
        );
    }

    // ---------------------------------------------------------------------
    // Updates
    // ---------------------------------------------------------------------

    pub(crate) fn check_updates(&mut self, manual: bool) {
        if self.prefs.update_repo.trim().is_empty() {
            self.page = Page::Settings;
            self.toast(
                "Configura il repository GitHub in Impostazioni per gli aggiornamenti.",
                ToastKind::Warning,
            );
            return;
        }
        if manual {
            self.update_dialog = true;
        }
        self.updater
            .check(&self.prefs.update_repo, self.prefs.update_channel, manual);
    }

    pub(crate) fn install_update(&mut self, ctx: &egui::Context) {
        if self.show_is_active() {
            self.toast(
                "Ferma lo spettacolo prima di installare un aggiornamento.",
                ToastKind::Warning,
            );
            return;
        }
        match self.updater.install() {
            Ok(()) => {
                self.quit_confirmed = true;
                self.autosave();
                self.save_prefs_now();
                self.close_output_viewport();
                let _ = self.engine.stop_song(0);
                let _ = self.engine.stop_background(0);
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            Err(err) => self.toast(err, ToastKind::Error),
        }
    }

    // ---------------------------------------------------------------------
    // Menu and window lifecycle
    // ---------------------------------------------------------------------

    fn handle_menu_action(&mut self, ctx: &egui::Context, action: &str) {
        match action {
            "quit" | "close" => self.request_quit(ctx),
            "new" => self.new_project(),
            "open" => self.open_project_dialog(),
            "save" => {
                self.save(false);
            }
            "save-as" => {
                self.save(true);
            }
            "import" => self.import_dialog(),
            "add-to-show" => {
                if let Some(asset) = self.selected_asset.clone() {
                    self.add_asset_to_show(&asset);
                } else {
                    self.toast("Seleziona un media nella Libreria.", ToastKind::Info);
                }
            }
            "relink" => self.relink_dialog(),
            "undo" => self.undo(),
            "redo" => self.redo(),
            "go" => self.request_go(),
            "pause" => self.toggle_pause(),
            "stop" => self.stop_show(),
            "skip" => self.skip_song(),
            "back" => self.previous_song(),
            "panic" => self.panic(),
            "mode-manual" => self.set_show_mode(PlaybackMode::Manual),
            "mode-auto" => self.set_show_mode(PlaybackMode::Automatic),
            "toggle-auto" => self.toggle_auto_suspend(),
            "start-bg" => self.start_background_manual(),
            "stop-bg" => self.stop_background_manual(),
            "blackout" => self.toggle_blackout(),
            "toggle-output" => self.toggle_output(ctx),
            "page-library" => self.page = Page::Library,
            "page-timeline" => self.page = Page::Timeline,
            "page-titles" => self.page = Page::Titles,
            "page-live" => self.page = Page::Live,
            "page-settings" | "settings" => self.page = Page::Settings,
            "update" => self.check_updates(true),
            "shortcuts" => self.show_shortcuts = true,
            "about" => self.about_open = true,
            "data-folder" => self.open_folder(&self.paths.support),
            other => tracing::debug!(action = other, "unhandled menu action"),
        }
    }

    fn request_quit(&mut self, ctx: &egui::Context) {
        if self.show_is_active() && self.prefs.confirm_quit_during_show {
            self.confirm_quit = true;
        } else {
            self.quit_now(ctx);
        }
    }

    fn quit_now(&mut self, ctx: &egui::Context) {
        self.quit_confirmed = true;
        self.autosave();
        self.save_prefs_now();
        self.save_library();
        self.video.stop();
        let _ = self.engine.stop_song(0);
        let _ = self.engine.stop_background(0);
        self.show_assertion = None;
        self.close_output_viewport();
        ctx.send_viewport_cmd_to(output_viewport_id(), egui::ViewportCommand::Close);
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }

    fn handle_close_request(&mut self, ctx: &egui::Context) {
        if self.quit_confirmed {
            return;
        }
        if !ctx.input(|input| input.viewport().close_requested()) {
            return;
        }
        if self.show_is_active() && self.prefs.confirm_quit_during_show {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.confirm_quit = true;
        } else {
            self.autosave();
            self.save_prefs_now();
            self.save_library();
            self.video.stop();
            let _ = self.engine.stop_song(0);
            let _ = self.engine.stop_background(0);
            self.quit_confirmed = true;
        }
    }

    // ---------------------------------------------------------------------
    // Per-frame logic
    // ---------------------------------------------------------------------

    fn logic_frame(&mut self, ctx: &egui::Context) {
        let now = Instant::now();
        let dt = now.duration_since(self.last_tick).as_millis().min(2_000) as u64;
        self.last_tick = now;

        for action in self.menu.poll() {
            self.handle_menu_action(ctx, &action);
        }

        let effects = self.show.tick(dt);
        self.apply_effects(effects);
        self.handle_audio_events();
        self.poll_imports();
        self.poll_background();

        if self.desired_background_key() != self.background_key && self.background_rx.is_none() {
            self.ensure_background_loaded();
        }

        if let Some(path) = self.screenshot.clone() {
            if !self.screenshot_requested && now >= self.screenshot_at {
                self.screenshot_requested = true;
                let callback_ctx = ctx.clone();
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(
                    egui::ScreenshotCallback::new(move |image| {
                        let width = image.width() as u32;
                        let height = image.height() as u32;
                        let mut bytes = Vec::with_capacity(image.pixels.len() * 4);
                        for pixel in &image.pixels {
                            bytes.extend_from_slice(&pixel.to_array());
                        }
                        if let Some(parent) = path.parent() {
                            let _ = std::fs::create_dir_all(parent);
                        }
                        if let Err(error) = image::save_buffer(
                            &path,
                            &bytes,
                            width,
                            height,
                            image::ColorType::Rgba8,
                        ) {
                            eprintln!("screenshot: {error}");
                        }
                        callback_ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }),
                ));
            }
        }

        if self.output_close_signal.swap(false, Ordering::SeqCst) && self.output_open {
            ctx.send_viewport_cmd_to(output_viewport_id(), egui::ViewportCommand::Close);
            self.output_open = false;
            self.toast("Uscita video chiusa.", ToastKind::Info);
        }

        if self.video.is_active() {
            if let Some(frame) = self.video.tick() {
                if frame.width > 0 && frame.height > 0 {
                    let size = [frame.width as usize, frame.height as usize];
                    let image = egui::ColorImage::from_rgba_unmultiplied(size, &frame.rgba);
                    self.video_texture = Some(ctx.load_texture(
                        format!("video-frame-{}", frame.pts_ms),
                        image,
                        egui::TextureOptions::LINEAR,
                    ));
                    self.video_aspect = Some(frame.width as f32 / frame.height.max(1) as f32);
                }
            }
            if let Some(error) = self.video.last_error().map(str::to_string) {
                if !self.video_error_shown {
                    self.video_error_shown = true;
                    self.toast(format!("Video: {error}"), ToastKind::Warning);
                }
            }
        }

        self.levels = self.engine.levels();
        self.song_meter = self.levels.song_peak.max(self.song_meter * 0.86);
        self.bg_meter = self.levels.background_peak.max(self.bg_meter * 0.86);

        self.show.sync_statuses(&mut self.project);
        if self.last_snapshot.elapsed() > Duration::from_secs(5) {
            crash::set_snapshot(&self.project);
            self.last_snapshot = now;
        }
        self.sync_show_assertion();

        if now.duration_since(self.last_display_poll) > Duration::from_secs(2) {
            self.last_display_poll = now;
            self.refresh_displays();
            let system = nerd_ui_kit::system::detect_with_egui(ctx);
            if system != self.system {
                self.system = system;
                self.palette =
                    crate::theme::install_with_system(ctx, self.prefs.theme, self.system);
            }
        }

        if let Some(at) = self.startup_update_at {
            if now >= at {
                self.startup_update_at = None;
                self.check_updates(false);
            }
        }
        self.updater.poll();
        if let Some(release) = self.updater.available() {
            if !self.update_notified {
                self.update_notified = true;
                if !self.show_is_active() {
                    self.toast(
                        format!("Aggiornamento {} disponibile.", release.version),
                        ToastKind::Info,
                    );
                }
            }
        }

        if self.prefs_dirty && now.duration_since(self.last_prefs_save) > Duration::from_secs(1) {
            self.save_prefs_now();
        }
        if self.dirty && now.duration_since(self.last_autosave) > Duration::from_secs(3) {
            self.autosave();
        }

        if let Some(snackbar) = &self.snackbar {
            if snackbar.toast.born.elapsed() > Duration::from_secs(5) {
                self.snackbar = None;
            }
        }

        self.handle_close_request(ctx);

        let title = format!(
            "{}{} — {}",
            if self.dirty { "● " } else { "" },
            self.project.name,
            nerd_core::APP_NAME
        );
        if title != self.window_title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.window_title = title;
        }

        let active = self.show.phase() != Phase::Idle || self.show.background_running();
        ctx.request_repaint_after(if active {
            Duration::from_millis(16)
        } else if self.updater.status.is_busy() || self.imports_in_flight > 0 {
            Duration::from_millis(80)
        } else {
            Duration::from_millis(200)
        });
    }

    pub(crate) fn refresh_displays(&mut self) {
        let mut displays = load_displays();
        if displays.is_empty() {
            displays.push(fallback_display());
        }
        self.displays = displays;
        let index = self.project.output.display_index;
        if !self.displays.iter().any(|display| display.index == index) {
            self.project.output.display_index = 0;
            if self.output_open {
                self.close_output_viewport();
                self.toast(
                    "Lo schermo scelto non è più disponibile. Uscita chiusa.",
                    ToastKind::Warning,
                );
            }
        }
    }

    // ---------------------------------------------------------------------
    // UI
    // ---------------------------------------------------------------------

    #[allow(dead_code)]
    fn header_ui(&mut self, ui: &mut egui::Ui) {
        let palette = self.palette;
        egui::Panel::top("workspace-header")
            .frame(
                egui::Frame::new()
                    .fill(palette.colors.panel)
                    .inner_margin(egui::Margin::symmetric(18, 10))
                    .stroke(Stroke::new(1.0, palette.colors.separator)),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let (logo_rect, _) =
                        ui.allocate_exact_size(egui::vec2(32.0, 32.0), egui::Sense::hover());
                    ui.painter().rect_filled(
                        logo_rect,
                        egui::CornerRadius::same(9),
                        palette.colors.accent_soft,
                    );
                    ui.painter().text(
                        logo_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "N",
                        egui::FontId::proportional(18.0),
                        palette.colors.accent,
                    );
                    ui.add_space(4.0);
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(nerd_core::APP_NAME)
                                    .size(14.0)
                                    .strong()
                                    .color(palette.colors.text),
                            );
                            crate::widgets::pill(
                                ui,
                                format!("v{}", nerd_core::APP_VERSION),
                                palette.colors.accent,
                            );
                        });
                        ui.label(
                            RichText::new(&self.project.name)
                                .size(11.5)
                                .color(palette.colors.text_secondary),
                        );
                    });
                    ui.add_space(18.0);
                    ui.separator();
                    ui.add_space(10.0);

                    let phase = self.show.phase();
                    let phase_color = match phase {
                        Phase::Playing => palette.colors.accent,
                        Phase::Paused => palette.colors.amber,
                        Phase::Waiting => palette.colors.title,
                        Phase::Panic | Phase::Stopping => palette.colors.red,
                        Phase::Finished => palette.colors.green,
                        Phase::Idle => palette.colors.text_secondary,
                    };
                    crate::widgets::pill(ui, output::phase_label(phase), phase_color);
                    if let Some(song) = self.live_song() {
                        ui.label(
                            RichText::new(song.display_title())
                                .size(13.0)
                                .strong()
                                .color(palette.colors.text),
                        );
                        if !song.artist.is_empty() {
                            ui.label(
                                RichText::new(&song.artist)
                                    .size(11.5)
                                    .color(palette.colors.text_secondary),
                            );
                        }
                    } else if let Some(next) = self.next_song() {
                        ui.label(
                            RichText::new(format!("Prossimo: {}", next.display_title()))
                                .size(12.0)
                                .color(palette.colors.text_secondary),
                        );
                    }
                    if self.show.is_counting_down() {
                        crate::widgets::pill(
                            ui,
                            format!(
                                "Prossimo tra {:.1}s",
                                self.show.gap_remaining_ms().unwrap_or(0) as f32 / 1000.0
                            ),
                            palette.colors.amber,
                        );
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let save_label = if self.dirty {
                            crate::strings::UNSAVED
                        } else {
                            crate::strings::SAVED
                        };
                        let save_color = if self.dirty {
                            palette.colors.amber
                        } else {
                            palette.colors.text_secondary
                        };
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new(format!("● {save_label}"))
                                        .size(11.0)
                                        .color(save_color),
                                )
                                .frame(false),
                            )
                            .on_hover_text("Salva la serata (⌘S)")
                            .clicked()
                        {
                            self.save(false);
                        }
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new(if self.project.output.blackout {
                                        "BLACKOUT"
                                    } else {
                                        "Blackout"
                                    })
                                    .size(11.0)
                                    .color(
                                        if self.project.output.blackout {
                                            Color32::WHITE
                                        } else {
                                            palette.colors.text_secondary
                                        },
                                    ),
                                )
                                .fill(if self.project.output.blackout {
                                    palette.colors.red
                                } else {
                                    palette.colors.card
                                })
                                .stroke(Stroke::new(1.0, palette.colors.separator))
                                .corner_radius(egui::CornerRadius::same(7)),
                            )
                            .on_hover_text("Blackout uscita video (⌘B)")
                            .clicked()
                        {
                            self.toggle_blackout();
                        }
                        let output_color = if self.output_open {
                            palette.colors.accent
                        } else {
                            palette.colors.text_secondary
                        };
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new(format!("◉ {}", self.display_name()))
                                        .size(11.0)
                                        .color(output_color),
                                )
                                .fill(palette.colors.card)
                                .stroke(Stroke::new(1.0, palette.colors.separator))
                                .corner_radius(egui::CornerRadius::same(7)),
                            )
                            .on_hover_text("Apri o chiudi l'uscita video")
                            .clicked()
                        {
                            self.toggle_output(ui.ctx());
                        }
                        if let Some(release) = self.updater.available() {
                            let color = palette.colors.title;
                            if ui
                                .add(
                                    egui::Button::new(
                                        RichText::new(format!("↑ {}", release.version))
                                            .size(11.0)
                                            .color(color),
                                    )
                                    .fill(color.gamma_multiply(0.12))
                                    .stroke(Stroke::new(1.0, color.gamma_multiply(0.4)))
                                    .corner_radius(egui::CornerRadius::same(7)),
                                )
                                .on_hover_text("Aggiornamento disponibile")
                                .clicked()
                            {
                                self.update_dialog = true;
                            }
                        }
                    });
                });
            });
    }

    #[allow(dead_code)]
    fn recovery_banner(&mut self, ui: &mut egui::Ui) {
        if !self.recovery_prompt || self.recovery.is_none() {
            return;
        }
        let palette = self.palette;
        egui::Panel::top("recovery-banner")
            .frame(
                egui::Frame::new()
                    .fill(palette.colors.amber.gamma_multiply(0.12))
                    .inner_margin(egui::Margin::symmetric(18, 8))
                    .stroke(Stroke::new(1.0, palette.colors.amber.gamma_multiply(0.4))),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("Sessione non salvata rilevata")
                            .size(12.0)
                            .strong()
                            .color(palette.colors.amber),
                    );
                    ui.label(
                        RichText::new(
                            "L'app è stata chiusa in modo imprevisto durante l'ultima serata.",
                        )
                        .size(11.5)
                        .color(palette.colors.text_secondary),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if crate::widgets::accent_button(ui, palette, "Ripristina sessione")
                            .clicked()
                        {
                            self.restore_recovery();
                        }
                        if ui.button("Ignora").clicked() {
                            self.discard_recovery();
                        }
                    });
                });
            });
    }

    #[allow(dead_code)]
    fn missing_banner(&mut self, ui: &mut egui::Ui) {
        if self.missing.is_empty() {
            return;
        }
        let palette = self.palette;
        let count = self.missing.len();
        let names: Vec<String> = self
            .missing
            .iter()
            .take(2)
            .map(|missing| missing.song_title.clone())
            .collect();
        egui::Panel::top("missing-banner")
            .frame(
                egui::Frame::new()
                    .fill(palette.colors.red.gamma_multiply(0.1))
                    .inner_margin(egui::Margin::symmetric(18, 7))
                    .stroke(Stroke::new(1.0, palette.colors.red.gamma_multiply(0.35))),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!("⚠ {count} file mancanti"))
                            .size(11.5)
                            .strong()
                            .color(palette.colors.red),
                    );
                    ui.label(
                        RichText::new(names.join(", "))
                            .size(11.0)
                            .color(palette.colors.text_secondary),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if crate::widgets::ghost_button(ui, palette, "Ricollega…").clicked() {
                            self.relink_dialog();
                        }
                    });
                });
            });
    }

    #[allow(dead_code)]
    fn footer_ui(&mut self, ui: &mut egui::Ui) {
        let palette = self.palette;
        egui::Panel::bottom("workspace-pages")
            .frame(
                egui::Frame::new()
                    .fill(palette.colors.card)
                    .inner_margin(egui::Margin::symmetric(16, 7))
                    .stroke(Stroke::new(1.0, palette.colors.separator)),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    for page in Page::ALL {
                        let selected = self.page == page;
                        let text = RichText::new(page.title()).size(12.5).color(if selected {
                            palette.colors.text
                        } else {
                            palette.colors.text_secondary
                        });
                        let button = egui::Button::new(text)
                            .fill(if selected {
                                palette.colors.card
                            } else {
                                Color32::TRANSPARENT
                            })
                            .stroke(if selected {
                                Stroke::new(1.0, palette.colors.separator)
                            } else {
                                Stroke::NONE
                            })
                            .corner_radius(egui::CornerRadius::same(8))
                            .min_size(egui::vec2(118.0, 32.0));
                        if ui
                            .add(button)
                            .on_hover_text(format!("{} (⌘{})", page.title(), page.index() + 1))
                            .clicked()
                        {
                            self.page = page;
                        }
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .button("?")
                            .on_hover_text("Scorciatoie da tastiera")
                            .clicked()
                        {
                            self.show_shortcuts = true;
                        }
                        ui.label(
                            RichText::new("Spazio GO · P Pausa · S Stop · N Salta · B Indietro")
                                .size(10.5)
                                .color(palette.colors.text_secondary),
                        );
                    });
                });
            });
    }

    fn shortcuts_window(&mut self, ctx: &egui::Context) {
        if !self.show_shortcuts {
            return;
        }
        let palette = self.palette;
        let mut open = true;
        egui::Window::new("Scorciatoie")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .min_width(420.0)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.label(
                    RichText::new("Durante lo spettacolo")
                        .size(12.0)
                        .color(palette.colors.text_secondary),
                );
                ui.add_space(6.0);
                for (key, action) in [
                    ("Spazio", "GO / avvia il brano"),
                    ("P", "Pausa o riprendi"),
                    ("S", "Stop con fade"),
                    ("N", "Salta al prossimo"),
                    ("B", "Brano precedente"),
                    ("⇧⌘P", "Panic"),
                    ("⌘B", "Blackout uscita video"),
                ] {
                    crate::widgets::shortcut_chip(ui, palette, key, action);
                }
                ui.add_space(10.0);
                ui.label(
                    RichText::new("Sempre")
                        .size(12.0)
                        .color(palette.colors.text_secondary),
                );
                ui.add_space(6.0);
                for (key, action) in [
                    ("⌘N", "Nuova serata"),
                    ("⌘O", "Apri serata"),
                    ("⌘S", "Salva"),
                    ("⇧⌘S", "Salva con nome"),
                    ("⌘I", "Importa media"),
                    ("⌘Z", "Annulla"),
                    ("⇧⌘Z", "Ripeti"),
                    ("⌘1…5", "Cambia pagina"),
                    ("⌘,", "Impostazioni"),
                ] {
                    crate::widgets::shortcut_chip(ui, palette, key, action);
                }
                ui.add_space(12.0);
                if ui.button("Chiudi").clicked() {
                    self.show_shortcuts = false;
                }
            });
        if !open {
            self.show_shortcuts = false;
        }
    }

    fn about_window(&mut self, ctx: &egui::Context) {
        if !self.about_open {
            return;
        }
        let palette = self.palette;
        let mut open = true;
        egui::Window::new(format!("Informazioni su {}", nerd_core::APP_NAME))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .min_width(380.0)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.label(
                    RichText::new(nerd_core::APP_NAME)
                        .size(24.0)
                        .strong()
                        .color(palette.colors.text),
                );
                ui.label(
                    RichText::new(format!("Alpha {}", nerd_core::APP_VERSION))
                        .size(12.0)
                        .color(palette.colors.accent),
                );
                ui.add_space(10.0);
                ui.label("Regia audio e video per le serate della scuola di musica.");
                ui.label(
                    RichText::new("macOS 13+ · Universal · Rust")
                        .size(11.0)
                        .color(palette.colors.text_secondary),
                );
                ui.add_space(12.0);
                if ui.button("Chiudi").clicked() {
                    self.about_open = false;
                }
            });
        if !open {
            self.about_open = false;
        }
    }

    fn dialogs(&mut self, ctx: &egui::Context) {
        self.shortcuts_window(ctx);
        self.about_window(ctx);
        self.update_window(ctx);

        if self.confirm_go {
            let mut open = true;
            let mut confirm = false;
            let mut cancel = false;
            egui::Window::new("Brano già in onda")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label(
                        "Un brano è attualmente in onda. Vuoi interromperlo e passare al successivo?",
                    );
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        if crate::widgets::danger_button(
                            ui,
                            self.palette,
                            "Salta e avvia il prossimo",
                        )
                        .clicked()
                        {
                            confirm = true;
                        }
                        if ui.button("Annulla").clicked() {
                            cancel = true;
                        }
                    });
                });
            if confirm {
                self.confirm_go_now();
            } else if cancel || !open {
                self.confirm_go = false;
            }
            open = true;
            let _ = open;
        }

        if self.confirm_quit {
            let mut quit = false;
            let mut cancel = false;
            let active = self.live_song().map(|song| song.display_title());
            egui::Window::new("Uscire da Project Nerd?")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    if let Some(title) = active {
                        ui.label(
                            RichText::new(format!("In onda: {title}"))
                                .strong()
                                .color(self.palette.colors.amber),
                        );
                        ui.add_space(4.0);
                    }
                    ui.label(
                        "Lo spettacolo verrà fermato. La serata viene salvata automaticamente.",
                    );
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        if crate::widgets::danger_button(ui, self.palette, "Esci").clicked() {
                            quit = true;
                        }
                        if ui.button("Resta").clicked() {
                            cancel = true;
                        }
                    });
                });
            if quit {
                self.confirm_quit = false;
                self.quit_now(ctx);
            } else if cancel {
                self.confirm_quit = false;
            }
        }
    }

    fn update_window(&mut self, ctx: &egui::Context) {
        if !self.update_dialog {
            return;
        }
        let palette = self.palette;
        let mut open = true;
        let status = self.updater.status.clone();
        egui::Window::new("Aggiornamenti")
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_width(520.0)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.label(
                    RichText::new(format!(
                        "Canale {} · versione installata {}",
                        match self.prefs.update_channel {
                            UpdateChannel::Stable => "Stabile",
                            UpdateChannel::Alpha => "Alpha",
                        },
                        nerd_core::APP_VERSION
                    ))
                    .size(11.0)
                    .color(palette.colors.text_secondary),
                );
                ui.add_space(8.0);
                ui.label(status.description());
                match &status {
                    UpdateStatus::Available(release) => {
                        ui.add_space(8.0);
                        ui.label(
                            RichText::new(format!("{} · {}", release.name, release.version))
                                .strong()
                                .size(13.0),
                        );
                        let meta = if release.published_at.is_empty() {
                            release.tag.clone()
                        } else {
                            format!("{} · {}", release.tag, release.published_at)
                        };
                        ui.label(
                            RichText::new(meta)
                                .size(10.5)
                                .color(palette.colors.text_secondary),
                        );
                        if release.zip_size > 0 {
                            ui.label(
                                RichText::new(format!(
                                    "Download: {:.1} MB",
                                    release.zip_size as f32 / 1_000_000.0
                                ))
                                .size(11.0)
                                .color(palette.colors.text_secondary),
                            );
                        }
                        if !release.notes.trim().is_empty() {
                            ui.add_space(6.0);
                            egui::ScrollArea::vertical()
                                .max_height(200.0)
                                .show(ui, |ui| {
                                    ui.label(
                                        RichText::new(&release.notes)
                                            .size(11.5)
                                            .color(palette.colors.text_secondary),
                                    );
                                });
                        }
                        ui.add_space(10.0);
                        if crate::widgets::accent_button(
                            ui,
                            palette,
                            "Scarica aggiornamento",
                        )
                        .clicked()
                        {
                            self.updater.download(
                                &self.prefs.update_repo,
                                release.clone(),
                            );
                        }
                    }
                    UpdateStatus::Downloading { progress, .. } => {
                        ui.add_space(8.0);
                        ui.add(
                            egui::ProgressBar::new(*progress)
                                .show_percentage()
                                .desired_width(460.0),
                        );
                    }
                    UpdateStatus::Ready(prepared) => {
                        ui.add_space(8.0);
                        ui.label(
                            RichText::new(format!(
                                "Versione {} pronta.",
                                prepared.release.version
                            ))
                            .strong(),
                        );
                        ui.label(
                            RichText::new(
                                "L'app verrà sostituita e riavviata. L'installazione avviene solo su conferma.",
                            )
                            .size(11.0)
                            .color(palette.colors.text_secondary),
                        );
                        ui.add_space(8.0);
                        let blocked = self.show_is_active();
                        ui.add_enabled_ui(!blocked, |ui| {
                            if crate::widgets::accent_button(
                                ui,
                                palette,
                                "Installa e riavvia",
                            )
                            .clicked()
                            {
                                self.install_update(ui.ctx());
                            }
                        });
                        if blocked {
                            ui.label(
                                RichText::new(
                                    "Ferma lo spettacolo per installare l'aggiornamento.",
                                )
                                .size(11.0)
                                .color(palette.colors.amber),
                            );
                        }
                    }
                    UpdateStatus::Error(error) => {
                        ui.add_space(6.0);
                        ui.label(RichText::new(error).color(palette.colors.red));
                    }
                    _ => {}
                }
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    ui.add_enabled_ui(
                        !self.updater.is_busy() && !self.prefs.update_repo.trim().is_empty(),
                        |ui| {
                            if ui.button("Controlla ora").clicked() {
                                self.check_updates(true);
                            }
                        },
                    );
                    if ui.button("Apri Impostazioni").clicked() {
                        self.page = Page::Settings;
                        self.update_dialog = false;
                    }
                    if ui.button("Chiudi").clicked() {
                        self.update_dialog = false;
                    }
                });
            });
        if !open {
            self.update_dialog = false;
        }
    }

    fn snackbar_ui(&mut self, ctx: &egui::Context) {
        let Some(snackbar) = &self.snackbar else {
            return;
        };
        let palette = self.palette;
        let color = match snackbar.toast.kind {
            ToastKind::Info => palette.colors.title,
            ToastKind::Success => palette.colors.green,
            ToastKind::Warning => palette.colors.amber,
            ToastKind::Error => palette.colors.red,
        };
        egui::Area::new(egui::Id::unique("project-nerd-toast"))
            .order(egui::Order::Foreground)
            .anchor(egui::Align2::RIGHT_TOP, [-20.0, 78.0])
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(palette.colors.card)
                    .stroke(Stroke::new(1.0, color.gamma_multiply(0.5)))
                    .corner_radius(egui::CornerRadius::same(10))
                    .inner_margin(egui::Margin::symmetric(14, 10))
                    .show(ui, |ui| {
                        ui.set_max_width(420.0);
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("●").size(10.0).color(color));
                            ui.label(
                                RichText::new(&snackbar.toast.message)
                                    .size(12.0)
                                    .color(palette.colors.text),
                            );
                        });
                    });
            });
    }

    fn drop_overlay(&mut self, ctx: &egui::Context) {
        let hovering = ctx.input(|input| !input.raw.hovered_files.is_empty());
        if !hovering {
            return;
        }
        let palette = self.palette;
        egui::Area::new(egui::Id::unique("drop-overlay"))
            .order(egui::Order::Tooltip)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(palette.colors.panel.gamma_multiply(0.96))
                    .stroke(Stroke::new(2.0, palette.colors.accent))
                    .corner_radius(egui::CornerRadius::same(16))
                    .inner_margin(egui::Margin::same(30))
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new("Rilascia per importare")
                                .size(18.0)
                                .strong()
                                .color(palette.colors.text),
                        );
                        ui.label(
                            RichText::new("Audio, video o una serata .nerdshow")
                                .size(12.0)
                                .color(palette.colors.text_secondary),
                        );
                    });
            });
    }

    fn handle_dropped_files(&mut self, ctx: &egui::Context) {
        let files = ctx.input(|input| input.raw.dropped_files.clone());
        if files.is_empty() {
            return;
        }
        let mut media = Vec::new();
        for file in files {
            let path = file.path().to_path_buf();
            if path
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| ext.eq_ignore_ascii_case(nerd_core::PROJECT_EXTENSION))
            {
                if self.show_is_active() {
                    self.toast(
                        "Ferma lo spettacolo prima di aprire una serata.",
                        ToastKind::Warning,
                    );
                } else {
                    self.open_project_path(path);
                }
            } else {
                media.push(path);
            }
        }
        if !media.is_empty() {
            self.import_paths(media);
        }
    }

    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        let modal_open = self.confirm_go
            || self.confirm_quit
            || self.show_shortcuts
            || self.about_open
            || self.update_dialog;
        if modal_open {
            if ctx.input(|input| input.key_pressed(egui::Key::Escape)) {
                self.confirm_go = false;
                self.confirm_quit = false;
                self.show_shortcuts = false;
                self.about_open = false;
                self.update_dialog = false;
            }
            return;
        }
        if ctx.memory(|memory| memory.focused().is_some()) {
            return;
        }
        let (go, pause, stop, skip, back) = ctx.input(|input| {
            (
                input.key_pressed(egui::Key::Space),
                input.key_pressed(egui::Key::P),
                input.key_pressed(egui::Key::S),
                input.key_pressed(egui::Key::N),
                input.key_pressed(egui::Key::B),
            )
        });
        if go {
            self.request_go();
        }
        if pause {
            self.toggle_pause();
        }
        if stop {
            self.stop_show();
        }
        if skip {
            self.skip_song();
        }
        if back {
            self.previous_song();
        }
    }

    pub(crate) fn restore_recovery(&mut self) {
        let Some(mut project) = self.recovery.take() else {
            return;
        };
        persistence::normalize_loaded_project(&mut project);
        project.reset_statuses();
        project.output.blackout = false;
        if project.date.trim().is_empty() {
            project.date = today();
        }
        self.project = project;
        self.project_path = None;
        self.dirty = true;
        self.undo.reset();
        self.selected_song = self.project.songs.first().map(|song| song.id.clone());
        self.rebuild_show();
        self.refresh_missing();
        self.background_key.clear();
        self.ensure_background_loaded();
        self.recovery_prompt = false;
        persistence::clear_recovery(&self.paths.support);
        self.page = Page::Timeline;
        self.toast("Sessione ripristinata.", ToastKind::Success);
    }

    pub(crate) fn discard_recovery(&mut self) {
        self.recovery = None;
        self.recovery_prompt = false;
        persistence::clear_recovery(&self.paths.support);
        self.toast("Sessione di recupero ignorata.", ToastKind::Info);
    }

    fn ui_frame(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.handle_dropped_files(&ctx);
        self.handle_shortcuts(&ctx);

        crate::shell::titlebar(self, ui, frame);
        crate::shell::recovery_banner(self, ui);
        crate::shell::missing_banner(self, ui);
        crate::shell::pagebar(self, ui);

        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(self.palette.colors.window)
                    .inner_margin(egui::Margin::ZERO),
            )
            .show(ui, |ui| {
                pages::show(self, ui);
            });

        self.drop_overlay(&ctx);
        self.snackbar_ui(&ctx);
        self.dialogs(&ctx);
        self.output_viewport(&ctx);
    }
}

impl eframe::App for NerdApp {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.logic_frame(ctx);
    }

    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        self.ui_frame(ui, frame);
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        self.palette.colors.window.to_normalized_gamma_f32()
    }
}

fn song_key(song: &Song) -> String {
    format!(
        "{}|{}|{}|{}",
        song.id,
        song.effective_start_ms(),
        song.effective_end_ms(),
        song.audio
            .as_ref()
            .map(|path| path.to_string_lossy().to_string())
            .unwrap_or_default()
    )
}

fn today() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

fn output_viewport_id() -> egui::ViewportId {
    egui::ViewportId::from_hash_of("project-nerd-output")
}

fn load_displays() -> Vec<nerd_video::DisplayInfo> {
    let mut displays = nerd_video::displays();
    if displays.is_empty() {
        displays.push(fallback_display());
    }
    displays
}

fn fallback_display() -> nerd_video::DisplayInfo {
    nerd_video::DisplayInfo {
        index: 0,
        name: "Schermo principale".to_string(),
        width: 0,
        height: 0,
        is_main: true,
        refresh_hz: 0.0,
    }
}

mod pages {
    pub use crate::pages::*;
}

use crate::crash;
