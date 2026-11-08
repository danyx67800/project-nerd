//! Small reusable editor widgets.

use eframe::egui::{self, Color32, FontId, Pos2, Rect, Response, RichText, Stroke, Ui, Vec2};
use nerd_core::model::{PlaybackMode, SongStatus};

use crate::theme::Palette;

pub fn filled_button(
    ui: &mut Ui,
    text: impl Into<String>,
    fill: Color32,
    text_color: Color32,
    min_size: Vec2,
) -> Response {
    ui.add(
        egui::Button::new(
            RichText::new(text.into())
                .size(13.5)
                .color(text_color)
                .strong(),
        )
        .fill(fill)
        .stroke(Stroke::new(1.0, fill.gamma_multiply(1.2)))
        .corner_radius(egui::CornerRadius::same(9))
        .min_size(min_size),
    )
}

pub fn accent_button(ui: &mut Ui, palette: Palette, text: impl Into<String>) -> Response {
    filled_button(
        ui,
        text,
        palette.accent,
        Color32::from_rgb(9, 23, 21),
        Vec2::new(0.0, 34.0),
    )
}

pub fn danger_button(ui: &mut Ui, palette: Palette, text: impl Into<String>) -> Response {
    filled_button(
        ui,
        text,
        palette.danger,
        Color32::from_rgb(30, 8, 8),
        Vec2::new(0.0, 34.0),
    )
}

pub fn ghost_button(ui: &mut Ui, palette: Palette, text: impl Into<String>) -> Response {
    ui.add(
        egui::Button::new(RichText::new(text.into()).size(13.0).color(palette.text))
            .fill(Color32::TRANSPARENT)
            .stroke(Stroke::new(1.0, palette.border))
            .corner_radius(egui::CornerRadius::same(8))
            .min_size(Vec2::new(0.0, 32.0)),
    )
}

pub fn pill(ui: &mut Ui, text: impl Into<String>, color: Color32) -> Response {
    egui::Frame::new()
        .fill(color.gamma_multiply(0.16))
        .stroke(Stroke::new(1.0, color.gamma_multiply(0.45)))
        .corner_radius(egui::CornerRadius::same(20))
        .inner_margin(egui::Margin::symmetric(9, 3))
        .show(ui, |ui| {
            ui.label(RichText::new(text.into()).size(11.0).color(color).strong())
        })
        .response
}

pub fn section_label(ui: &mut Ui, palette: Palette, text: &str) {
    ui.label(
        RichText::new(text.to_uppercase())
            .size(10.5)
            .strong()
            .color(palette.text_muted),
    );
    ui.add_space(4.0);
}

pub fn empty_state(ui: &mut Ui, palette: Palette, title: &str, body: &str) {
    ui.vertical_centered(|ui| {
        ui.add_space(48.0);
        ui.label(RichText::new(title).size(17.0).strong().color(palette.text));
        ui.add_space(6.0);
        ui.label(RichText::new(body).size(12.5).color(palette.text_muted));
    });
}

pub fn waveform(
    painter: &egui::Painter,
    rect: Rect,
    peaks: &[f32],
    color: Color32,
    played: Option<f32>,
) {
    if peaks.is_empty() {
        return;
    }
    painter.rect_filled(
        rect,
        egui::CornerRadius::same(4),
        color.gamma_multiply(0.08),
    );
    let mid = rect.center().y;
    let step = (rect.width() / peaks.len() as f32).max(1.0);
    let played_x = played.map(|p| rect.left() + rect.width() * p.clamp(0.0, 1.0));
    for (i, peak) in peaks.iter().enumerate() {
        let x = rect.left() + i as f32 * step + step * 0.5;
        let amplitude = (peak.clamp(0.0, 1.0) * rect.height() * 0.46).max(0.5);
        let active = played_x.is_none_or(|limit| x <= limit);
        painter.line_segment(
            [Pos2::new(x, mid - amplitude), Pos2::new(x, mid + amplitude)],
            Stroke::new(
                step.max(1.0),
                if active {
                    color
                } else {
                    color.gamma_multiply(0.28)
                },
            ),
        );
    }
}

pub fn vu_meter(ui: &mut Ui, palette: Palette, value: f32, label: &str, color: Color32) {
    let width = ui.available_width().max(80.0);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 12.0), egui::Sense::hover());
    ui.painter()
        .rect_filled(rect, egui::CornerRadius::same(4), palette.background);
    let filled = rect.width() * value.clamp(0.0, 1.0);
    let fill = if value > 0.92 {
        palette.danger
    } else if value > 0.8 {
        palette.warning
    } else {
        color
    };
    ui.painter().rect_filled(
        Rect::from_min_size(rect.min, Vec2::new(filled, rect.height())),
        egui::CornerRadius::same(4),
        fill,
    );
    ui.label(RichText::new(label).size(10.5).color(palette.text_muted));
    let _ = FontId::proportional(10.0);
}

pub fn status_color(palette: Palette, status: SongStatus) -> Color32 {
    match status {
        SongStatus::Pending => palette.text_muted,
        SongStatus::Live => palette.accent,
        SongStatus::Done => palette.success,
        SongStatus::Skipped => palette.warning,
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
    let rect = ui.available_rect_before_wrap();
    let y = rect.top() + 4.0;
    ui.painter().line_segment(
        [Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)],
        Stroke::new(1.0, palette.border),
    );
    ui.add_space(9.0);
}

pub fn shortcut_chip(ui: &mut Ui, palette: Palette, key: &str, action: &str) {
    ui.horizontal(|ui| {
        egui::Frame::new()
            .fill(palette.raised)
            .stroke(Stroke::new(1.0, palette.border))
            .corner_radius(egui::CornerRadius::same(5))
            .inner_margin(egui::Margin::symmetric(7, 2))
            .show(ui, |ui| {
                ui.label(
                    RichText::new(key)
                        .size(10.5)
                        .color(palette.text)
                        .monospace(),
                );
            });
        ui.label(RichText::new(action).size(11.0).color(palette.text_muted));
    });
}
