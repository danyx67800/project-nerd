//! Shared projector renderer.
//!
//! Preview and the real output call exactly the same painting function, so a
//! title cannot look different on the projector. Sizes are relative to the
//! output height.

use eframe::egui::{
    self, Align2, Color32, FontId, Painter, Pos2, Rect, Stroke, TextureHandle, Vec2,
};
use nerd_core::model::{
    IdleScreen, TitleAnimation, TitleInfo, TitleLine, TitlePosition, TitleTemplate,
};
use nerd_core::Phase;

#[derive(Debug, Clone)]
pub struct ProjectorSong {
    pub title: String,
    pub artist: String,
    pub info: TitleInfo,
    pub has_video: bool,
}

#[derive(Clone)]
pub struct ProjectorState {
    pub template: TitleTemplate,
    pub song: Option<ProjectorSong>,
    pub title_elapsed_ms: u64,
    pub title_active: bool,
    pub blackout: bool,
    pub show_titles: bool,
    pub idle_screen: IdleScreen,
    pub courtesy_message: String,
    pub next_title: Option<String>,
    pub phase: Phase,
    pub video_frame: Option<TextureHandle>,
    pub video_aspect: Option<f32>,
    pub logo: Option<TextureHandle>,
    pub project_name: String,
    pub venue: String,
    /// Used by the Titoli preview to fake a video behind the title.
    pub preview_backdrop: bool,
}

/// Paints the complete projector frame into `rect`.
pub fn paint_output(painter: &Painter, rect: Rect, state: &ProjectorState) {
    painter.rect_filled(rect, egui::CornerRadius::ZERO, Color32::BLACK);
    if state.blackout {
        return;
    }
    let painter = painter.with_clip_rect(rect);

    if state.preview_backdrop {
        paint_preview_backdrop(&painter, rect);
    }

    if let Some(texture) = &state.video_frame {
        let aspect = state.video_aspect.unwrap_or_else(|| {
            let size = texture.size_vec2();
            if size.y > 0.0 {
                size.x / size.y
            } else {
                16.0 / 9.0
            }
        });
        let dest = fit_rect(rect, aspect);
        painter.image(
            texture.id(),
            dest,
            Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
            Color32::WHITE,
        );
    } else if state.idle_screen == IdleScreen::Courtesy
        && !state.preview_backdrop
        && !state.song.as_ref().is_some_and(|s| s.has_video)
    {
        paint_courtesy(&painter, rect, state);
    }

    if state.show_titles {
        if let Some(song) = &state.song {
            if state.title_active {
                paint_title(&painter, rect, state, song);
            }
        }
    }
}

fn fit_rect(container: Rect, aspect: f32) -> Rect {
    let aspect = if aspect.is_finite() && aspect > 0.0 {
        aspect
    } else {
        16.0 / 9.0
    };
    let container_aspect = container.width() / container.height().max(1.0);
    if container_aspect > aspect {
        let height = container.height();
        let width = height * aspect;
        Rect::from_center_size(container.center(), Vec2::new(width, height))
    } else {
        let width = container.width();
        let height = width / aspect;
        Rect::from_center_size(container.center(), Vec2::new(width, height))
    }
}

fn paint_preview_backdrop(painter: &Painter, rect: Rect) {
    // A deliberately abstract "stage" so title previews have contrast without
    // shipping copyrighted footage.
    painter.rect_filled(rect, egui::CornerRadius::ZERO, Color32::from_rgb(9, 15, 24));
    let bands = 18;
    for i in 0..bands {
        let t = i as f32 / bands as f32;
        let y = rect.top() + rect.height() * t;
        let color = Color32::from_rgba_unmultiplied(
            (18.0 + 30.0 * t) as u8,
            (42.0 + 20.0 * (1.0 - t)) as u8,
            (66.0 + 38.0 * (1.0 - t)) as u8,
            255,
        );
        painter.rect_filled(
            Rect::from_min_size(
                Pos2::new(rect.left(), y),
                Vec2::new(rect.width(), rect.height() / bands as f32 + 1.0),
            ),
            egui::CornerRadius::ZERO,
            color,
        );
    }
    let glow_center = Pos2::new(
        rect.left() + rect.width() * 0.68,
        rect.top() + rect.height() * 0.42,
    );
    for i in 0..16 {
        let radius = rect.height() * (0.65 - i as f32 * 0.035);
        painter.circle_filled(
            glow_center,
            radius,
            Color32::from_rgba_unmultiplied(70, 217, 191, 3 + i as u8),
        );
    }
    painter.rect_filled(
        Rect::from_min_size(
            Pos2::new(rect.left(), rect.bottom() - rect.height() * 0.16),
            Vec2::new(rect.width(), rect.height() * 0.16),
        ),
        egui::CornerRadius::ZERO,
        Color32::from_rgba_unmultiplied(3, 5, 8, 180),
    );
}

