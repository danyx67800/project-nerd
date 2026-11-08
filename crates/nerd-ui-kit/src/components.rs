//! Custom egui components built only from design-system tokens.

// Deterministic Area/id salts are still built with `Id::new`; egui 0.37's
// replacement is not accepted by every Area/Window API used by the kit.
#![allow(deprecated)]

use std::f32::consts::TAU;

use egui::{
    Align, Align2, Color32, Context, CornerRadius, FontId, Frame, Id, InnerResponse, Layout,
    Margin, Painter, Pos2, Rect, Response, RichText, ScrollArea, Sense, Stroke, StrokeKind,
    TextureHandle, Ui, Vec2,
};

use crate::icon::{icon, Icon};
use crate::tokens::{
    body, caption, micro, page_title, secondary, section_title, timecode, tint, Theme, S2,
};

pub fn lerp(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    Color32::from_rgba_unmultiplied(
        (a.r() as f32 + (b.r() as f32 - a.r() as f32) * t).round() as u8,
        (a.g() as f32 + (b.g() as f32 - a.g() as f32) * t).round() as u8,
        (a.b() as f32 + (b.b() as f32 - a.b() as f32) * t).round() as u8,
        (a.a() as f32 + (b.a() as f32 - a.a() as f32) * t).round() as u8,
    )
}

pub fn with_alpha(color: Color32, alpha: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), alpha)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonVariant {
    Primary,
    Secondary,
    Tinted,
    Plain,
    Destructive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlSize {
    Small,
    Regular,
    Large,
}

pub struct Button {
    label: String,
    icon: Option<Icon>,
    variant: ButtonVariant,
    size: ControlSize,
    keycap: Option<String>,
    min_width: f32,
    min_height: f32,
    enabled: bool,
    tooltip: Option<String>,
    shortcut: Option<String>,
}

impl Button {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            icon: None,
            variant: ButtonVariant::Secondary,
            size: ControlSize::Regular,
            keycap: None,
            min_width: 0.0,
            min_height: 0.0,
            enabled: true,
            tooltip: None,
            shortcut: None,
        }
    }

    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    pub fn keycap(mut self, key: impl Into<String>) -> Self {
        self.keycap = Some(key.into());
        self
    }

    pub fn min_width(mut self, width: f32) -> Self {
        self.min_width = width;
        self
    }

    pub fn min_height(mut self, height: f32) -> Self {
        self.min_height = height;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn tooltip(mut self, text: impl Into<String>) -> Self {
        self.tooltip = Some(text.into());
        self
    }

    pub fn shortcut(mut self, text: impl Into<String>) -> Self {
        self.shortcut = Some(text.into());
        self
    }

    pub fn show(self, ui: &mut Ui, theme: &Theme) -> Response {
        let height = self.min_height.max(match self.size {
            ControlSize::Small => theme.metrics.control_sm,
            ControlSize::Regular => theme.metrics.control,
            ControlSize::Large => theme.metrics.control_lg,
        });
        let font = match self.size {
            ControlSize::Small => body(),
            ControlSize::Regular => body(),
            ControlSize::Large => FontId::proportional(14.0),
        };
        let icon_size = match self.size {
            ControlSize::Small => 14.0,
            ControlSize::Regular => 16.0,
            ControlSize::Large => 18.0,
        };
        let pad = match self.size {
            ControlSize::Small => 8.0,
            ControlSize::Regular => 12.0,
            ControlSize::Large => 14.0,
        };
        let text_galley =
            ui.painter()
                .layout_no_wrap(self.label.clone(), font.clone(), theme.colors.text);
        let key_width = self.keycap.as_ref().map(|key| {
            ui.painter()
                .layout_no_wrap(key.clone(), micro(), theme.colors.text_secondary)
                .size()
                .x
                + 10.0
        });
        let content_width = (if self.icon.is_some() {
            icon_size + 7.0
        } else {
            0.0
        }) + text_galley.size().x
            + key_width.map(|w| w + S2).unwrap_or(0.0);
        let width = (content_width + pad * 2.0).max(self.min_width);
        let mut response = ui.add_sized(
            Vec2::new(width, height),
            egui::Button::new(RichText::new(&self.label).color(Color32::TRANSPARENT))
                .fill(Color32::TRANSPARENT)
                .stroke(Stroke::NONE)
                .frame(false)
                .min_size(Vec2::new(width, height)),
        );
        let rect = response.rect;

        let hover = ui.ctx().animate_bool_with_time(
            response.id.with("hover"),
            response.hovered() && self.enabled,
            theme.motion_time(),
        );
        let (base, hovered, border, text) = match self.variant {
            ButtonVariant::Primary => (
                theme.colors.accent,
                theme.colors.accent_hover,
                Color32::TRANSPARENT,
                theme.colors.on_accent,
            ),
            ButtonVariant::Secondary => (
                theme.colors.card,
                theme.colors.hover,
                theme.colors.separator_strong,
                theme.colors.text,
            ),
            ButtonVariant::Tinted => (
                theme.colors.accent_soft,
                tint(theme.colors.accent, 58),
                Color32::TRANSPARENT,
                theme.colors.accent,
            ),
            ButtonVariant::Plain => (
                Color32::TRANSPARENT,
                theme.colors.hover,
                Color32::TRANSPARENT,
                theme.colors.text,
            ),
            ButtonVariant::Destructive => (
                theme.colors.red,
                theme.colors.red.gamma_multiply(1.12),
                Color32::TRANSPARENT,
                Color32::WHITE,
            ),
        };
        let fill = if !self.enabled {
            with_alpha(base, 80)
        } else {
            lerp(base, hovered, hover)
        };
        let text = if self.enabled {
            text
        } else {
            theme.colors.text_tertiary
        };
        let painter = ui.painter();
        if fill.a() > 0 {
            painter.rect_filled(rect, CornerRadius::same(7), fill);
        }
        if border.a() > 0 {
            painter.rect_stroke(
                rect,
                CornerRadius::same(7),
                Stroke::new(1.0, border),
                StrokeKind::Inside,
            );
        }
        if response.has_focus() && self.enabled {
            painter.rect_stroke(
                rect.expand(1.5),
                CornerRadius::same(9),
                Stroke::new(2.0, theme.colors.accent),
                StrokeKind::Outside,
            );
        }
        let mut x = rect.left() + pad;
        if let Some(icon_kind) = self.icon {
            let icon_rect = Rect::from_center_size(
                Pos2::new(x + icon_size / 2.0, rect.center().y),
                Vec2::splat(icon_size),
            );
            icon(painter, icon_rect, icon_kind, text, 1.55);
            x += icon_size + 7.0;
        }
        let text_pos = Pos2::new(x, rect.center().y - text_galley.size().y / 2.0);
        painter.galley(text_pos, text_galley, text);
        if let Some(key) = &self.keycap {
            let key_width = key_width.unwrap_or(22.0);
            let key_rect = Rect::from_center_size(
                Pos2::new(rect.right() - pad - key_width / 2.0, rect.center().y),
                Vec2::new(key_width, 19.0),
            );
            painter.rect_filled(
                key_rect,
                CornerRadius::same(4),
                with_alpha(theme.colors.window, 120),
            );
            painter.rect_stroke(
                key_rect,
                CornerRadius::same(4),
                Stroke::new(1.0, with_alpha(theme.colors.separator_strong, 120)),
                StrokeKind::Inside,
            );
            painter.text(
                key_rect.center(),
                Align2::CENTER_CENTER,
                key,
                micro(),
                with_alpha(text, 210),
            );
        }

        if let Some(tooltip) = &self.tooltip {
            response = response.on_hover_ui(|ui| {
                tooltip_body(ui, theme, tooltip, self.shortcut.as_deref());
            });
        }
        response
    }
}

