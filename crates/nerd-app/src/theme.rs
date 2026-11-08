//! Workspace theme and runtime SF font loading.
//!
//! The SF font is read from the operating system at startup and is never
//! bundled in the repository.

use std::sync::Arc;

use egui::{Color32, FontData, FontDefinitions, FontFamily, FontId, TextStyle, Theme, Visuals};
use nerd_core::prefs::ThemeMode;

#[derive(Debug, Clone, Copy)]
pub struct Palette {
    pub background: Color32,
    pub panel: Color32,
    pub panel_alt: Color32,
    pub raised: Color32,
    pub border: Color32,
    pub text: Color32,
    pub text_muted: Color32,
    pub accent: Color32,
    pub accent_soft: Color32,
    pub warning: Color32,
    pub danger: Color32,
    pub success: Color32,
    pub video: Color32,
    pub title: Color32,
    pub blackout: Color32,
}

impl Palette {
    pub fn dark() -> Self {
        Self {
            background: Color32::from_rgb(13, 14, 17),
            panel: Color32::from_rgb(22, 24, 29),
            panel_alt: Color32::from_rgb(18, 20, 24),
            raised: Color32::from_rgb(30, 33, 40),
            border: Color32::from_rgb(44, 48, 58),
            text: Color32::from_rgb(236, 238, 243),
            text_muted: Color32::from_rgb(143, 149, 161),
            accent: Color32::from_rgb(70, 217, 191),
            accent_soft: Color32::from_rgb(28, 68, 63),
            warning: Color32::from_rgb(255, 184, 84),
            danger: Color32::from_rgb(255, 96, 96),
            success: Color32::from_rgb(103, 214, 141),
            video: Color32::from_rgb(145, 132, 255),
            title: Color32::from_rgb(97, 176, 255),
            blackout: Color32::from_rgb(5, 6, 8),
        }
    }

    pub fn light() -> Self {
        Self {
            background: Color32::from_rgb(243, 244, 247),
            panel: Color32::from_rgb(252, 252, 254),
            panel_alt: Color32::from_rgb(238, 240, 244),
            raised: Color32::from_rgb(255, 255, 255),
            border: Color32::from_rgb(210, 214, 222),
            text: Color32::from_rgb(25, 27, 32),
            text_muted: Color32::from_rgb(104, 110, 122),
            accent: Color32::from_rgb(0, 145, 124),
            accent_soft: Color32::from_rgb(210, 242, 235),
            warning: Color32::from_rgb(176, 110, 0),
            danger: Color32::from_rgb(198, 44, 44),
            success: Color32::from_rgb(25, 132, 70),
            video: Color32::from_rgb(88, 75, 190),
            title: Color32::from_rgb(30, 100, 180),
            blackout: Color32::from_rgb(9, 10, 12),
        }
    }
}

pub fn install(ctx: &egui::Context, mode: ThemeMode) -> Palette {
    install_fonts(ctx);
    let system_dark = ctx
        .input(|input| input.raw.system_theme)
        .map(|theme| theme == Theme::Dark)
        .unwrap_or(true);
    let dark = match mode {
        ThemeMode::Dark => true,
        ThemeMode::Light => false,
        ThemeMode::System => system_dark,
    };
    let palette = if dark {
        Palette::dark()
    } else {
        Palette::light()
    };

    let theme = if dark { Theme::Dark } else { Theme::Light };
    ctx.set_theme(theme);
    let mut style = (*ctx.style_of(theme)).clone();
    style.visuals = if dark {
        Visuals::dark()
    } else {
        Visuals::light()
    };
    style.visuals.panel_fill = palette.panel;
    style.visuals.window_fill = palette.panel;
    style.visuals.extreme_bg_color = palette.background;
    style.visuals.faint_bg_color = palette.panel_alt;
    style.visuals.override_text_color = Some(palette.text);
    style.visuals.hyperlink_color = palette.accent;
    style.visuals.selection.bg_fill = palette.accent.gamma_multiply(0.35);
    style.visuals.selection.stroke = egui::Stroke::new(1.0, palette.accent);
    style.visuals.widgets.noninteractive.bg_fill = palette.panel;
    style.visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, palette.border);
    style.visuals.widgets.inactive.bg_fill = palette.raised;
    style.visuals.widgets.inactive.weak_bg_fill = palette.raised;
    style.visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, palette.border);
    style.visuals.widgets.hovered.bg_fill = palette.raised.gamma_multiply(1.15);
    style.visuals.widgets.hovered.weak_bg_fill = palette.raised.gamma_multiply(1.15);
    style.visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, palette.accent);
    style.visuals.widgets.active.bg_fill = palette.accent_soft;
    style.visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0, palette.accent);
    style.visuals.widgets.open.bg_fill = palette.raised;
    style.visuals.window_stroke = egui::Stroke::new(1.0, palette.border);
    style.visuals.window_corner_radius = egui::CornerRadius::same(12);
    style.visuals.menu_corner_radius = egui::CornerRadius::same(10);

    style.spacing.item_spacing = egui::vec2(8.0, 8.0);
    style.spacing.button_padding = egui::vec2(12.0, 7.0);
    style.spacing.interact_size = egui::vec2(34.0, 28.0);
    style.spacing.slider_width = 180.0;
    style.spacing.combo_width = 220.0;
    style.spacing.indent = 18.0;
    style.spacing.scroll = egui::style::ScrollStyle::solid();

    style.text_styles.insert(
        TextStyle::Heading,
        FontId::new(24.0, FontFamily::Proportional),
    );
    style
        .text_styles
        .insert(TextStyle::Body, FontId::new(14.0, FontFamily::Proportional));
    style.text_styles.insert(
        TextStyle::Button,
        FontId::new(14.0, FontFamily::Proportional),
    );
    style.text_styles.insert(
        TextStyle::Small,
        FontId::new(11.5, FontFamily::Proportional),
    );
    style.text_styles.insert(
        TextStyle::Monospace,
        FontId::new(12.5, FontFamily::Monospace),
    );

    ctx.set_style_of(theme, style);
    palette
}

fn install_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    let mut loaded = false;
    if let Ok(bytes) = std::fs::read("/System/Library/Fonts/SFNS.ttf") {
        fonts
            .font_data
            .insert("sf".to_string(), Arc::new(FontData::from_owned(bytes)));
        if let Some(family) = fonts.families.get_mut(&FontFamily::Proportional) {
            family.insert(0, "sf".to_string());
        }
        loaded = true;
    }
    if let Ok(bytes) = std::fs::read("/System/Library/Fonts/SFNSMono.ttf") {
        fonts
            .font_data
            .insert("sf_mono".to_string(), Arc::new(FontData::from_owned(bytes)));
        if let Some(family) = fonts.families.get_mut(&FontFamily::Monospace) {
            family.insert(0, "sf_mono".to_string());
        }
    }
    ctx.set_fonts(fonts);
    if !loaded {
        tracing::warn!("SF font not found, using egui default fonts");
    }
}

/// A reusable card frame with the editor look.
pub fn card_frame(palette: Palette) -> egui::Frame {
    egui::Frame::new()
        .fill(palette.panel_alt)
        .inner_margin(egui::Margin::same(16))
        .stroke(egui::Stroke::new(1.0, palette.border))
        .corner_radius(egui::CornerRadius::same(12))
}