fn paint_courtesy(painter: &Painter, rect: Rect, state: &ProjectorState) {
    painter.rect_filled(rect, egui::CornerRadius::ZERO, Color32::from_rgb(6, 8, 11));
    let accent = Color32::from_rgba_unmultiplied(70, 217, 191, 200);
    let center_x = rect.center().x;
    let name_size = (rect.height() * 0.032).clamp(14.0, 54.0);
    let message_size = (rect.height() * 0.022).clamp(11.0, 36.0);
    let next_size = (rect.height() * 0.052).clamp(18.0, 90.0);

    let mut y = rect.top() + rect.height() * 0.2;
    if let Some(logo) = &state.logo {
        let logo_h = rect.height() * 0.14;
        let logo_w = logo_h * 2.4;
        let logo_rect = Rect::from_center_size(
            Pos2::new(center_x, y + logo_h / 2.0),
            Vec2::new(logo_w, logo_h),
        );
        painter.image(
            logo.id(),
            logo_rect,
            Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
            Color32::WHITE,
        );
        y += logo_h + rect.height() * 0.06;
    } else {
        painter.text(
            Pos2::new(center_x, y),
            Align2::CENTER_TOP,
            &state.project_name,
            FontId::proportional(name_size),
            Color32::from_rgba_unmultiplied(236, 238, 243, 210),
        );
        y += name_size * 1.6;
        if !state.venue.is_empty() {
            painter.text(
                Pos2::new(center_x, y),
                Align2::CENTER_TOP,
                &state.venue,
                FontId::proportional(message_size),
                Color32::from_rgba_unmultiplied(143, 149, 161, 210),
            );
            y += message_size * 2.0;
        } else {
            y += rect.height() * 0.03;
        }
    }

    painter.line_segment(
        [
            Pos2::new(center_x - rect.width() * 0.04, y),
            Pos2::new(center_x + rect.width() * 0.04, y),
        ],
        Stroke::new((rect.height() * 0.003).max(1.0), accent),
    );
    y += rect.height() * 0.045;

    let message = if state.phase == Phase::Finished {
        "Fine serata"
    } else {
        &state.courtesy_message
    };
    painter.text(
        Pos2::new(center_x, y),
        Align2::CENTER_TOP,
        message,
        FontId::proportional(message_size),
        Color32::from_rgba_unmultiplied(143, 149, 161, 230),
    );
    y += message_size * 1.9;

    if let Some(next) = &state.next_title {
        painter.text(
            Pos2::new(center_x, y),
            Align2::CENTER_TOP,
            next,
            FontId::proportional(next_size),
            Color32::from_rgba_unmultiplied(236, 238, 243, 245),
        );
    } else if state.phase == Phase::Idle {
        painter.text(
            Pos2::new(center_x, y),
            Align2::CENTER_TOP,
            "Pronto per iniziare",
            FontId::proportional(next_size * 0.8),
            Color32::from_rgba_unmultiplied(236, 238, 243, 220),
        );
    }
}