fn tooltip_body(ui: &mut Ui, theme: &Theme, text: &str, shortcut: Option<&str>) {
    Frame::new()
        .fill(theme.colors.popover)
        .stroke(Stroke::new(1.0, theme.colors.separator_strong))
        .corner_radius(CornerRadius::same(7))
        .inner_margin(Margin::symmetric(9, 6))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(text).size(11.0).color(theme.colors.text));
                if let Some(shortcut) = shortcut {
                    keycap(ui, theme, shortcut);
                }
            });
        });
}

pub fn button(ui: &mut Ui, theme: &Theme, label: impl Into<String>) -> Response {
    Button::new(label).show(ui, theme)
}

pub fn primary_button(ui: &mut Ui, theme: &Theme, label: impl Into<String>) -> Response {
    Button::new(label)
        .variant(ButtonVariant::Primary)
        .show(ui, theme)
}

pub fn tinted_button(ui: &mut Ui, theme: &Theme, label: impl Into<String>) -> Response {
    Button::new(label)
        .variant(ButtonVariant::Tinted)
        .show(ui, theme)
}

pub fn destructive_button(ui: &mut Ui, theme: &Theme, label: impl Into<String>) -> Response {
    Button::new(label)
        .variant(ButtonVariant::Destructive)
        .show(ui, theme)
}

pub fn icon_button(
    ui: &mut Ui,
    theme: &Theme,
    icon_kind: Icon,
    tooltip: Option<&str>,
    selected: bool,
) -> Response {
    let mut button = Button::new("")
        .icon(icon_kind)
        .variant(if selected {
            ButtonVariant::Tinted
        } else {
            ButtonVariant::Plain
        })
        .min_width(28.0)
        .min_height(28.0);
    if let Some(tooltip) = tooltip {
        button = button.tooltip(tooltip);
    }
    button.show(ui, theme)
}

pub fn keycap(ui: &mut Ui, theme: &Theme, key: &str) -> Response {
    let galley = ui
        .painter()
        .layout_no_wrap(key.to_string(), micro(), theme.colors.text);
    let width = (galley.size().x + 12.0).max(20.0);
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 18.0), Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, CornerRadius::same(4), theme.colors.card);
    painter.rect_stroke(
        rect,
        CornerRadius::same(4),
        Stroke::new(1.0, theme.colors.separator_strong),
        StrokeKind::Inside,
    );
    painter.galley(
        Pos2::new(
            rect.center().x - galley.size().x / 2.0,
            rect.center().y - galley.size().y / 2.0,
        ),
        galley,
        theme.colors.text,
    );
    response
}

