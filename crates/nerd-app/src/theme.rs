//! Compatibility bridge between the application and `nerd-ui-kit`.
//!
//! All visual decisions live in the design-system crate. This module only maps
//! the persisted appearance preference and keeps the legacy `Palette` name for
//! code that has not moved to the new shell yet.

use nerd_core::prefs::ThemeMode;
use nerd_ui_kit::{system, Appearance, SystemAppearance};

pub use nerd_ui_kit::Theme as Palette;

pub fn appearance(mode: ThemeMode) -> Appearance {
    match mode {
        ThemeMode::Dark => Appearance::Dark,
        ThemeMode::Light => Appearance::Light,
        ThemeMode::System => Appearance::System,
    }
}

pub fn install(ctx: &egui::Context, mode: ThemeMode) -> Palette {
    let system = system::detect_with_egui(ctx);
    install_with_system(ctx, mode, system)
}

pub fn install_with_system(
    ctx: &egui::Context,
    mode: ThemeMode,
    system_appearance: SystemAppearance,
) -> Palette {
    nerd_ui_kit::install(ctx, appearance(mode), system_appearance)
}

pub fn card_frame(palette: Palette) -> egui::Frame {
    egui::Frame::new()
        .fill(palette.colors.card)
        .inner_margin(egui::Margin::same(16))
        .stroke(egui::Stroke::new(1.0, palette.colors.separator))
        .corner_radius(egui::CornerRadius::same(11))
}
