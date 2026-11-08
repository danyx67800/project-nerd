//! Runtime system appearance.
//!
//! The design system never hard-codes the accent. macOS exposes the user's
//! accent and accessibility preferences through AppKit; on non-macOS builds the
//! neutral fallback keeps the library testable.

use egui::Color32;

use crate::tokens::SystemAppearance;

#[cfg(target_os = "macos")]
pub fn detect() -> SystemAppearance {
    use objc2_app_kit::{NSColor, NSColorSpace, NSWorkspace};

    let accent = NSColor::controlAccentColor();
    let space = NSColorSpace::sRGBColorSpace();
    let accent = accent
        .colorUsingColorSpace(&space)
        .map(|color| {
            Color32::from_rgb(
                (color.redComponent().clamp(0.0, 1.0) * 255.0).round() as u8,
                (color.greenComponent().clamp(0.0, 1.0) * 255.0).round() as u8,
                (color.blueComponent().clamp(0.0, 1.0) * 255.0).round() as u8,
            )
        })
        .unwrap_or_else(|| Color32::from_rgb(10, 132, 255));

    let workspace = NSWorkspace::sharedWorkspace();
    SystemAppearance {
        dark: true,
        accent,
        reduce_motion: workspace.accessibilityDisplayShouldReduceMotion(),
        increase_contrast: workspace.accessibilityDisplayShouldIncreaseContrast(),
        reduce_transparency: workspace.accessibilityDisplayShouldReduceTransparency(),
    }
}

#[cfg(not(target_os = "macos"))]
pub fn detect() -> SystemAppearance {
    SystemAppearance::default()
}

/// Uses egui's system theme signal when available, otherwise the macOS signal.
pub fn detect_with_egui(ctx: &egui::Context) -> SystemAppearance {
    let mut appearance = detect();
    if let Some(theme) = ctx.input(|input| input.raw.system_theme) {
        appearance.dark = theme == egui::Theme::Dark;
    }
    appearance
}