pub fn toggle(ui: &mut Ui, theme: &Theme, value: &mut bool, label: &str) -> Response {
    let row_height = 26.0;
    let width = ui.available_width();
    let mut response = ui.add_sized(
        Vec2::new(width, row_height),
        egui::Button::new(RichText::new(label).color(Color32::TRANSPARENT))
            .fill(Color32::TRANSPARENT)
            .stroke(Stroke::NONE)
            .frame(false),
    );
    if response.clicked() {
        *value = !*value;
        response.mark_changed();
    }
    let rect = response.rect;
    let t =
        ui.ctx()
            .animate_bool_with_time(response.id.with("switch"), *value, theme.motion_time());
    let track = Rect::from_min_size(
        Pos2::new(rect.left(), rect.center().y - 11.0),
        Vec2::new(40.0, 22.0),
    );
    let painter = ui.painter();
    painter.rect_filled(
        track,
        CornerRadius::same(11),
        lerp(
            if theme.high_contrast {
                theme.colors.text_tertiary
            } else {
                theme.colors.separator_strong
            },
            theme.colors.accent,
            t,
        ),
    );
    let knob_x = track.left() + 11.0 + t * 18.0;
    painter.circle_filled(Pos2::new(knob_x, track.center().y), 8.0, Color32::WHITE);
    if response.has_focus() {
        painter.rect_stroke(
            track.expand(2.0),
            CornerRadius::same(13),
            Stroke::new(2.0, theme.colors.accent),
            StrokeKind::Outside,
        );
    }
    painter.text(
        Pos2::new(track.right() + 9.0, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        body(),
        theme.colors.text,
    );
    response
}

pub fn segmented(
    ui: &mut Ui,
    theme: &Theme,
    id: &str,
    options: &[&str],
    selected: &mut usize,
    min_width: f32,
) -> Response {
    let height = 30.0;
    let widths: Vec<f32> = options
        .iter()
        .map(|option| {
            ui.painter()
                .layout_no_wrap(option.to_string(), body(), theme.colors.text)
                .size()
                .x
                + 24.0
        })
        .collect();
    let width = widths.iter().sum::<f32>().max(min_width);
    let mut response = ui.add_sized(
        Vec2::new(width, height),
        egui::Button::new(RichText::new(options.join(" ")).color(Color32::TRANSPARENT))
            .fill(Color32::TRANSPARENT)
            .stroke(Stroke::NONE)
            .frame(false),
    );
    let rect = response.rect;
    let painter = ui.painter();
    painter.rect_filled(rect, CornerRadius::same(7), theme.colors.card);
    painter.rect_stroke(
        rect,
        CornerRadius::same(7),
        Stroke::new(1.0, theme.colors.separator),
        StrokeKind::Inside,
    );
    let pointer = response.interact_pointer_pos();
    let mut x = rect.left();
    for (index, option) in options.iter().enumerate() {
        let segment = Rect::from_min_size(
            Pos2::new(x, rect.top()),
            Vec2::new(widths[index], rect.height()),
        );
        let selected_here = index == *selected;
        let hovered = pointer.is_some_and(|p| segment.contains(p));
        if selected_here {
            painter.rect_filled(
                segment.shrink(2.0),
                CornerRadius::same(5),
                theme.colors.accent_soft,
            );
        } else if hovered {
            painter.rect_filled(
                segment.shrink(2.0),
                CornerRadius::same(5),
                theme.colors.hover,
            );
        }
        painter.text(
            segment.center(),
            Align2::CENTER_CENTER,
            option,
            body(),
            if selected_here {
                theme.colors.accent
            } else if hovered {
                theme.colors.text
            } else {
                theme.colors.text_secondary
            },
        );
        x += widths[index];
    }
    if response.has_focus() {
        painter.rect_stroke(
            rect.expand(1.5),
            CornerRadius::same(9),
            Stroke::new(2.0, theme.colors.accent),
            StrokeKind::Outside,
        );
    }
    if response.clicked() {
        if let Some(pointer) = pointer {
            let mut x = rect.left();
            for (index, width) in widths.iter().enumerate() {
                if pointer.x >= x && pointer.x <= x + width {
                    if *selected != index {
                        *selected = index;
                        response.mark_changed();
                    }
                    break;
                }
                x += width;
            }
        }
    }
    if response.has_focus() {
        let mut changed = false;
        ui.input(|input| {
            if input.key_pressed(egui::Key::ArrowLeft) {
                *selected = selected.saturating_sub(1);
                changed = true;
            }
            if input.key_pressed(egui::Key::ArrowRight) {
                *selected = (*selected + 1).min(options.len().saturating_sub(1));
                changed = true;
            }
        });
        if changed {
            response.mark_changed();
        }
    }
    let _ = id;
    response
}

pub fn search_field(ui: &mut Ui, theme: &Theme, value: &mut String, hint: &str) -> Response {
    let width = ui.available_width();
    let frame = Frame::new()
        .fill(theme.colors.card)
        .stroke(Stroke::new(1.0, theme.colors.separator))
        .corner_radius(CornerRadius::same(7))
        .inner_margin(Margin::symmetric(8, 4));
    let inner = frame.show(ui, |ui| {
        ui.set_width(width.max(80.0));
        ui.horizontal(|ui| {
            let (rect, _) = ui.allocate_exact_size(Vec2::splat(16.0), Sense::hover());
            icon(
                ui.painter(),
                rect,
                Icon::Search,
                theme.colors.text_tertiary,
                1.5,
            );
            let response = ui.add_sized(
                [ui.available_width(), 22.0],
                egui::TextEdit::singleline(value)
                    .hint_text(
                        RichText::new(hint)
                            .size(12.0)
                            .color(theme.colors.text_tertiary),
                    )
                    .frame(egui::Frame::new().fill(Color32::TRANSPARENT))
                    .text_color(theme.colors.text)
                    .font(body()),
            );
            if response.has_focus() {
                ui.painter().rect_stroke(
                    ui.max_rect().expand(2.0),
                    CornerRadius::same(7),
                    Stroke::new(2.0, theme.colors.accent),
                    StrokeKind::Outside,
                );
            }
            response
        })
        .inner
    });
    inner.inner
}

pub fn text_field(ui: &mut Ui, theme: &Theme, value: &mut String, hint: &str) -> Response {
    let frame = Frame::new()
        .fill(theme.colors.card)
        .stroke(Stroke::new(1.0, theme.colors.separator))
        .corner_radius(CornerRadius::same(6))
        .inner_margin(Margin::symmetric(8, 5));
    frame
        .show(ui, |ui| {
            ui.add_sized(
                [ui.available_width(), 20.0],
                egui::TextEdit::singleline(value)
                    .hint_text(
                        RichText::new(hint)
                            .size(12.0)
                            .color(theme.colors.text_tertiary),
                    )
                    .frame(egui::Frame::new().fill(Color32::TRANSPARENT))
                    .text_color(theme.colors.text)
                    .font(body()),
            )
        })
        .inner
}

pub fn bare_text_field(
    ui: &mut Ui,
    theme: &Theme,
    value: &mut String,
    hint: &str,
    width: f32,
    font: FontId,
) -> Response {
    ui.add_sized(
        [width, 22.0],
        egui::TextEdit::singleline(value)
            .hint_text(
                RichText::new(hint)
                    .size(font.size)
                    .color(theme.colors.text_tertiary),
            )
            .frame(egui::Frame::new().fill(Color32::TRANSPARENT))
            .text_color(theme.colors.text)
            .font(font)
            .margin(Margin::ZERO),
    )
}

pub fn field_label(ui: &mut Ui, theme: &Theme, label: &str) {
    ui.label(
        RichText::new(label)
            .size(11.0)
            .color(theme.colors.text_secondary),
    );
}

pub fn labeled_field(
    ui: &mut Ui,
    theme: &Theme,
    label: &str,
    value: &mut String,
    hint: &str,
) -> Response {
    field_label(ui, theme, label);
    ui.add_space(3.0);
    text_field(ui, theme, value, hint)
}

pub fn number_field(
    ui: &mut Ui,
    theme: &Theme,
    label: &str,
    value: &mut f64,
    range: std::ops::RangeInclusive<f64>,
    suffix: &str,
    speed: f64,
) -> Response {
    field_label(ui, theme, label);
    ui.add_space(3.0);
    Frame::new()
        .fill(theme.colors.card)
        .stroke(Stroke::new(1.0, theme.colors.separator))
        .corner_radius(CornerRadius::same(6))
        .inner_margin(Margin::symmetric(8, 4))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let response = ui.add(
                    egui::DragValue::new(value)
                        .range(range)
                        .speed(speed)
                        .suffix(suffix)
                        .max_decimals(1),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if icon_button(ui, theme, Icon::Plus, Some("Aumenta"), false).clicked() {
                        // DragValue responds to the next frame; direct increment keeps
                        // the stepper usable without a second interaction.
                        *value += speed;
                    }
                    if icon_button(ui, theme, Icon::Minus, Some("Diminuisci"), false).clicked() {
                        *value -= speed;
                    }
                });
                response
            })
            .inner
        })
        .inner
}

