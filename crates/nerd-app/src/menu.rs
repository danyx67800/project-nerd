//! Native macOS menu bar built with muda.
//!
//! The menu is installed once, after eframe has created the NSApplication.
//! Items are identified by stable string ids; the UI polls the muda channel in
//! `logic`, so menu actions work even while a song is playing.

use muda::accelerator::{Accelerator, Code, Modifiers};
use muda::{Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};
use nerd_core::APP_NAME;

pub struct MenuBar {
    _menu: Menu,
    /// Readable setup error, shown once in the UI instead of silently losing
    /// the menu.
    pub error: Option<String>,
}

impl MenuBar {
    pub fn install() -> Self {
        match Self::build() {
            Ok(menu) => Self {
                _menu: menu,
                error: None,
            },
            Err(error) => {
                tracing::error!(%error, "native menu setup failed");
                Self {
                    _menu: Menu::new(),
                    error: Some(error.to_string()),
                }
            }
        }
    }

    fn build() -> Result<Menu, muda::Error> {
        let menu = Menu::new();

        let about = MenuItem::with_id("about", format!("Informazioni su {APP_NAME}"), true, None);
        let update = MenuItem::with_id(
            "update",
            "Cerca aggiornamenti…",
            true,
            accel(Modifiers::META | Modifiers::SHIFT, Code::KeyU),
        );
        let settings = MenuItem::with_id(
            "settings",
            "Impostazioni…",
            true,
            Some(Accelerator::new(Modifiers::META, Code::Comma)),
        );
        let quit = PredefinedMenuItem::quit(Some(&format!("Esci da {APP_NAME}")));
        let app_menu = submenu(
            APP_NAME,
            &[
                &about,
                &PredefinedMenuItem::separator(),
                &update,
                &PredefinedMenuItem::separator(),
                &settings,
                &PredefinedMenuItem::separator(),
                &PredefinedMenuItem::services(None),
                &PredefinedMenuItem::separator(),
                &PredefinedMenuItem::hide(Some(&format!("Nascondi {APP_NAME}"))),
                &PredefinedMenuItem::hide_others(None),
                &PredefinedMenuItem::show_all(None),
                &PredefinedMenuItem::separator(),
                &quit,
            ],
        );

        let new = MenuItem::with_id(
            "new",
            "Nuova serata",
            true,
            Some(Accelerator::new(Modifiers::META, Code::KeyN)),
        );
        let open = MenuItem::with_id(
            "open",
            "Apri…",
            true,
            Some(Accelerator::new(Modifiers::META, Code::KeyO)),
        );
        let save = MenuItem::with_id(
            "save",
            "Salva",
            true,
            Some(Accelerator::new(Modifiers::META, Code::KeyS)),
        );
        let save_as = MenuItem::with_id(
            "save-as",
            "Salva con nome…",
            true,
            Some(Accelerator::new(
                Modifiers::META | Modifiers::SHIFT,
                Code::KeyS,
            )),
        );
        let import = MenuItem::with_id(
            "import",
            "Importa media…",
            true,
            Some(Accelerator::new(Modifiers::META, Code::KeyI)),
        );
        let add_to_show = MenuItem::with_id(
            "add-to-show",
            "Aggiungi alla scaletta",
            true,
            Some(Accelerator::new(Modifiers::META, Code::Enter)),
        );
        let relink = MenuItem::with_id("relink", "Ricollega file mancanti…", true, None);
        let close = PredefinedMenuItem::close_window(Some("Chiudi finestra"));
        let file_menu = submenu(
            "File",
            &[
                &new,
                &open,
                &PredefinedMenuItem::separator(),
                &save,
                &save_as,
                &PredefinedMenuItem::separator(),
                &import,
                &add_to_show,
                &relink,
                &PredefinedMenuItem::separator(),
                &close,
            ],
        );

        let undo = MenuItem::with_id(
            "undo",
            "Annulla",
            false,
            Some(Accelerator::new(Modifiers::META, Code::KeyZ)),
        );
        let redo = MenuItem::with_id(
            "redo",
            "Ripeti",
            false,
            Some(Accelerator::new(
                Modifiers::META | Modifiers::SHIFT,
                Code::KeyZ,
            )),
        );
        let cut = PredefinedMenuItem::cut(None);
        let copy = PredefinedMenuItem::copy(None);
        let paste = PredefinedMenuItem::paste(None);
        let select_all = PredefinedMenuItem::select_all(None);
        let edit_menu = submenu(
            "Modifica",
            &[
                &undo,
                &redo,
                &PredefinedMenuItem::separator(),
                &cut,
                &copy,
                &paste,
                &select_all,
            ],
        );

        let go = MenuItem::with_id("go", "GO", true, None);
        let pause = MenuItem::with_id("pause", "Pausa / Riprendi", true, None);
        let stop = MenuItem::with_id("stop", "Stop con fade", true, None);
        let skip = MenuItem::with_id("skip", "Salta brano", true, None);
        let back = MenuItem::with_id("back", "Brano precedente", true, None);
        let panic = MenuItem::with_id(
            "panic",
            "Panic (stop immediato)",
            true,
            Some(Accelerator::new(
                Modifiers::META | Modifiers::SHIFT,
                Code::KeyP,
            )),
        );
        let manual = MenuItem::with_id("mode-manual", "Modalità Manuale", true, None);
        let automatic = MenuItem::with_id("mode-auto", "Modalità Automatica", true, None);
        let suspend =
            MenuItem::with_id("toggle-auto", "Sospendi / riprendi automatico", true, None);
        let start_bg = MenuItem::with_id("start-bg", "Avvia sottofondo", true, None);
        let stop_bg = MenuItem::with_id("stop-bg", "Ferma sottofondo", true, None);
        let blackout = MenuItem::with_id(
            "blackout",
            "Blackout uscita video",
            true,
            Some(Accelerator::new(Modifiers::META, Code::KeyB)),
        );
        let toggle_output = MenuItem::with_id(
            "toggle-output",
            "Apri / chiudi uscita video",
            true,
            Some(Accelerator::new(
                Modifiers::META | Modifiers::SHIFT,
                Code::KeyV,
            )),
        );
        let show_menu = submenu(
            "Spettacolo",
            &[
                &go,
                &pause,
                &stop,
                &skip,
                &back,
                &PredefinedMenuItem::separator(),
                &panic,
                &PredefinedMenuItem::separator(),
                &manual,
                &automatic,
                &suspend,
                &PredefinedMenuItem::separator(),
                &start_bg,
                &stop_bg,
                &PredefinedMenuItem::separator(),
                &blackout,
                &toggle_output,
            ],
        );

        let page_library = MenuItem::with_id(
            "page-library",
            "Libreria",
            true,
            Some(Accelerator::new(Modifiers::META, Code::Digit1)),
        );
        let page_timeline = MenuItem::with_id(
            "page-timeline",
            "Scaletta",
            true,
            Some(Accelerator::new(Modifiers::META, Code::Digit2)),
        );
        let page_titles = MenuItem::with_id(
            "page-titles",
            "Titoli",
            true,
            Some(Accelerator::new(Modifiers::META, Code::Digit3)),
        );
        let page_live = MenuItem::with_id(
            "page-live",
            "Live",
            true,
            Some(Accelerator::new(Modifiers::META, Code::Digit4)),
        );
        let page_settings = MenuItem::with_id(
            "page-settings",
            "Impostazioni",
            true,
            Some(Accelerator::new(Modifiers::META, Code::Digit5)),
        );
        let fullscreen = PredefinedMenuItem::fullscreen(None);
        let view_menu = submenu(
            "Vista",
            &[
                &page_library,
                &page_timeline,
                &page_titles,
                &page_live,
                &page_settings,
                &PredefinedMenuItem::separator(),
                &fullscreen,
            ],
        );

        let minimize = PredefinedMenuItem::minimize(None);
        let maximize = PredefinedMenuItem::maximize(None);
        let front = PredefinedMenuItem::bring_all_to_front(None);
        let window_menu = submenu(
            "Finestra",
            &[
                &minimize,
                &maximize,
                &PredefinedMenuItem::separator(),
                &front,
            ],
        );

        let shortcuts = MenuItem::with_id("shortcuts", "Scorciatoie da tastiera", true, None);
        let data_folder = MenuItem::with_id("data-folder", "Apri cartella dati", true, None);
        let help_menu = submenu(
            "Aiuto",
            &[&shortcuts, &PredefinedMenuItem::separator(), &data_folder],
        );

        menu.append_items(&[
            &app_menu,
            &file_menu,
            &edit_menu,
            &show_menu,
            &view_menu,
            &window_menu,
            &help_menu,
        ])?;
        menu.init_for_nsapp();
        Ok(menu)
    }

    /// Drains pending menu activations. Returns stable string ids.
    pub fn poll(&self) -> Vec<String> {
        let mut out = Vec::new();
        while let Ok(event) = MenuEvent::receiver().try_recv() {
            out.push(event.id.0);
        }
        out
    }
}

fn accel(modifiers: Modifiers, key: Code) -> Option<Accelerator> {
    Some(Accelerator::new(modifiers, key))
}

fn submenu(title: &str, items: &[&dyn muda::IsMenuItem]) -> Submenu {
    Submenu::with_items(title, true, items).unwrap_or_else(|_| Submenu::new(title, true))
}
