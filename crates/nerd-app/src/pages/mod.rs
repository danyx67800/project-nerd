//! Workspace pages.

pub mod library;
pub mod live;
pub mod settings;
pub mod timeline;
pub mod titles;

use eframe::egui::Ui;

use crate::app::NerdApp;
use crate::state::Page;

pub fn show(app: &mut NerdApp, ui: &mut Ui) {
    match app.page {
        Page::Library => library::show(app, ui),
        Page::Timeline => timeline::show(app, ui),
        Page::Titles => titles::show(app, ui),
        Page::Live => live::show(app, ui),
        Page::Settings => settings::show(app, ui),
    }
}