pub fn slider(
    ui: &mut Ui,
    theme: &Theme,
    value: &mut f32,
    range: std::ops::RangeInclusive<f32>,
    suffix: &str,
    width: f32,
) -> Response {
    let height = 26.0;
    let (rect, mut response) =
        ui.allocate_exact_size(Vec2::new(width, height), Sense::click_and_drag());
    let track = Rect::from_min_max(
        Pos2::new(rect.left(), rect.center().y - 2.0),
        Pos2::new(rect.right(), rect.center().y + 2.0),
    );
    let min = *range.start();
    let max = *range.end();
    let span = (max - min).max(f32::EPSILON);
    let mut changed = false;
    if (response.dragged() || response.clicked()) && response.interact_pointer_pos().is_some() {
        if let Some(pos) = response.interact_pointer_pos() {
            let t = ((pos.x - track.left()) / track.width().max(1.0)).clamp(0.0, 1.0);
            let next = min + t * span;
            if (*value - next).abs() > 0.0001 {
                *value = next;
                changed = true;
            }
        }
    }
    if response.has_focus() {
        ui.input(|input| {
            if input.key_pressed(egui::Key::ArrowLeft) {
                *value = (*value - span * 0.01).max(min);
                changed = true;
            }
            if input.key_pressed(egui::Key::ArrowRight) {
                *value = (*value + span * 0.01).min(max);
                changed = true;
            }
        });
    }
    if changed {
        response.mark_changed();
    }
    let t = ((*value - min) / span).clamp(0.0, 1.0);
    let painter = ui.painter();
    painter.rect_filled(track, CornerRadius::same(2), theme.colors.separator_strong);
    painter.rect_filled(
        Rect::from_min_size(track.min, Vec2::new(track.width() * t, track.height())),
        CornerRadius::same(2),
        theme.colors.accent,
    );
    painter.circle_filled(
        Pos2::new(track.left() + track.width() * t, track.center().y),
        6.0,
        Color32::WHITE,
    );
    painter.circle_stroke(
        Pos2::new(track.left() + track.width() * t, track.center().y),
        6.0,
        Stroke::new(1.0, with_alpha(theme.colors.shadow, 90)),
    );
    if response.has_focus() {
        painter.rect_stroke(
            rect.expand(1.0),
            CornerRadius::same(7),
            Stroke::new(2.0, theme.colors.accent),
            StrokeKind::Outside,
        );
    }
    if !suffix.is_empty() {
        painter.text(
            Pos2::new(rect.right() + 8.0, rect.center().y),
            Align2::LEFT_CENTER,
            format!("{value:.1}{suffix}"),
            timecode(11.0),
            theme.colors.text_secondary,
        );
    }
    response
}

pub fn card<R>(ui: &mut Ui, theme: &Theme, add: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
    Frame::new()
        .fill(theme.colors.card)
        .stroke(Stroke::new(1.0, theme.colors.separator))
        .corner_radius(CornerRadius::same(11))
        .inner_margin(Margin::same(16))
        .show(ui, add)
}

pub fn section<R>(
    ui: &mut Ui,
    theme: &Theme,
    id: &str,
    title: &str,
    default_open: bool,
    add: impl FnOnce(&mut Ui) -> R,
) -> Option<R> {
    let id = Id::new(("ui-section", id));
    let mut open = ui
        .data(|data| data.get_temp::<bool>(id))
        .unwrap_or(default_open);
    let width = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 30.0), Sense::click());
    if response.clicked() {
        open = !open;
        ui.data_mut(|data| data.insert_temp(id, open));
    }
    let painter = ui.painter();
    painter.text(
        Pos2::new(rect.left(), rect.center().y),
        Align2::LEFT_CENTER,
        title,
        section_title(),
        theme.colors.text,
    );
    let chevron_rect = Rect::from_center_size(
        Pos2::new(rect.right() - 8.0, rect.center().y),
        Vec2::splat(16.0),
    );
    icon(
        painter,
        chevron_rect,
        if open {
            Icon::ChevronDown
        } else {
            Icon::ChevronRight
        },
        theme.colors.text_tertiary,
        1.6,
    );
    painter.line_segment(
        [
            Pos2::new(rect.left(), rect.bottom()),
            Pos2::new(rect.right(), rect.bottom()),
        ],
        Stroke::new(1.0, theme.colors.separator),
    );
    if open {
        ui.add_space(8.0);
        Some(add(ui))
    } else {
        None
    }
}

pub fn section_label(ui: &mut Ui, theme: &Theme, text: &str) {
    ui.label(
        RichText::new(text.to_uppercase())
            .size(11.0)
            .color(theme.colors.text_tertiary),
    );
}

pub fn divider(ui: &mut Ui, theme: &Theme) {
    let rect = ui.available_rect_before_wrap();
    let y = rect.top() + 4.0;
    ui.painter().line_segment(
        [Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)],
        Stroke::new(1.0, theme.colors.separator),
    );
    ui.add_space(9.0);
}

pub struct ListRow<'a> {
    pub title: &'a str,
    pub subtitle: &'a str,
    pub duration: &'a str,
    pub icon: Icon,
    pub accent: Color32,
    pub selected: bool,
    pub badge: Option<&'a str>,
    pub waveform: Option<&'a [f32]>,
    pub thumbnail: Option<&'a TextureHandle>,
    pub trailing_icon: Option<Icon>,
    pub missing: bool,
}

