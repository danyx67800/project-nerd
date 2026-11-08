//! Project Nerd — macOS application entry point.

// Legacy panes are retained behind the new shell while menu commands migrate.
#![allow(dead_code)]
#![cfg_attr(all(not(debug_assertions), target_os = "macos"), allow(unused))]

mod app;
mod crash;
mod demo;
mod logging;
mod media;
mod menu;
mod output;
mod pages;
mod paths;
mod platform;
mod shell;
mod state;
mod strings;
mod theme;
mod update;
mod video_player;
mod widgets;

use std::path::PathBuf;

use nerd_core::prefs::ThemeMode;
use nerd_core::{APP_ID, APP_NAME};

use crate::state::Page;

struct Flags {
    options: app::LaunchOptions,
    size: [f32; 2],
}

fn main() -> eframe::Result {
    let flags = parse_flags();
    let paths = if flags.options.demo && std::env::var_os("PROJECT_NERD_SUPPORT").is_none() {
        paths::AppPaths::demo()
    } else {
        paths::AppPaths::resolve()
    };
    if let Err(err) = paths.ensure() {
        eprintln!("Impossibile creare la cartella dati: {err}");
    }
    let _log_guard = logging::init(&paths);
    crash::install(paths.support.clone());

    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title(APP_NAME)
            .with_app_id(APP_ID)
            .with_inner_size(flags.size)
            .with_min_inner_size([1100.0, 700.0])
            .with_fullsize_content_view(true)
            .with_titlebar_shown(false)
            .with_title_shown(false),
        ..Default::default()
    };

    eframe::run_native(
        APP_NAME,
        options,
        Box::new(move |cc| Ok(Box::new(app::NerdApp::new(cc, paths, flags.options)))),
    )
}

fn parse_flags() -> Flags {
    let mut options = app::LaunchOptions {
        initial: None,
        demo: false,
        gallery: false,
        page: None,
        appearance: None,
        screenshot: None,
    };
    let mut size = [1500.0, 940.0];
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--demo" => options.demo = true,
            "--gallery" => options.gallery = true,
            value if value.starts_with("--page=") => {
                options.page = Some(page_from_name(value.trim_start_matches("--page=")));
            }
            value if value.starts_with("--theme=") => {
                options.appearance = Some(match value.trim_start_matches("--theme=") {
                    "light" | "chiaro" => ThemeMode::Light,
                    "system" | "sistema" => ThemeMode::System,
                    _ => ThemeMode::Dark,
                });
            }
            value if value.starts_with("--screenshot=") => {
                options.screenshot = Some(PathBuf::from(value.trim_start_matches("--screenshot=")));
            }
            value if value.starts_with("--size=") => {
                if let Some((w, h)) = value.trim_start_matches("--size=").split_once('x') {
                    if let (Ok(w), Ok(h)) = (w.parse::<f32>(), h.parse::<f32>()) {
                        size = [w, h];
                    }
                }
            }
            value if !value.starts_with('-') => options.initial = Some(PathBuf::from(value)),
            _ => {}
        }
    }
    Flags { options, size }
}

fn page_from_name(name: &str) -> Page {
    match name.to_ascii_lowercase().as_str() {
        "scaletta" | "timeline" => Page::Timeline,
        "titoli" | "titles" => Page::Titles,
        "live" => Page::Live,
        "impostazioni" | "settings" => Page::Settings,
        "galleria" | "gallery" => Page::Gallery,
        _ => Page::Library,
    }
}