fn paint_title(painter: &Painter, rect: Rect, state: &ProjectorState, song: &ProjectorSong) {
    let template = &state.template;
    let elapsed = state.title_elapsed_ms;
    if elapsed < template.delay_ms {
        return;
    }
    let visible_ms = template.duration_ms;
    let t = elapsed - template.delay_ms;
    if visible_ms > 0 && t > visible_ms {
        return;
    }

    let mut progress = 1.0f32;
    if template.animation != TitleAnimation::None {
        progress = (t as f32 / 520.0).clamp(0.0, 1.0);
        progress = progress * progress * (3.0 - 2.0 * progress);
    }
    let mut alpha = progress;
    if visible_ms > 0 {
        let remaining = visible_ms.saturating_sub(t);
        let exit = (remaining as f32 / 420.0).clamp(0.0, 1.0);
        alpha *= exit;
    }
    if alpha <= 0.001 {
        return;
    }

    let scale = if template.animation == TitleAnimation::Scale {
        0.92 + 0.08 * progress
    } else {
        1.0
    };
    let slide = if template.animation == TitleAnimation::SlideUp {
        (1.0 - progress) * rect.height() * 0.05
    } else {
        0.0
    };

    let title_font = FontId::proportional(
        (rect.height() * template.title_size_pct / 100.0 * 1.0).clamp(11.0, rect.height() * 0.2)
            * scale,
    );
    let body_font = FontId::proportional(
        (rect.height() * template.body_size_pct / 100.0).clamp(7.0, rect.height() * 0.12) * scale,
    );

    let lines: Vec<TitleLine> = song.info.resolved_lines(&song.title, &song.artist);
    if lines.is_empty() {
        return;
    }

    let line_height = |line: &TitleLine| -> f32 {
        match line {
            TitleLine::Title(_) => title_font.size * 1.24,
            TitleLine::Artist(_) => body_font.size * 1.6,
            TitleLine::Meta(_) => body_font.size * 1.5,
        }
    };
    let total_lines_height: f32 = lines.iter().map(line_height).sum();
    let margin = rect.width() * 0.075;
    let panel_padding = rect.width() * 0.022;
    let panel_width = rect.width() - margin * 2.0;
    let panel_height = total_lines_height + panel_padding * 2.0;

    let top = match template.position {
        TitlePosition::Upper => rect.top() + rect.height() * 0.14,
        TitlePosition::Center => rect.center().y - panel_height / 2.0,
        TitlePosition::Lower => rect.bottom() - rect.height() * 0.12 - panel_height,
    } + slide;

    let panel_rect = Rect::from_min_size(
        Pos2::new(rect.left() + margin, top),
        Vec2::new(panel_width, panel_height),
    );

    if template.panel {
        let panel_alpha = (template.panel_opacity * alpha * 255.0) as u8;
        painter.rect_filled(
            panel_rect,
            egui::CornerRadius::same((rect.height() * 0.012) as u8),
            Color32::from_rgba_unmultiplied(7, 9, 13, panel_alpha),
        );
    }

    let accent = color_with_alpha(template.accent_color, alpha);
    let accent_width = (rect.width() * 0.004).max(2.0);
    let accent_inset = panel_padding * 0.55;
    painter.rect_filled(
        Rect::from_min_size(
            Pos2::new(panel_rect.left() + accent_inset, top + panel_padding),
            Vec2::new(accent_width, (title_font.size * 1.05).max(3.0)),
        ),
        egui::CornerRadius::same(2),
        accent,
    );

    let text_x = panel_rect.left() + panel_padding + accent_width + accent_inset;
    let mut y = top + panel_padding;
    for line in &lines {
        let (text, font, color) = match line {
            TitleLine::Title(text) => (
                text,
                title_font.clone(),
                color_with_alpha(template.title_color, alpha),
            ),
            TitleLine::Artist(text) => (
                text,
                body_font.clone(),
                color_with_alpha(template.text_color, alpha * 0.96),
            ),
            TitleLine::Meta(text) => (
                text,
                body_font.clone(),
                color_with_alpha(template.text_color, alpha * 0.72),
            ),
        };
        if text.trim().is_empty() {
            continue;
        }
        let shadow = Color32::from_rgba_unmultiplied(0, 0, 0, (alpha * 130.0) as u8);
        let shadow_offset = (rect.height() * 0.0025).max(1.0);
        painter.text(
            Pos2::new(text_x + shadow_offset, y + shadow_offset),
            Align2::LEFT_TOP,
            text,
            font.clone(),
            shadow,
        );
        painter.text(Pos2::new(text_x, y), Align2::LEFT_TOP, text, font, color);
        y += line_height(line);
    }
}

fn color_with_alpha(rgba: [u8; 4], alpha: f32) -> Color32 {
    let base = Color32::from_rgba_unmultiplied(rgba[0], rgba[1], rgba[2], rgba[3]);
    let a = (base.a() as f32 * alpha.clamp(0.0, 1.0)) as u8;
    Color32::from_rgba_unmultiplied(rgba[0], rgba[1], rgba[2], a)
}

/// Formats the phase as a short Italian label.
pub fn phase_label(phase: Phase) -> &'static str {
    match phase {
        Phase::Idle => "Fermo",
        Phase::Playing => "In onda",
        Phase::Paused => "In pausa",
        Phase::Stopping => "In chiusura",
        Phase::Waiting => "In attesa",
        Phase::Finished => "Fine serata",
        Phase::Panic => "Panic",
    }
}