impl<'a> ListRow<'a> {
    pub fn new(title: &'a str, subtitle: &'a str, icon_kind: Icon) -> Self {
        Self {
            title,
            subtitle,
            duration: "",
            icon: icon_kind,
            accent: Color32::from_rgb(112, 168, 255),
            selected: false,
            badge: None,
            waveform: None,
            thumbnail: None,
            trailing_icon: None,
            missing: false,
        }
    }

    pub fn show(self, ui: &mut Ui, theme: &Theme) -> Response {
        let width = ui.available_width();
        let (rect, response) =
            ui.allocate_exact_size(Vec2::new(width, theme.metrics.row), Sense::click_and_drag());
        let hover = ui.ctx().animate_bool_with_time(
            response.id.with("row-hover"),
            response.hovered(),
            theme.motion_time(),
        );
        let painter = ui.painter();
        if self.selected || hover > 0.0 {
            painter.rect_filled(
                rect.shrink2(Vec2::new(0.0, 1.0)),
                CornerRadius::same(6),
                if self.selected {
                    theme.colors.accent_soft
                } else {
                    lerp(Color32::TRANSPARENT, theme.colors.hover, hover)
                },
            );
        }
        if self.selected {
            painter.rect_filled(
                Rect::from_min_size(
                    rect.min + Vec2::new(0.0, 5.0),
                    Vec2::new(2.0, rect.height() - 10.0),
                ),
                CornerRadius::same(1),
                theme.colors.accent,
            );
        }
        let lead = Rect::from_center_size(
            Pos2::new(rect.left() + 28.0, rect.center().y),
            Vec2::new(42.0, 30.0),
        );
        painter.rect_filled(lead, CornerRadius::same(5), theme.colors.well);
        if let Some(texture) = self.thumbnail {
            paint_texture_fit(painter, texture, lead);
        } else if let Some(peaks) = self.waveform.filter(|peaks| !peaks.is_empty()) {
            waveform(
                painter,
                lead.shrink2(Vec2::new(5.0, 7.0)),
                peaks,
                self.accent,
                None,
            );
        } else {
            icon(
                painter,
                lead.shrink(8.0),
                self.icon,
                if self.missing {
                    theme.colors.red
                } else {
                    self.accent
                },
                1.5,
            );
        }
        let text_x = lead.right() + 10.0;
        let title_color = if self.missing {
            theme.colors.red
        } else {
            theme.colors.text
        };
        paint_truncated(
            painter,
            Pos2::new(text_x, rect.center().y - 9.0),
            self.title,
            body(),
            title_color,
            rect.right() - text_x - 55.0,
        );
        paint_truncated(
            painter,
            Pos2::new(text_x, rect.center().y + 6.0),
            self.subtitle,
            caption(),
            theme.colors.text_secondary,
            rect.right() - text_x - 55.0,
        );
        if !self.duration.is_empty() {
            painter.text(
                Pos2::new(rect.right() - 10.0, rect.top() + 9.0),
                Align2::RIGHT_TOP,
                self.duration,
                timecode(11.0),
                theme.colors.text_secondary,
            );
        }
        if let Some(badge) = self.badge {
            let galley = painter.layout_no_wrap(badge.to_string(), micro(), theme.colors.accent);
            let badge_rect = Rect::from_min_size(
                Pos2::new(rect.right() - galley.size().x - 20.0, rect.bottom() - 22.0),
                Vec2::new(galley.size().x + 10.0, 16.0),
            );
            painter.rect_filled(badge_rect, CornerRadius::same(8), theme.colors.accent_soft);
            painter.galley(
                Pos2::new(
                    badge_rect.left() + 5.0,
                    badge_rect.center().y - galley.size().y / 2.0,
                ),
                galley,
                theme.colors.accent,
            );
        }
        if let Some(trailing) = self.trailing_icon {
            icon(
                painter,
                Rect::from_center_size(
                    Pos2::new(rect.right() - 16.0, rect.center().y),
                    Vec2::splat(14.0),
                ),
                trailing,
                theme.colors.text_tertiary,
                1.4,
            );
        }
        if response.has_focus() {
            painter.rect_stroke(
                rect.shrink(1.0),
                CornerRadius::same(7),
                Stroke::new(2.0, theme.colors.accent),
                StrokeKind::Inside,
            );
        }
        response
    }
}

pub fn status_pill(
    ui: &mut Ui,
    _theme: &Theme,
    text: &str,
    color: Color32,
    icon_kind: Option<Icon>,
) -> Response {
    let galley = ui
        .painter()
        .layout_no_wrap(text.to_string(), micro(), Color32::WHITE);
    let width = galley.size().x + 22.0 + if icon_kind.is_some() { 16.0 } else { 0.0 };
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 22.0), Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, CornerRadius::same(11), tint(color, 32));
    painter.rect_stroke(
        rect,
        CornerRadius::same(11),
        Stroke::new(1.0, tint(color, 96)),
        StrokeKind::Inside,
    );
    painter.circle_filled(Pos2::new(rect.left() + 10.0, rect.center().y), 3.0, color);
    let mut x = 17.0;
    if let Some(icon_kind) = icon_kind {
        icon(
            painter,
            Rect::from_center_size(
                Pos2::new(rect.left() + x + 6.0, rect.center().y),
                Vec2::splat(12.0),
            ),
            icon_kind,
            color,
            1.3,
        );
        x += 16.0;
    }
    painter.text(
        Pos2::new(rect.left() + x, rect.center().y),
        Align2::LEFT_CENTER,
        text,
        micro(),
        color,
    );
    response
}

pub fn badge(ui: &mut Ui, _theme: &Theme, text: &str, color: Color32) -> Response {
    let galley = ui
        .painter()
        .layout_no_wrap(text.to_string(), micro(), color);
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(galley.size().x + 12.0, 18.0), Sense::hover());
    ui.painter()
        .rect_filled(rect, CornerRadius::same(9), tint(color, 26));
    ui.painter().galley(
        Pos2::new(rect.left() + 6.0, rect.center().y - galley.size().y / 2.0),
        galley,
        color,
    );
    response
}

pub fn progress_bar(ui: &mut Ui, theme: &Theme, progress: f32, color: Color32) -> Response {
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 4.0), Sense::hover());
    ui.painter()
        .rect_filled(rect, CornerRadius::same(2), theme.colors.separator);
    ui.painter().rect_filled(
        Rect::from_min_size(
            rect.min,
            Vec2::new(rect.width() * progress.clamp(0.0, 1.0), rect.height()),
        ),
        CornerRadius::same(2),
        color,
    );
    response
}

