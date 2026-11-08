//! Semantic tokens shared by every screen.
//!
//! The palette is intentionally small: four neutral surfaces, one system
//! accent and four fixed status colours (green, mint, amber, red). All colours
//! are opaque except separators and pressed tints, which keeps text contrast
//! predictable in both appearances.

use egui::{Color32, FontFamily, FontId};

/// Appearance requested by the user.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Appearance {
    #[default]
    Dark,
    Light,
    System,
}

/// System-wide accessibility and accent information.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SystemAppearance {
    pub dark: bool,
    pub accent: Color32,
    pub reduce_motion: bool,
    pub increase_contrast: bool,
    pub reduce_transparency: bool,
}

impl Default for SystemAppearance {
    fn default() -> Self {
        Self {
            dark: true,
            accent: Color32::from_rgb(10, 132, 255),
            reduce_motion: false,
            increase_contrast: false,
            reduce_transparency: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Colors {
    /// Darkest surface: viewer canvas, timeline well, audio waveform bed.
    pub well: Color32,
    /// Main window background behind panels.
    pub window: Color32,
    /// Sidebar, inspector, toolbar and page chrome.
    pub panel: Color32,
    /// Cards, fields, rows and popovers.
    pub card: Color32,
    pub hover: Color32,
    pub pressed: Color32,
    pub popover: Color32,
    pub separator: Color32,
    pub separator_strong: Color32,
    pub text: Color32,
    pub text_secondary: Color32,
    pub text_tertiary: Color32,
    pub accent: Color32,
    pub accent_hover: Color32,
    pub accent_soft: Color32,
    pub on_accent: Color32,
    pub green: Color32,
    pub mint: Color32,
    pub amber: Color32,
    pub red: Color32,
    pub video: Color32,
    pub title: Color32,
    pub blackout: Color32,
    pub shadow: Color32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Metrics {
    pub titlebar: f32,
    pub pagebar: f32,
    pub sidebar_min: f32,
    pub sidebar_max: f32,
    pub inspector_min: f32,
    pub inspector_max: f32,
    pub control_sm: f32,
    pub control: f32,
    pub control_lg: f32,
    pub row: f32,
    pub live_hit: f32,
}

impl Default for Metrics {
    fn default() -> Self {
        Self {
            titlebar: 52.0,
            pagebar: 56.0,
            sidebar_min: 220.0,
            sidebar_max: 420.0,
            inspector_min: 260.0,
            inspector_max: 440.0,
            control_sm: 28.0,
            control: 32.0,
            control_lg: 40.0,
            row: 44.0,
            live_hit: 56.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Theme {
    pub dark: bool,
    pub high_contrast: bool,
    pub reduce_motion: bool,
    pub reduce_transparency: bool,
    pub colors: Colors,
    pub metrics: Metrics,
}

impl Theme {
    pub fn new(dark: bool, system: SystemAppearance) -> Self {
        let high_contrast = system.increase_contrast;
        let colors = if dark {
            dark_colors(system.accent, high_contrast)
        } else {
            light_colors(system.accent, high_contrast)
        };
        Self {
            dark,
            high_contrast,
            reduce_motion: system.reduce_motion,
            reduce_transparency: system.reduce_transparency,
            colors,
            metrics: Metrics::default(),
        }
    }

    pub fn mode(appearance: Appearance, system: SystemAppearance) -> Self {
        let dark = match appearance {
            Appearance::Dark => true,
            Appearance::Light => false,
            Appearance::System => system.dark,
        };
        Self::new(dark, system)
    }

    pub fn motion_time(&self) -> f32 {
        if self.reduce_motion {
            0.0
        } else {
            0.14
        }
    }

    /// A light neutral used on top of the accent, chosen from accent luminance.
    pub fn on_accent(&self) -> Color32 {
        self.colors.on_accent
    }

    pub fn row_fill(&self, selected: bool, hovered: bool) -> Color32 {
        if selected {
            self.colors.accent_soft
        } else if hovered {
            self.colors.hover
        } else {
            Color32::TRANSPARENT
        }
    }

    pub fn row_stroke(&self, selected: bool) -> egui::Stroke {
        if selected {
            egui::Stroke::new(
                if self.high_contrast { 2.0 } else { 1.0 },
                self.colors.accent,
            )
        } else {
            egui::Stroke::NONE
        }
    }

    pub fn status_color(&self, status: StatusColor) -> Color32 {
        match status {
            StatusColor::Neutral => self.colors.text_secondary,
            StatusColor::Accent => self.colors.accent,
            StatusColor::Green => self.colors.green,
            StatusColor::Mint => self.colors.mint,
            StatusColor::Amber => self.colors.amber,
            StatusColor::Red => self.colors.red,
            StatusColor::Video => self.colors.video,
            StatusColor::Title => self.colors.title,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusColor {
    Neutral,
    Accent,
    Green,
    Mint,
    Amber,
    Red,
    Video,
    Title,
}

fn dark_colors(accent: Color32, high_contrast: bool) -> Colors {
    let text = if high_contrast {
        Color32::WHITE
    } else {
        Color32::from_rgb(235, 235, 240)
    };
    let secondary = if high_contrast {
        Color32::from_rgb(210, 210, 218)
    } else {
        Color32::from_rgb(174, 174, 184)
    };
    let tertiary = if high_contrast {
        Color32::from_rgb(178, 178, 188)
    } else {
        Color32::from_rgb(138, 138, 148)
    };
    Colors {
        well: Color32::from_rgb(18, 18, 20),
        window: Color32::from_rgb(30, 30, 32),
        panel: Color32::from_rgb(36, 36, 39),
        card: Color32::from_rgb(42, 42, 45),
        hover: Color32::from_rgb(52, 52, 56),
        pressed: Color32::from_rgb(60, 60, 65),
        popover: Color32::from_rgb(48, 48, 52),
        separator: Color32::from_rgba_unmultiplied(255, 255, 255, 23),
        separator_strong: Color32::from_rgba_unmultiplied(255, 255, 255, 42),
        text,
        text_secondary: secondary,
        text_tertiary: tertiary,
        accent,
        accent_hover: accent.gamma_multiply(1.12),
        accent_soft: Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), 38),
        on_accent: on_accent(accent),
        green: Color32::from_rgb(74, 220, 128),
        mint: Color32::from_rgb(96, 224, 196),
        amber: Color32::from_rgb(255, 190, 82),
        red: Color32::from_rgb(255, 96, 102),
        video: Color32::from_rgb(112, 168, 255),
        title: Color32::from_rgb(196, 156, 255),
        blackout: Color32::from_rgb(6, 6, 8),
        shadow: Color32::from_rgba_unmultiplied(0, 0, 0, 110),
    }
}

fn light_colors(accent: Color32, high_contrast: bool) -> Colors {
    let text = if high_contrast {
        Color32::BLACK
    } else {
        Color32::from_rgb(29, 29, 31)
    };
    let secondary = if high_contrast {
        Color32::from_rgb(40, 40, 44)
    } else {
        Color32::from_rgb(84, 84, 90)
    };
    let tertiary = if high_contrast {
        Color32::from_rgb(62, 62, 68)
    } else {
        Color32::from_rgb(108, 108, 116)
    };
    Colors {
        well: Color32::from_rgb(216, 216, 220),
        window: Color32::from_rgb(236, 236, 239),
        panel: Color32::from_rgb(245, 245, 247),
        card: Color32::from_rgb(255, 255, 255),
        hover: Color32::from_rgb(233, 233, 237),
        pressed: Color32::from_rgb(220, 220, 225),
        popover: Color32::from_rgb(255, 255, 255),
        separator: Color32::from_rgba_unmultiplied(0, 0, 0, 26),
        separator_strong: Color32::from_rgba_unmultiplied(0, 0, 0, 46),
        text,
        text_secondary: secondary,
        text_tertiary: tertiary,
        accent,
        accent_hover: accent.gamma_multiply(0.9),
        accent_soft: Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), 30),
        on_accent: on_accent(accent),
        green: Color32::from_rgb(18, 128, 58),
        mint: Color32::from_rgb(0, 122, 99),
        amber: Color32::from_rgb(145, 88, 0),
        red: Color32::from_rgb(196, 42, 48),
        video: Color32::from_rgb(30, 92, 200),
        title: Color32::from_rgb(106, 63, 196),
        blackout: Color32::from_rgb(6, 6, 8),
        shadow: Color32::from_rgba_unmultiplied(0, 0, 0, 34),
    }
}

fn on_accent(accent: Color32) -> Color32 {
    let luminance =
        0.2126 * srgb(accent.r()) + 0.7152 * srgb(accent.g()) + 0.0722 * srgb(accent.b());
    if luminance > 0.24 {
        Color32::from_rgb(10, 10, 12)
    } else {
        Color32::WHITE
    }
}

fn srgb(channel: u8) -> f32 {
    let c = channel as f32 / 255.0;
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// Spacing scale, aligned to a 4 pt grid.
pub const S1: f32 = 4.0;
pub const S2: f32 = 8.0;
pub const S3: f32 = 12.0;
pub const S4: f32 = 16.0;
pub const S5: f32 = 24.0;
pub const S6: f32 = 32.0;

/// Converts a semantic colour to a translucent fill without changing its hue.
pub fn tint(color: Color32, alpha: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), alpha)
}

pub fn hairline(theme: &Theme) -> egui::Stroke {
    egui::Stroke::new(1.0, theme.colors.separator)
}

pub fn page_title() -> FontId {
    FontId::new(22.0, FontFamily::Proportional)
}

pub fn section_title() -> FontId {
    FontId::new(15.0, FontFamily::Proportional)
}

pub fn body() -> FontId {
    FontId::new(13.0, FontFamily::Proportional)
}

pub fn secondary() -> FontId {
    FontId::new(12.0, FontFamily::Proportional)
}

pub fn caption() -> FontId {
    FontId::new(11.0, FontFamily::Proportional)
}

pub fn micro() -> FontId {
    FontId::new(11.0, FontFamily::Proportional)
}

pub fn live_title() -> FontId {
    FontId::new(44.0, FontFamily::Proportional)
}

pub fn live_countdown() -> FontId {
    FontId::new(96.0, FontFamily::Monospace)
}

pub fn timecode(size: f32) -> FontId {
    FontId::new(size, FontFamily::Monospace)
}
