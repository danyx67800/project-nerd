//! Project Nerd — macOS application entry point.

#![cfg_attr(all(not(debug_assertions), target_os = "macos"), allow(unused))]

mod app;
mod crash;
mod logging;
mod media;
mod menu;
mod output;
mod pages;
mod paths;
mod platform;
mod state;
mod strings;
mod theme;
mod update;
mod video_player;
mod widgets;

use std::path::PathBuf;

use nerd_core::{APP_ID, APP_NAME};

fn main() -> eframe::Result {
    let paths = paths::AppPaths::resolve();
    if let Err(err) = paths.ensure() {
        eprintln!("Impossibile creare la cartella dati: {err}");
    }
    let _log_guard = logging::init(&paths);
    crash::install(paths.support.clone());

    let initial = std::env::args()
        .skip(1)
        .find(|arg| !arg.starts_with('-'))
        .map(PathBuf::from);

    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title(APP_NAME)
            .with_app_id(APP_ID)
            .with_inner_size([1500.0, 940.0])
            .with_min_inner_size([1120.0, 720.0]),
        ..Default::default()
    };

    eframe::run_native(
        APP_NAME,
        options,
        Box::new(move |cc| Ok(Box::new(app::NerdApp::new(cc, paths, initial)))),
    )
}