pub fn waveform(painter: &Painter, rect: Rect, peaks: &[f32], color: Color32, played: Option<f32>) {
    let painter = painter.with_clip_rect(rect);
    if peaks.is_empty() {
        painter.line_segment(
            [
                Pos2::new(rect.left(), rect.center().y),
                Pos2::new(rect.right(), rect.center().y),
            ],
            Stroke::new(1.0, with_alpha(color, 55)),
        );
        return;
    }
    let mid = rect.center().y;
    let step = (rect.width() / peaks.len() as f32).max(1.0);
    let limit = played.map(|p| rect.left() + rect.width() * p.clamp(0.0, 1.0));
    let base = with_alpha(color, 60);
    for (index, peak) in peaks.iter().enumerate() {
        let x = rect.left() + index as f32 * step + step * 0.5;
        let amplitude = (peak.clamp(0.0, 1.0) * rect.height() * 0.46).max(0.5);
        let active = limit.is_none_or(|limit| x <= limit);
        painter.line_segment(
            [Pos2::new(x, mid - amplitude), Pos2::new(x, mid + amplitude)],
            Stroke::new(step.max(1.0), if active { color } else { base }),
        );
    }
}

pub fn vu_meter(
    ui: &mut Ui,
    theme: &Theme,
    label: &str,
    left: f32,
    right: f32,
    peak_left: f32,
    peak_right: f32,
) -> Response {
    let width = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 38.0), Sense::hover());
    let painter = ui.painter();
    painter.text(
        Pos2::new(rect.left(), rect.top() + 1.0),
        Align2::LEFT_TOP,
        label,
        caption(),
        theme.colors.text_secondary,
    );
    for (index, (value, peak)) in [(left, peak_left), (right, peak_right)].iter().enumerate() {
        let y = rect.top() + 15.0 + index as f32 * 10.0;
        let bar = Rect::from_min_max(
            Pos2::new(rect.left(), y),
            Pos2::new(rect.right() - 3.0, y + 6.0),
        );
        painter.rect_filled(bar, CornerRadius::same(2), theme.colors.well);
        let segments = 32;
        let gap = 2.0;
        let segment_w = (bar.width() - gap * (segments as f32 - 1.0)) / segments as f32;
        for segment in 0..segments {
            let t = segment as f32 / segments as f32;
            let color = if t > 0.9 {
                theme.colors.red
            } else if t > 0.76 {
                theme.colors.amber
            } else {
                theme.colors.green
            };
            let active = t <= value.clamp(0.0, 1.0);
            let x = bar.left() + segment as f32 * (segment_w + gap);
            painter.rect_filled(
                Rect::from_min_size(Pos2::new(x, y), Vec2::new(segment_w, 6.0)),
                CornerRadius::same(1),
                if active { color } else { with_alpha(color, 30) },
            );
        }
        let peak_x = bar.left() + bar.width() * peak.clamp(0.0, 1.0);
        painter.line_segment(
            [Pos2::new(peak_x, y - 1.0), Pos2::new(peak_x, y + 7.0)],
            Stroke::new(1.0, theme.colors.text),
        );
    }
    response
}

pub fn empty_state(
    ui: &mut Ui,
    theme: &Theme,
    icon_kind: Icon,
    title: &str,
    body_text: &str,
    action: impl FnOnce(&mut Ui),
) {
    ui.vertical_centered(|ui| {
        ui.add_space(48.0);
        let (rect, _) = ui.allocate_exact_size(Vec2::splat(44.0), Sense::hover());
        ui.painter()
            .rect_filled(rect, CornerRadius::same(12), theme.colors.card);
        icon(
            ui.painter(),
            rect.shrink(12.0),
            icon_kind,
            theme.colors.text_tertiary,
            1.5,
        );
        ui.add_space(12.0);
        ui.label(RichText::new(title).size(16.0).color(theme.colors.text));
        ui.add_space(4.0);
        ui.label(
            RichText::new(body_text)
                .size(12.0)
                .color(theme.colors.text_secondary),
        );
        ui.add_space(14.0);
        action(ui);
    });
}

pub fn drop_zone(ui: &mut Ui, theme: &Theme, text: &str, active: bool) -> Response {
    let width = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 92.0), Sense::click());
    let painter = ui.painter();
    let fill = if active {
        tint(theme.colors.accent, 22)
    } else {
        with_alpha(theme.colors.card, 120)
    };
    painter.rect_filled(rect, CornerRadius::same(10), fill);
    let color = if active {
        theme.colors.accent
    } else {
        theme.colors.separator_strong
    };
    dashed_rect(painter, rect.shrink(1.0), 7.0, 5.0, Stroke::new(1.0, color));
    let icon_rect = Rect::from_center_size(
        Pos2::new(rect.center().x, rect.top() + 27.0),
        Vec2::splat(22.0),
    );
    icon(
        painter,
        icon_rect,
        Icon::Download,
        if active {
            theme.colors.accent
        } else {
            theme.colors.text_tertiary
        },
        1.5,
    );
    painter.text(
        Pos2::new(rect.center().x, rect.top() + 48.0),
        Align2::CENTER_TOP,
        text,
        body(),
        theme.colors.text,
    );
    painter.text(
        Pos2::new(rect.center().x, rect.top() + 66.0),
        Align2::CENTER_TOP,
        "Audio, video o una serata .nerdshow",
        caption(),
        theme.colors.text_tertiary,
    );
    response
}

