//! egui style installation for the design system.

use egui::{
    Color32, CornerRadius, FontFamily, FontId, Stroke, Style, TextStyle, Theme as EguiTheme,
    Visuals,
};

use crate::tokens::{Appearance, Theme};

/// Installs the tokens and a clean egui style.
///
/// Pages still use the kit's components for all visible controls. This style
/// only makes native fallbacks (text editing, menus, tooltips and scrollbars)
/// consistent with the design system.
pub fn install(
    ctx: &egui::Context,
    appearance: Appearance,
    system: crate::tokens::SystemAppearance,
) -> Theme {
    let theme = Theme::mode(appearance, system);
    let egui_theme = if theme.dark {
        EguiTheme::Dark
    } else {
        EguiTheme::Light
    };
    ctx.set_theme(egui_theme);

    let mut style = (*ctx.style_of(egui_theme)).clone();
    style.visuals = if theme.dark {
        Visuals::dark()
    } else {
        Visuals::light()
    };
    let c = theme.colors;
    style.visuals.panel_fill = c.panel;
    style.visuals.window_fill = c.panel;
    style.visuals.extreme_bg_color = c.well;
    style.visuals.faint_bg_color = c.window;
    style.visuals.override_text_color = Some(c.text);
    style.visuals.hyperlink_color = c.accent;
    style.visuals.selection.bg_fill = c.accent_soft;
    style.visuals.selection.stroke = Stroke::new(1.0, c.accent);
    style.visuals.window_stroke = Stroke::new(1.0, c.separator_strong);
    style.visuals.window_corner_radius = CornerRadius::same(12);
    style.visuals.menu_corner_radius = CornerRadius::same(8);
    style.visuals.popup_shadow = egui::epaint::Shadow {
        offset: [0, 8],
        blur: 24,
        spread: 0,
        color: c.shadow,
    };
    style.visuals.window_shadow = egui::epaint::Shadow {
        offset: [0, 12],
        blur: 32,
        spread: 0,
        color: c.shadow,
    };
    style.visuals.widgets.noninteractive.bg_fill = c.panel;
    style.visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, c.separator);
    style.visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, c.text);
    style.visuals.widgets.inactive.bg_fill = c.card;
    style.visuals.widgets.inactive.weak_bg_fill = c.card;
    style.visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, c.separator);
    style.visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, c.text);
    style.visuals.widgets.hovered.bg_fill = c.hover;
    style.visuals.widgets.hovered.weak_bg_fill = c.hover;
    style.visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, c.separator_strong);
    style.visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, c.text);
    style.visuals.widgets.active.bg_fill = c.pressed;
    style.visuals.widgets.active.weak_bg_fill = c.pressed;
    style.visuals.widgets.active.bg_stroke = Stroke::new(1.5, c.accent);
    style.visuals.widgets.active.fg_stroke = Stroke::new(1.0, c.text);
    style.visuals.widgets.open.bg_fill = c.card;
    style.visuals.widgets.open.weak_bg_fill = c.card;
    style.visuals.widgets.open.bg_stroke = Stroke::new(1.0, c.separator_strong);
    style.visuals.widgets.open.fg_stroke = Stroke::new(1.0, c.text);

    style.spacing.item_spacing = egui::vec2(8.0, 8.0);
    style.spacing.button_padding = egui::vec2(12.0, 6.0);
    style.spacing.interact_size = egui::vec2(32.0, 32.0);
    style.spacing.slider_width = 180.0;
    style.spacing.combo_width = 180.0;
    style.spacing.indent = 16.0;
    style.spacing.scroll = egui::style::ScrollStyle::solid();
    style.spacing.scroll.bar_width = 5.0;
    style.spacing.scroll.floating = true;
    style.animation_time = theme.motion_time();

    style.text_styles.insert(
        TextStyle::Heading,
        FontId::new(22.0, FontFamily::Proportional),
    );
    style
        .text_styles
        .insert(TextStyle::Body, FontId::new(13.0, FontFamily::Proportional));
    style.text_styles.insert(
        TextStyle::Button,
        FontId::new(13.0, FontFamily::Proportional),
    );
    style.text_styles.insert(
        TextStyle::Small,
        FontId::new(11.0, FontFamily::Proportional),
    );
    style.text_styles.insert(
        TextStyle::Monospace,
        FontId::new(12.0, FontFamily::Monospace),
    );

    let other = if egui_theme == EguiTheme::Dark {
        EguiTheme::Light
    } else {
        EguiTheme::Dark
    };
    ctx.set_style_of(egui_theme, style.clone());
    ctx.set_style_of(other, style);
    theme
}

/// A style that can be scoped to a child UI (used by the gallery to show both
/// appearances at once without changing the application theme).
#[allow(clippy::field_reassign_with_default)]
pub fn style_for(theme: &Theme) -> Style {
    let mut style = Style::default();
    style.visuals = if theme.dark {
        Visuals::dark()
    } else {
        Visuals::light()
    };
    style.visuals.panel_fill = theme.colors.panel;
    style.visuals.window_fill = theme.colors.panel;
    style.visuals.extreme_bg_color = theme.colors.well;
    style.visuals.override_text_color = Some(theme.colors.text);
    style.visuals.selection.bg_fill = theme.colors.accent_soft;
    style.visuals.selection.stroke = Stroke::new(1.0, theme.colors.accent);
    style.visuals.window_corner_radius = CornerRadius::same(12);
    style.animation_time = theme.motion_time();
    style
}

pub fn dark_theme(system: crate::tokens::SystemAppearance) -> Theme {
    Theme::new(true, system)
}

pub fn light_theme(system: crate::tokens::SystemAppearance) -> Theme {
    Theme::new(false, system)
}

pub fn text_color(theme: &Theme, alpha: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(
        theme.colors.text.r(),
        theme.colors.text.g(),
        theme.colors.text.b(),
        alpha,
    )
}
