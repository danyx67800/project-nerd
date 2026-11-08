//! Compatibility wrappers around `nerd-ui-kit` components.
//!
//! New page code uses the kit directly. These helpers keep the remaining shell
//! dialogs readable while they migrate.

use eframe::egui::{self, Color32, Response, RichText, Ui};
use nerd_core::model::{PlaybackMode, SongStatus};

use crate::theme::Palette;

pub fn accent_button(ui: &mut Ui, palette: Palette, text: impl Into<String>) -> Response {
    nerd_ui_kit::Button::new(text)
        .variant(nerd_ui_kit::ButtonVariant::Primary)
        .show(ui, &palette)
}

pub fn danger_button(ui: &mut Ui, palette: Palette, text: impl Into<String>) -> Response {
    nerd_ui_kit::Button::new(text)
        .variant(nerd_ui_kit::ButtonVariant::Destructive)
        .show(ui, &palette)
}

pub fn ghost_button(ui: &mut Ui, palette: Palette, text: impl Into<String>) -> Response {
    nerd_ui_kit::Button::new(text)
        .variant(nerd_ui_kit::ButtonVariant::Secondary)
        .show(ui, &palette)
}

pub fn pill(ui: &mut Ui, text: impl Into<String>, color: Color32) -> Response {
    let theme = Palette::new(true, nerd_ui_kit::SystemAppearance::default());
    nerd_ui_kit::status_pill(ui, &theme, &text.into(), color, None)
}

pub fn section_label(ui: &mut Ui, palette: Palette, text: &str) {
    nerd_ui_kit::section_label(ui, &palette, text);
}

pub fn empty_state(ui: &mut Ui, palette: Palette, title: &str, body: &str) {
    nerd_ui_kit::empty_state(
        ui,
        &palette,
        nerd_ui_kit::Icon::Music,
        title,
        body,
        |_ui| {},
    );
}

pub fn waveform(
    painter: &egui::Painter,
    rect: egui::Rect,
    peaks: &[f32],
    color: Color32,
    played: Option<f32>,
) {
    nerd_ui_kit::waveform(painter, rect, peaks, color, played);
}

pub fn vu_meter(ui: &mut Ui, palette: Palette, value: f32, label: &str, _color: Color32) {
    let _ = nerd_ui_kit::vu_meter(ui, &palette, label, value, value, value, value);
}

pub fn status_color(palette: Palette, status: SongStatus) -> Color32 {
    match status {
        SongStatus::Pending => palette.colors.text_secondary,
        SongStatus::Live => palette.colors.green,
        SongStatus::Done => palette.colors.green,
        SongStatus::Skipped => palette.colors.amber,
    }
}

pub fn status_label(status: SongStatus) -> &'static str {
    match status {
        SongStatus::Pending => "In attesa",
        SongStatus::Live => "In onda",
        SongStatus::Done => "Fatto",
        SongStatus::Skipped => "Saltato",
    }
}

pub fn mode_label(mode: PlaybackMode) -> &'static str {
    match mode {
        PlaybackMode::Manual => crate::strings::MODE_MANUAL,
        PlaybackMode::Automatic => crate::strings::MODE_AUTO,
    }
}

pub fn divider(ui: &mut Ui, palette: Palette) {
    nerd_ui_kit::divider(ui, &palette);
}

pub fn shortcut_chip(ui: &mut Ui, palette: Palette, key: &str, action: &str) {
    ui.horizontal(|ui| {
        nerd_ui_kit::keycap(ui, &palette, key);
        ui.label(
            RichText::new(action)
                .size(11.0)
                .color(palette.colors.text_secondary),
        );
    });
}