fn dashed_rect(painter: &Painter, rect: Rect, radius: f32, dash: f32, stroke: Stroke) {
    let r = radius.min(rect.width() / 2.0).min(rect.height() / 2.0);
    let corners = [
        (Pos2::new(rect.left() + r, rect.top()), 0.0),
        (Pos2::new(rect.right() - r, rect.top()), 0.0),
        (
            Pos2::new(rect.right(), rect.top() + r),
            std::f32::consts::FRAC_PI_2,
        ),
        (
            Pos2::new(rect.right(), rect.bottom() - r),
            std::f32::consts::FRAC_PI_2,
        ),
        (
            Pos2::new(rect.right() - r, rect.bottom()),
            std::f32::consts::PI,
        ),
        (
            Pos2::new(rect.left() + r, rect.bottom()),
            std::f32::consts::PI,
        ),
        (
            Pos2::new(rect.left(), rect.bottom() - r),
            3.0 * std::f32::consts::FRAC_PI_2,
        ),
        (
            Pos2::new(rect.left(), rect.top() + r),
            3.0 * std::f32::consts::FRAC_PI_2,
        ),
    ];
    for (start, angle) in corners {
        let steps = 9;
        for step in 0..steps {
            if step % 2 == 1 {
                continue;
            }
            let a0 = angle + TAU / 4.0 * step as f32 / steps as f32;
            let a1 = angle + TAU / 4.0 * (step + 1) as f32 / steps as f32;
            painter.line_segment(
                [
                    Pos2::new(start.x + a0.cos() * r, start.y + a0.sin() * r),
                    Pos2::new(start.x + a1.cos() * r, start.y + a1.sin() * r),
                ],
                stroke,
            );
        }
    }
    let _ = dash;
}

pub fn paint_truncated(
    painter: &Painter,
    pos: Pos2,
    text: &str,
    font: FontId,
    color: Color32,
    max_width: f32,
) {
    let galley = painter.layout_no_wrap(text.to_string(), font, color);
    let painter = painter.with_clip_rect(Rect::from_min_size(
        pos - Vec2::new(0.0, 2.0),
        Vec2::new(max_width.max(8.0), 40.0),
    ));
    painter.galley(pos, galley, color);
}

pub fn paint_texture_fit(painter: &Painter, texture: &TextureHandle, rect: Rect) {
    let size = texture.size_vec2();
    let aspect = if size.y > 0.0 {
        size.x / size.y
    } else {
        16.0 / 9.0
    };
    let dest = fit_rect(rect, aspect);
    painter.image(
        texture.id(),
        dest,
        Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
        Color32::WHITE,
    );
}

pub fn fit_rect(container: Rect, aspect: f32) -> Rect {
    let aspect = if aspect.is_finite() && aspect > 0.0 {
        aspect
    } else {
        16.0 / 9.0
    };
    let container_aspect = container.width() / container.height().max(1.0);
    if container_aspect > aspect {
        let height = container.height();
        Rect::from_center_size(container.center(), Vec2::new(height * aspect, height))
    } else {
        let width = container.width();
        Rect::from_center_size(container.center(), Vec2::new(width, width / aspect))
    }
}

pub fn page_header(
    ui: &mut Ui,
    theme: &Theme,
    title: &str,
    subtitle: &str,
    actions: impl FnOnce(&mut Ui),
) {
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(
                RichText::new(title)
                    .font(page_title())
                    .color(theme.colors.text),
            );
            ui.add_space(2.0);
            ui.label(
                RichText::new(subtitle)
                    .font(secondary())
                    .color(theme.colors.text_secondary),
            );
        });
        ui.with_layout(Layout::right_to_left(Align::Center), actions);
    });
}

#[allow(clippy::too_many_arguments)]
pub fn menu_button(
    ui: &mut Ui,
    theme: &Theme,
    id: &str,
    label: &str,
    icon_kind: Option<Icon>,
    variant: ButtonVariant,
    min_width: f32,
    add: impl FnOnce(&mut Ui),
) -> Response {
    let mut button = Button::new(label).variant(variant).min_width(min_width);
    if let Some(icon_kind) = icon_kind {
        button = button.icon(icon_kind);
    }
    let response = button.show(ui, theme);
    let state_id = Id::new(("menu-button", id));
    if response.clicked() {
        let open = ui
            .data(|data| data.get_temp::<bool>(state_id))
            .unwrap_or(false);
        ui.data_mut(|data| data.insert_temp(state_id, !open));
    }
    let open = ui
        .data(|data| data.get_temp::<bool>(state_id))
        .unwrap_or(false);
    if open {
        let mut close = false;
        let screen = ui.ctx().content_rect();
        let mut pos = response.rect.left_bottom() + Vec2::new(0.0, 4.0);
        if pos.y + 240.0 > screen.bottom() {
            pos.y = response.rect.top() - 4.0 - 240.0;
        }
        pos.x = pos.x.min(screen.right() - 240.0);
        let area = egui::Area::new(state_id.with("popup"))
            .order(egui::Order::Foreground)
            .fixed_pos(pos)
            .show(ui.ctx(), |ui| {
                Frame::new()
                    .fill(theme.colors.popover)
                    .stroke(Stroke::new(1.0, theme.colors.separator_strong))
                    .corner_radius(CornerRadius::same(9))
                    .inner_margin(Margin::same(6))
                    .shadow(egui::epaint::Shadow {
                        offset: [0, 8],
                        blur: 24,
                        spread: 0,
                        color: theme.colors.shadow,
                    })
                    .show(ui, |ui| {
                        ui.set_min_width(190.0);
                        add(ui);
                        if ui.input(|input| input.pointer.any_click()) {
                            close = true;
                        }
                    });
            });
        let popup_rect = area.response.rect;
        let pointer = ui.ctx().input(|input| input.pointer.interact_pos());
        if ui.ctx().input(|input| input.key_pressed(egui::Key::Escape))
            || pointer.is_some_and(|pos| {
                ui.ctx().input(|input| input.pointer.any_click())
                    && !popup_rect.contains(pos)
                    && !response.rect.contains(pos)
            })
        {
            close = true;
        }
        if close {
            ui.data_mut(|data| data.insert_temp(state_id, false));
        }
    }
    response
}

pub fn menu_item(ui: &mut Ui, theme: &Theme, label: &str, icon_kind: Option<Icon>) -> Response {
    let response = ui.add(
        egui::Button::new(RichText::new(label).size(12.0).color(theme.colors.text))
            .frame(false)
            .min_size(Vec2::new(ui.available_width(), 28.0)),
    );
    let rect = response.rect;
    if response.hovered() {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(5), theme.colors.hover);
    }
    if let Some(icon_kind) = icon_kind {
        icon(
            ui.painter(),
            Rect::from_center_size(
                Pos2::new(rect.left() + 12.0, rect.center().y),
                Vec2::splat(14.0),
            ),
            icon_kind,
            theme.colors.text_secondary,
            1.4,
        );
        ui.painter().text(
            Pos2::new(rect.left() + 24.0, rect.center().y),
            Align2::LEFT_CENTER,
            label,
            body(),
            theme.colors.text,
        );
    } else {
        ui.painter().text(
            Pos2::new(rect.left() + 8.0, rect.center().y),
            Align2::LEFT_CENTER,
            label,
            body(),
            theme.colors.text,
        );
    }
    response
}

pub fn sheet(
    ctx: &Context,
    theme: &Theme,
    id: &str,
    open: &mut bool,
    title: &str,
    subtitle: &str,
    add: impl FnOnce(&mut Ui),
) {
    if !*open {
        return;
    }
    let screen = ctx.content_rect();
    egui::Area::new(Id::new(("sheet-dim", id)))
        .order(egui::Order::Middle)
        .fixed_pos(screen.min)
        .show(ctx, |ui| {
            let (_, response) = ui.allocate_exact_size(screen.size(), Sense::click());
            ui.painter()
                .rect_filled(screen, CornerRadius::ZERO, with_alpha(Color32::BLACK, 120));
            let _ = response;
        });
    let mut close = false;
    egui::Window::new(title)
        .id(Id::new(("sheet", id)))
        .title_bar(false)
        .resizable(false)
        .collapsible(false)
        .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
        .frame(
            Frame::new()
                .fill(theme.colors.panel)
                .stroke(Stroke::new(1.0, theme.colors.separator_strong))
                .corner_radius(CornerRadius::same(12))
                .inner_margin(Margin::same(20))
                .shadow(egui::epaint::Shadow {
                    offset: [0, 16],
                    blur: 40,
                    spread: 0,
                    color: theme.colors.shadow,
                }),
        )
        .show(ctx, |ui| {
            ui.set_min_width(420.0);
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(title)
                        .font(section_title())
                        .color(theme.colors.text),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if icon_button(ui, theme, Icon::Close, Some("Chiudi"), false).clicked() {
                        close = true;
                    }
                });
            });
            if !subtitle.is_empty() {
                ui.add_space(2.0);
                ui.label(
                    RichText::new(subtitle)
                        .font(secondary())
                        .color(theme.colors.text_secondary),
                );
            }
            ui.add_space(12.0);
            add(ui);
        });
    if close {
        *open = false;
    }
}

pub fn toast_frame(ctx: &Context, theme: &Theme, message: &str, color: Color32) {
    egui::Area::new(Id::new("nerd-toast"))
        .order(egui::Order::Foreground)
        .anchor(Align2::RIGHT_BOTTOM, [-18.0, -72.0])
        .show(ctx, |ui| {
            Frame::new()
                .fill(theme.colors.popover)
                .stroke(Stroke::new(1.0, theme.colors.separator_strong))
                .corner_radius(CornerRadius::same(9))
                .inner_margin(Margin::symmetric(12, 9))
                .shadow(egui::epaint::Shadow {
                    offset: [0, 8],
                    blur: 24,
                    spread: 0,
                    color: theme.colors.shadow,
                })
                .show(ui, |ui| {
                    ui.set_max_width(420.0);
                    ui.horizontal(|ui| {
                        icon(
                            ui.painter(),
                            Rect::from_center_size(
                                ui.cursor().min + Vec2::new(8.0, 0.0),
                                Vec2::splat(14.0),
                            ),
                            Icon::Dot,
                            color,
                            1.0,
                        );
                        ui.add_space(16.0);
                        ui.label(RichText::new(message).font(body()).color(theme.colors.text));
                    });
                });
        });
}

pub fn format_clock(ms: u64) -> String {
    let total = ms / 1000;
    format!("{:02}:{:02}", total / 60, total % 60)
}

pub fn scroll(ui: &mut Ui, id: &str, add: impl FnOnce(&mut Ui)) {
    ScrollArea::vertical()
        .id_salt(id)
        .auto_shrink([false, false])
        .show(ui, |ui| add(ui));
}

pub fn chip_row(ui: &mut Ui, theme: &Theme, items: &[(&str, &str)]) {
    ui.horizontal_wrapped(|ui| {
        for (key, action) in items {
            keycap(ui, theme, key);
            ui.label(
                RichText::new(*action)
                    .font(caption())
                    .color(theme.colors.text_secondary),
            );
            ui.add_space(6.0);
        }
    });
}

pub fn screen_placeholder(
    painter: &Painter,
    rect: Rect,
    theme: &Theme,
    icon_kind: Icon,
    label: &str,
) {
    painter.rect_filled(rect, CornerRadius::same(8), theme.colors.well);
    painter.rect_stroke(
        rect,
        CornerRadius::same(8),
        Stroke::new(1.0, theme.colors.separator_strong),
        StrokeKind::Inside,
    );
    let icon_rect = Rect::from_center_size(
        Pos2::new(rect.center().x, rect.center().y - 12.0),
        Vec2::splat(28.0),
    );
    icon(
        painter,
        icon_rect,
        icon_kind,
        theme.colors.text_tertiary,
        1.5,
    );
    painter.text(
        Pos2::new(rect.center().x, rect.center().y + 16.0),
        Align2::CENTER_TOP,
        label,
        caption(),
        theme.colors.text_tertiary,
    );
}

pub fn metadata_row(ui: &mut Ui, theme: &Theme, label: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(label)
                .font(caption())
                .color(theme.colors.text_tertiary),
        );
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.add(
                egui::Label::new(
                    RichText::new(value)
                        .font(caption())
                        .color(theme.colors.text_secondary),
                )
                .truncate(),
            )
            .on_hover_text(value);
        });
    });
}

pub fn toolbar_frame(ui: &mut Ui, theme: &Theme, add: impl FnOnce(&mut Ui)) {
    Frame::new()
        .fill(theme.colors.panel)
        .inner_margin(Margin::symmetric(16, 8))
        .show(ui, |ui| add(ui));
    let rect = ui.max_rect();
    ui.painter().line_segment(
        [
            Pos2::new(rect.left(), rect.top()),
            Pos2::new(rect.right(), rect.top()),
        ],
        Stroke::new(1.0, theme.colors.separator),
    );
}

pub fn underlay(ui: &mut Ui, theme: &Theme, add: impl FnOnce(&mut Ui)) {
    Frame::new()
        .fill(theme.colors.well)
        .inner_margin(Margin::same(0))
        .show(ui, add);
}
