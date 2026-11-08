//! Running order, multitrack timeline and song inspector.

use eframe::egui::{
    self, Align, Align2, Color32, CornerRadius, FontId, Layout, Pos2, Rect, RichText, ScrollArea,
    Sense, Stroke, Ui, Vec2,
};
use nerd_core::model::{Song, SongStatus};

use crate::app::{NerdApp, OrderDrag};
use crate::state::{Page, ToastKind};
use crate::widgets;

const ROW_HEIGHT: f32 = 62.0;
const ROW_GAP: f32 = 6.0;
const GUTTER: f32 = 96.0;

pub fn show(app: &mut NerdApp, ui: &mut Ui) {
    toolbar(app, ui);

    egui::Panel::left("running-order")
        .resizable(true)
        .default_size(292.0)
        .min_size(240.0)
        .max_size(380.0)
        .frame(
            egui::Frame::new()
                .fill(app.palette.panel)
                .inner_margin(egui::Margin::symmetric(12, 12))
                .stroke(Stroke::new(1.0, app.palette.border)),
        )
        .show(ui, |ui| order_list(app, ui));

    egui::Panel::right("song-inspector")
        .resizable(true)
        .default_size(342.0)
        .min_size(290.0)
        .max_size(470.0)
        .frame(
            egui::Frame::new()
                .fill(app.palette.panel)
                .inner_margin(egui::Margin::same(16))
                .stroke(Stroke::new(1.0, app.palette.border)),
        )
        .show(ui, |ui| inspector(app, ui));

    egui::CentralPanel::default()
        .frame(
            egui::Frame::new()
                .fill(app.palette.background)
                .inner_margin(egui::Margin::same(18)),
        )
        .show(ui, |ui| timeline(app, ui));
}

fn toolbar(app: &mut NerdApp, ui: &mut Ui) {
    let palette = app.palette;
    let before = app.project.clone();
    let mut changed = false;
    let mut add_asset: Option<String> = None;
    egui::Panel::top("timeline-toolbar")
        .frame(
            egui::Frame::new()
                .fill(palette.panel)
                .inner_margin(egui::Margin::symmetric(18, 9))
                .stroke(Stroke::new(1.0, palette.border)),
        )
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(crate::strings::PAGE_TIMELINE)
                        .size(20.0)
                        .strong()
                        .color(palette.text),
                );
                ui.add_space(8.0);
                ui.label(
                    RichText::new(format!(
                        "{} brani · {}",
                        app.project.songs.len(),
                        crate::strings::format_short_duration(app.project.total_duration_ms())
                    ))
                    .size(11.0)
                    .color(palette.text_muted),
                );
                ui.add_space(14.0);
                let mut name = app.project.name.clone();
                if ui
                    .add(
                        egui::TextEdit::singleline(&mut name)
                            .desired_width(190.0)
                            .hint_text("Nome serata"),
                    )
                    .changed()
                {
                    app.project.name = name;
                    changed = true;
                }
                let mut venue = app.project.venue.clone();
                if ui
                    .add(
                        egui::TextEdit::singleline(&mut venue)
                            .desired_width(150.0)
                            .hint_text("Luogo"),
                    )
                    .changed()
                {
                    app.project.venue = venue;
                    changed = true;
                }
                let mut date = app.project.date.clone();
                if ui
                    .add(
                        egui::TextEdit::singleline(&mut date)
                            .desired_width(100.0)
                            .hint_text("Data"),
                    )
                    .changed()
                {
                    app.project.date = date;
                    changed = true;
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if crate::widgets::accent_button(ui, palette, "Salva").clicked() {
                        app.save(false);
                    }
                    if widgets::ghost_button(ui, palette, "Importa").clicked() {
                        app.import_dialog();
                    }
                    ui.menu_button("＋ Aggiungi", |ui| {
                        ui.set_min_width(340.0);
                        ui.label(
                            RichText::new("Dalla libreria")
                                .size(10.5)
                                .color(palette.text_muted),
                        );
                        ScrollArea::vertical().max_height(380.0).show(ui, |ui| {
                            let mut assets: Vec<_> = app.library.assets.clone();
                            assets.sort_by(|a, b| {
                                a.display_title()
                                    .to_lowercase()
                                    .cmp(&b.display_title().to_lowercase())
                            });
                            if assets.is_empty() {
                                ui.label("La libreria è vuota.");
                            }
                            for asset in assets {
                                if ui
                                    .button(format!(
                                        "{}  ·  {}",
                                        asset.display_title(),
                                        crate::strings::format_timecode(asset.duration_ms)
                                    ))
                                    .clicked()
                                {
                                    add_asset = Some(asset.id.clone());
                                }
                            }
                        });
                    });
                });
            });
        });
    if changed {
        app.commit_edit(&before);
    }
    if let Some(id) = add_asset {
        app.add_asset_to_show(&id);
    }
}

fn order_list(app: &mut NerdApp, ui: &mut Ui) {
    let palette = app.palette;
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("Ordine")
                .size(11.0)
                .strong()
                .color(palette.text_muted),
        );
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(
                RichText::new("trascina per riordinare")
                    .size(9.5)
                    .color(palette.text_muted),
            );
        });
    });
    ui.add_space(6.0);
    if app.project.songs.is_empty() {
        widgets::empty_state(
            ui,
            palette,
            "Scaletta vuota",
            "Aggiungi media dalla libreria o trascina qui i file.",
        );
        return;
    }

    let stride = ROW_HEIGHT + ROW_GAP;
    let ids: Vec<(usize, String)> = app
        .project
        .songs
        .iter()
        .enumerate()
        .map(|(index, song)| (index, song.id.clone()))
        .collect();
    ScrollArea::vertical()
        .id_salt("running-order-scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for (index, id) in &ids {
                let Some(song) = app.project.songs.get(*index).cloned() else {
                    continue;
                };
                let selected = app.selected_song.as_deref() == Some(id.as_str());
                let live = app.show.live_index() == Some(*index);
                let missing = song.audio.as_ref().is_none_or(|path| !path.is_file());
                let width = ui.available_width();
                let (rect, response) =
                    ui.allocate_exact_size(Vec2::new(width, ROW_HEIGHT), Sense::click_and_drag());
                let hovered = response.hovered();
                ui.painter().rect(
                    rect,
                    CornerRadius::same(9),
                    if selected {
                        palette.raised
                    } else if live {
                        palette.accent.gamma_multiply(0.12)
                    } else if hovered {
                        palette.raised.gamma_multiply(0.55)
                    } else {
                        Color32::TRANSPARENT
                    },
                    if selected {
                        Stroke::new(1.0, palette.accent.gamma_multiply(0.5))
                    } else if live {
                        Stroke::new(1.0, palette.accent.gamma_multiply(0.45))
                    } else {
                        Stroke::NONE
                    },
                    egui::StrokeKind::Inside,
                );
                let number_rect = Rect::from_min_size(
                    Pos2::new(rect.left() + 8.0, rect.top() + 8.0),
                    Vec2::new(24.0, 20.0),
                );
                ui.painter()
                    .rect_filled(number_rect, CornerRadius::same(5), palette.background);
                ui.painter().text(
                    number_rect.center(),
                    Align2::CENTER_CENTER,
                    format!("{:02}", index + 1),
                    FontId::monospace(10.0),
                    if live {
                        palette.accent
                    } else {
                        palette.text_muted
                    },
                );
                let status = widgets::status_color(palette, song.status);
                ui.painter().circle_filled(
                    Pos2::new(rect.left() + 20.0, rect.bottom() - 12.0),
                    3.0,
                    if live { palette.accent } else { status },
                );
                let text_x = rect.left() + 42.0;
                ui.painter().text(
                    Pos2::new(text_x, rect.top() + 10.0),
                    Align2::LEFT_TOP,
                    truncate(&song.display_title(), 26),
                    FontId::proportional(12.5),
                    if missing {
                        palette.danger
                    } else {
                        palette.text
                    },
                );
                let subtitle = if song.artist.trim().is_empty() {
                    if song.has_video() {
                        "Video".to_string()
                    } else {
                        "Audio".to_string()
                    }
                } else {
                    truncate(&song.artist, 24)
                };
                ui.painter().text(
                    Pos2::new(text_x, rect.top() + 32.0),
                    Align2::LEFT_TOP,
                    subtitle,
                    FontId::proportional(10.0),
                    palette.text_muted,
                );
                ui.painter().text(
                    Pos2::new(rect.right() - 9.0, rect.top() + 11.0),
                    Align2::RIGHT_TOP,
                    crate::strings::format_timecode(song.effective_duration_ms()),
                    FontId::monospace(9.5),
                    palette.text_muted,
                );
                if song.has_video() {
                    ui.painter().text(
                        Pos2::new(rect.right() - 9.0, rect.top() + 31.0),
                        Align2::RIGHT_TOP,
                        "▶",
                        FontId::proportional(9.5),
                        palette.video,
                    );
                }
                if live {
                    ui.painter().text(
                        Pos2::new(rect.right() - 9.0, rect.bottom() - 13.0),
                        Align2::RIGHT_BOTTOM,
                        "IN ONDA",
                        FontId::proportional(8.5),
                        palette.accent,
                    );
                }

                if response.clicked() {
                    app.selected_song = Some(id.clone());
                }
                if response.drag_started() {
                    if let Some(pos) = response.interact_pointer_pos() {
                        app.order_drag = Some(OrderDrag {
                            from: *index,
                            start_y: pos.y,
                            current_y: pos.y,
                        });
                    }
                }
                if let Some(drag) = &mut app.order_drag {
                    if let Some(pos) = ui.ctx().input(|input| input.pointer.interact_pos()) {
                        drag.current_y = pos.y;
                    }
                }
                let song_id = id.clone();
                response.context_menu(|ui| {
                    if ui.button("Modifica titoli").clicked() {
                        app.selected_song = Some(song_id.clone());
                        app.page = Page::Titles;
                    }
                    if ui.button("Rivela il file nel Finder").clicked() {
                        if let Some(path) = song.audio.as_ref() {
                            app.reveal_path(path);
                        }
                    }
                    ui.separator();
                    if ui.button("Rimuovi dalla scaletta").clicked() {
                        app.remove_song(*index);
                    }
                });
                let _ = stride;
            }

            if let Some(drag) = app.order_drag {
                if ui.ctx().input(|input| input.pointer.any_released()) {
                    let stride = ROW_HEIGHT + ROW_GAP;
                    let target = (drag.from as f32 + (drag.current_y - drag.start_y) / stride)
                        .round()
                        .clamp(0.0, app.project.songs.len().saturating_sub(1) as f32)
                        as usize;
                    app.order_drag = None;
                    if target != drag.from {
                        app.move_song(drag.from, target);
                    }
                }
            }
        });
}

fn timeline(app: &mut NerdApp, ui: &mut Ui) {
    let palette = app.palette;
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("Timeline")
                .size(11.0)
                .strong()
                .color(palette.text_muted),
        );
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(
                RichText::new(format!(
                    "{} brani · {} · {}",
                    app.project.songs.len(),
                    crate::strings::format_short_duration(app.project.total_duration_ms()),
                    widgets::mode_label(app.project.show.mode)
                ))
                .size(10.5)
                .color(palette.text_muted),
            );
        });
    });
    ui.add_space(4.0);

    if app.project.songs.is_empty() {
        widgets::empty_state(
            ui,
            palette,
            "Nessun brano in scaletta",
            "Aggiungi un media dalla libreria per costruire la serata.",
        );
        return;
    }

    let total_ms = app.project.total_duration_ms().max(1);
    let total_seconds = (total_ms as f32 / 1000.0).max(1.0);
    let available_width = (ui.available_width() - 6.0).max(360.0);
    let pps = ((available_width - GUTTER) / total_seconds).clamp(2.0, 18.0);
    let content_width = GUTTER + total_seconds * pps;
    let height = 252.0;
    let waveforms: Vec<Vec<f32>> = app
        .project
        .songs
        .iter()
        .map(|song| {
            song.audio
                .as_ref()
                .and_then(|path| app.library.find_by_path(path))
                .map(|asset| asset.waveform.clone())
                .unwrap_or_default()
        })
        .collect();

    let mut clicked_song = None;
    ScrollArea::horizontal()
        .id_salt("timeline-scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.set_min_width(content_width);
            let (response, painter) = ui.allocate_painter(
                Vec2::new(content_width.max(ui.available_width()), height),
                Sense::click(),
            );
            let rect = response.rect;
            painter.rect_filled(rect, CornerRadius::same(10), palette.panel);

            let ruler_y = rect.top() + 8.0;
            let video_y = rect.top() + 30.0;
            let video_h = 44.0;
            let songs_y = video_y + video_h + 8.0;
            let songs_h = 74.0;
            let bg_y = songs_y + songs_h + 8.0;
            let bg_h = 30.0;
            let title_y = bg_y + bg_h + 8.0;
            let title_h = 30.0;

            let step = if total_seconds <= 120.0 {
                10.0
            } else if total_seconds <= 600.0 {
                30.0
            } else if total_seconds <= 1_800.0 {
                60.0
            } else {
                120.0
            };
            let mut tick = 0.0;
            while tick <= total_seconds {
                let x = rect.left() + GUTTER + tick * pps;
                painter.line_segment(
                    [Pos2::new(x, ruler_y), Pos2::new(x, rect.bottom() - 8.0)],
                    Stroke::new(1.0, palette.border.gamma_multiply(0.55)),
                );
                painter.text(
                    Pos2::new(x + 4.0, ruler_y - 2.0),
                    Align2::LEFT_TOP,
                    format_time_short(tick as u64),
                    FontId::monospace(9.0),
                    palette.text_muted,
                );
                tick += step;
            }

            for (label, y, h, color) in [
                ("VIDEO", video_y, video_h, palette.video),
                ("BRANI", songs_y, songs_h, palette.accent),
                ("SOTTOFONDO", bg_y, bg_h, palette.title),
                ("TITOLI", title_y, title_h, palette.text_muted),
            ] {
                let band = Rect::from_min_size(
                    Pos2::new(rect.left() + 6.0, y),
                    Vec2::new(rect.width() - 12.0, h),
                );
                painter.rect_filled(band, CornerRadius::same(6), palette.background);
                painter.text(
                    Pos2::new(rect.left() + 12.0, y + 6.0),
                    Align2::LEFT_TOP,
                    label,
                    FontId::proportional(8.5),
                    color.gamma_multiply(0.95),
                );
            }

            let mut x = rect.left() + GUTTER;
            let mut cursor_ms = 0u64;
            for (index, song) in app.project.songs.iter().enumerate() {
                let duration_ms = song.effective_duration_ms().max(1);
                let clip_width = (duration_ms as f32 / 1000.0 * pps).max(2.0);
                let song_rect = Rect::from_min_size(
                    Pos2::new(x, songs_y + 3.0),
                    Vec2::new(clip_width - 2.0, songs_h - 6.0),
                );
                let selected = app.selected_song.as_deref() == Some(song.id.as_str());
                let color = match song.status {
                    SongStatus::Live => palette.accent,
                    SongStatus::Done => palette.success,
                    SongStatus::Skipped => palette.warning,
                    SongStatus::Pending => palette.raised,
                };
                painter.rect(
                    song_rect,
                    CornerRadius::same(5),
                    if song.status == SongStatus::Pending {
                        palette.raised
                    } else {
                        color.gamma_multiply(0.24)
                    },
                    if selected {
                        Stroke::new(1.5, palette.text)
                    } else {
                        Stroke::new(1.0, color.gamma_multiply(0.55))
                    },
                    egui::StrokeKind::Inside,
                );
                let clip_painter = painter.with_clip_rect(song_rect);
                clip_painter.text(
                    Pos2::new(song_rect.left() + 7.0, song_rect.top() + 5.0),
                    Align2::LEFT_TOP,
                    format!("{:02}  {}", index + 1, song.display_title()),
                    FontId::proportional(10.5),
                    palette.text,
                );
                if clip_width > 170.0 {
                    clip_painter.text(
                        Pos2::new(song_rect.left() + 7.0, song_rect.bottom() - 17.0),
                        Align2::LEFT_BOTTOM,
                        truncate(&song.artist, 22),
                        FontId::proportional(9.0),
                        palette.text_muted,
                    );
                }
                if let Some(peaks) = waveforms.get(index).filter(|peaks| !peaks.is_empty()) {
                    let wave_rect = Rect::from_min_max(
                        Pos2::new(song_rect.left() + 4.0, song_rect.top() + 22.0),
                        Pos2::new(song_rect.right() - 4.0, song_rect.bottom() - 3.0),
                    );
                    let clip_painter = clip_painter.with_clip_rect(wave_rect);
                    crate::widgets::waveform(
                        &clip_painter,
                        wave_rect,
                        peaks,
                        color.gamma_multiply(0.7),
                        None,
                    );
                }

                if song.has_video() {
                    painter.rect(
                        Rect::from_min_size(
                            Pos2::new(x, video_y + 4.0),
                            Vec2::new(clip_width - 2.0, video_h - 8.0),
                        ),
                        CornerRadius::same(4),
                        palette.video.gamma_multiply(0.3),
                        Stroke::new(1.0, palette.video.gamma_multiply(0.65)),
                        egui::StrokeKind::Inside,
                    );
                    if clip_width > 80.0 {
                        painter
                            .with_clip_rect(Rect::from_min_size(
                                Pos2::new(x, video_y),
                                Vec2::new(clip_width, video_h),
                            ))
                            .text(
                                Pos2::new(x + 6.0, video_y + 12.0),
                                Align2::LEFT_TOP,
                                "▶ Video",
                                FontId::proportional(9.0),
                                palette.video,
                            );
                    }
                }

                if app.project.output.show_titles {
                    let title_ms = if app.project.title_template.duration_ms == 0 {
                        duration_ms
                    } else {
                        app.project.title_template.duration_ms.min(duration_ms)
                    };
                    let title_width = (title_ms as f32 / 1000.0 * pps).max(4.0);
                    painter.rect_filled(
                        Rect::from_min_size(
                            Pos2::new(x, title_y + 7.0),
                            Vec2::new(title_width, title_h - 14.0),
                        ),
                        CornerRadius::same(3),
                        palette.title.gamma_multiply(0.35),
                    );
                }

                if clip_width > 44.0 {
                    painter.text(
                        Pos2::new(song_rect.right() - 6.0, songs_y + 5.0),
                        Align2::RIGHT_TOP,
                        format_time_short((cursor_ms + duration_ms) / 1000),
                        FontId::monospace(8.5),
                        palette.text_muted,
                    );
                }

                if let Some(pos) = response.interact_pointer_pos() {
                    if response.clicked()
                        && pos.x >= x
                        && pos.x <= x + clip_width
                        && pos.y >= songs_y
                        && pos.y <= songs_y + songs_h
                    {
                        clicked_song = Some(song.id.clone());
                    }
                }

                x += clip_width;
                cursor_ms += duration_ms;
            }

            let bg_rect = Rect::from_min_size(
                Pos2::new(rect.left() + GUTTER, bg_y + 4.0),
                Vec2::new((rect.right() - GUTTER - 8.0).max(10.0), bg_h - 8.0),
            );
            if app.project.background.tracks.is_empty() {
                painter.text(
                    Pos2::new(bg_rect.left() + 8.0, bg_rect.center().y),
                    Align2::LEFT_CENTER,
                    "Nessun sottofondo configurato",
                    FontId::proportional(9.0),
                    palette.text_muted,
                );
            } else {
                painter.rect_filled(
                    bg_rect,
                    CornerRadius::same(4),
                    palette.title.gamma_multiply(0.16),
                );
                let label = format!(
                    "{} brani{} · volume {}",
                    app.project.background.tracks.len(),
                    if app.project.background.looping {
                        " · loop"
                    } else {
                        ""
                    },
                    crate::strings::format_volume(app.project.background.volume)
                );
                painter.text(
                    Pos2::new(bg_rect.left() + 8.0, bg_rect.center().y),
                    Align2::LEFT_CENTER,
                    label,
                    FontId::proportional(9.0),
                    palette.title,
                );
            }

            if let Some(live_index) = app.show.live_index() {
                let before: u64 = app
                    .project
                    .songs
                    .iter()
                    .take(live_index)
                    .map(|song| song.effective_duration_ms())
                    .sum();
                let elapsed = if matches!(
                    app.show.phase(),
                    nerd_core::Phase::Playing | nerd_core::Phase::Paused
                ) {
                    app.show.elapsed_ms()
                } else {
                    app.project
                        .songs
                        .get(live_index)
                        .map(|song| song.effective_duration_ms())
                        .unwrap_or(0)
                };
                let playhead_x = rect.left() + GUTTER + ((before + elapsed) as f32 / 1000.0 * pps);
                painter.line_segment(
                    [
                        Pos2::new(playhead_x, ruler_y),
                        Pos2::new(playhead_x, rect.bottom() - 6.0),
                    ],
                    Stroke::new(1.5, palette.accent),
                );
                painter.add(egui::Shape::convex_polygon(
                    vec![
                        Pos2::new(playhead_x - 4.0, ruler_y - 6.0),
                        Pos2::new(playhead_x + 4.0, ruler_y - 6.0),
                        Pos2::new(playhead_x, ruler_y + 1.0),
                    ],
                    palette.accent,
                    Stroke::NONE,
                ));
            }

            let _ = response.on_hover_text("Clicca un brano per selezionarlo");
        });
    if let Some(id) = clicked_song {
        app.selected_song = Some(id);
    }

    ui.add_space(12.0);
    if let Some(song) = app.selected_song().cloned() {
        let peaks = song
            .audio
            .as_ref()
            .and_then(|path| app.library.find_by_path(path))
            .map(|asset| asset.waveform.clone())
            .unwrap_or_default();
        egui::Frame::new()
            .fill(palette.panel)
            .stroke(Stroke::new(1.0, palette.border))
            .corner_radius(CornerRadius::same(11))
            .inner_margin(egui::Margin::same(14))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!(
                            "{} · {}",
                            song.display_title(),
                            crate::strings::format_timecode(song.effective_duration_ms())
                        ))
                        .size(11.5)
                        .strong()
                        .color(palette.text),
                    );
                    if let Some(path) = &song.audio {
                        ui.label(
                            RichText::new(
                                path.file_name()
                                    .and_then(|name| name.to_str())
                                    .unwrap_or("audio"),
                            )
                            .size(10.0)
                            .color(palette.text_muted),
                        );
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.button("Inspector del brano →").clicked() {
                            app.page = Page::Titles;
                        }
                    });
                });
                let (rect, _) =
                    ui.allocate_exact_size(Vec2::new(ui.available_width(), 52.0), Sense::hover());
                if peaks.is_empty() {
                    ui.painter()
                        .rect_filled(rect, CornerRadius::same(4), palette.background);
                    ui.painter().text(
                        rect.center(),
                        Align2::CENTER_CENTER,
                        "Forma d'onda non disponibile",
                        FontId::proportional(10.0),
                        palette.text_muted,
                    );
                } else {
                    widgets::waveform(
                        ui.painter(),
                        rect,
                        &peaks,
                        palette.accent,
                        if app.show.live_index()
                            == app.project.songs.iter().position(|s| s.id == song.id)
                        {
                            Some(app.show.progress_fraction())
                        } else {
                            None
                        },
                    );
                }
            });
    }
}

fn inspector(app: &mut NerdApp, ui: &mut Ui) {
    let palette = app.palette;
    let Some(index) = app.selected_song_index() else {
        widgets::empty_state(
            ui,
            palette,
            "Nessun brano selezionato",
            "Seleziona un brano nella scaletta o nella timeline.",
        );
        return;
    };
    let before_project = app.project.clone();
    let mut song = app.project.songs[index].clone();
    let before_json = serde_json::to_string(&song).unwrap_or_default();
    let active = app.show.phase() != nerd_core::Phase::Idle || app.show.background_running();
    let structural = !active;
    let mut remove = false;
    let mut reset_status = false;
    let mut reveal_audio: Option<std::path::PathBuf> = None;
    let mut goto_titles = false;

    ScrollArea::vertical()
        .id_salt("song-inspector-scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!("Brano {:02}", index + 1))
                        .size(10.5)
                        .strong()
                        .color(palette.text_muted),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    widgets::pill(
                        ui,
                        widgets::status_label(song.status),
                        widgets::status_color(palette, song.status),
                    );
                });
            });
            ui.add_space(6.0);
            if active {
                ui.label(
                    RichText::new(
                        "Spettacolo in corso: file e tempi sono bloccati. Volume e testi restano modificabili.",
                    )
                    .size(10.0)
                    .color(palette.warning),
                );
                ui.add_space(4.0);
            }

            widgets::section_label(ui, palette, "Identità");
            let mut title = song.title.clone();
            if ui
                .add(egui::TextEdit::singleline(&mut title).hint_text("Titolo"))
                .changed()
            {
                song.title = title;
            }
            let mut artist = song.artist.clone();
            if ui
                .add(egui::TextEdit::singleline(&mut artist).hint_text("Artista"))
                .changed()
            {
                song.artist = artist;
            }

            ui.add_space(14.0);
            widgets::section_label(ui, palette, "File");
            ui.horizontal(|ui| {
                let audio_name = song
                    .audio
                    .as_ref()
                    .and_then(|path| path.file_name())
                    .and_then(|name| name.to_str())
                    .unwrap_or("Nessun audio");
                ui.label(
                    RichText::new(audio_name)
                        .size(11.0)
                        .color(if song.audio.is_some() {
                            palette.text
                        } else {
                            palette.danger
                        }),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.add_enabled_ui(structural, |ui| {
                        if ui.button("Scegli audio…").clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .set_title("Scegli audio del brano")
                                .add_filter(
                                    "Audio",
                                    &["mp3", "m4a", "wav", "aiff", "aif", "flac"],
                                )
                                .pick_file()
                            {
                                if let Ok(analysis) = nerd_audio::analyze_file(&path) {
                                    song.duration_ms = analysis.duration_ms;
                                    song.start_ms = 0;
                                    song.end_ms = None;
                                    song.audio = Some(path);
                                } else {
                                    app.toast(
                                        "Il file audio non è decodificabile.",
                                        ToastKind::Error,
                                    );
                                }
                            }
                        }
                    });
                });
            });
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                let video_name = song
                    .video
                    .as_ref()
                    .and_then(|path| path.file_name())
                    .and_then(|name| name.to_str())
                    .unwrap_or("Nessun video");
                ui.label(
                    RichText::new(video_name)
                        .size(11.0)
                        .color(if song.video.is_some() {
                            palette.video
                        } else {
                            palette.text_muted
                        }),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.add_enabled_ui(structural, |ui| {
                        if ui.button("Scegli video…").clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .set_title("Scegli video del brano")
                                .add_filter("Video", &["mp4", "mov", "m4v"])
                                .pick_file()
                            {
                                let mut duration = song.duration_ms;
                                if let Ok(metadata) = nerd_video::probe(&path) {
                                    duration = metadata.duration_ms;
                                } else if let Ok(analysis) = nerd_audio::analyze_file(&path) {
                                    duration = analysis.duration_ms;
                                }
                                song.duration_ms = duration;
                                song.audio = Some(path.clone());
                                song.video = Some(path);
                            }
                        }
                        if song.video.is_some() && ui.button("Rimuovi video").clicked() {
                            song.video = None;
                        }
                    });
                });
            });
            if let Some(path) = song.audio.clone() {
                ui.add_space(2.0);
                if ui
                    .add(
                        egui::Label::new(
                            RichText::new(truncate(&path.to_string_lossy(), 44))
                                .size(9.5)
                                .color(palette.text_muted),
                        )
                        .truncate(),
                    )
                    .on_hover_text(path.to_string_lossy().to_string())
                    .clicked()
                {
                    reveal_audio = Some(path);
                }
            }
            if song.audio.as_ref().is_none_or(|path| !path.is_file()) {
                ui.label(
                    RichText::new("⚠ Il file non è disponibile.")
                        .size(10.5)
                        .color(palette.danger),
                );
                if ui.button("Ricollega i file…").clicked() {
                    app.relink_dialog();
                }
            }

            ui.add_space(14.0);
            widgets::section_label(ui, palette, "Tempo");
            ui.add_enabled_ui(structural, |ui| {
                let mut start = song.start_ms as f64 / 1000.0;
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Inizio").size(10.5).color(palette.text_muted));
                    let max = song.duration_ms as f64 / 1000.0;
                    if ui
                        .add(
                            egui::DragValue::new(&mut start)
                                .range(0.0..=max)
                                .speed(0.1)
                                .suffix(" s"),
                        )
                        .changed()
                    {
                        song.start_ms = (start * 1000.0) as u64;
                        if song.effective_duration_ms() == 0 {
                            song.start_ms = 0;
                        }
                    }
                });
                let mut end = song.effective_end_ms() as f64 / 1000.0;
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Fine").size(10.5).color(palette.text_muted));
                    let max = song.duration_ms as f64 / 1000.0;
                    if ui
                        .add(
                            egui::DragValue::new(&mut end)
                                .range(start..=max)
                                .speed(0.1)
                                .suffix(" s"),
                        )
                        .changed()
                    {
                        song.end_ms = if (end * 1000.0) as u64 >= song.duration_ms {
                            None
                        } else {
                            Some((end * 1000.0) as u64)
                        };
                    }
                });
                ui.label(
                    RichText::new(format!(
                        "Durata effettiva {}",
                        crate::strings::format_timecode(song.effective_duration_ms())
                    ))
                    .size(10.5)
                    .color(palette.accent),
                );
            });

            ui.add_space(12.0);
            widgets::section_label(ui, palette, "Livello e dissolvenze");
            ui.horizontal(|ui| {
                ui.label(RichText::new("Volume").size(10.5).color(palette.text_muted));
                let mut volume = song.volume;
                if ui
                    .add(
                        egui::Slider::new(&mut volume, 0.0..=1.5)
                            .show_value(false)
                            .clamping(egui::SliderClamping::Always),
                    )
                    .changed()
                {
                    song.volume = volume;
                    if app.show.live_index() == Some(index) {
                        let _ = app.engine.set_song_volume(song.volume);
                    }
                }
                ui.label(
                    RichText::new(crate::strings::format_volume(song.volume))
                        .size(10.0)
                        .color(palette.text),
                );
            });
            ui.horizontal(|ui| {
                ui.label(RichText::new("Fade in").size(10.5).color(palette.text_muted));
                let mut fade = song.fade_in_ms as f64 / 1000.0;
                if ui
                    .add(
                        egui::DragValue::new(&mut fade)
                            .range(0.0..=10.0)
                            .speed(0.05)
                            .suffix(" s"),
                    )
                    .changed()
                {
                    song.fade_in_ms = (fade * 1000.0) as u64;
                }
            });
            ui.horizontal(|ui| {
                ui.label(RichText::new("Fade out").size(10.5).color(palette.text_muted));
                let mut fade = song.fade_out_ms as f64 / 1000.0;
                if ui
                    .add(
                        egui::DragValue::new(&mut fade)
                            .range(0.0..=10.0)
                            .speed(0.05)
                            .suffix(" s"),
                    )
                    .changed()
                {
                    song.fade_out_ms = (fade * 1000.0) as u64;
                }
            });
            ui.add_enabled_ui(structural, |ui| {
                let mut has_gap = song.gap_after_ms.is_some();
                if ui.checkbox(&mut has_gap, "Pausa personalizzata dopo il brano").changed() {
                    song.gap_after_ms = if has_gap {
                        Some(app.project.show.default_gap_ms)
                    } else {
                        None
                    };
                }
                if let Some(gap) = song.gap_after_ms {
                    let mut seconds = gap as f64 / 1000.0;
                    if ui
                        .add(
                            egui::DragValue::new(&mut seconds)
                                .range(0.0..=120.0)
                                .speed(0.2)
                                .suffix(" s"),
                        )
                        .changed()
                    {
                        song.gap_after_ms = Some((seconds * 1000.0) as u64);
                    }
                }
            });

            ui.add_space(14.0);
            widgets::section_label(ui, palette, "Note");
            let mut notes = song.notes.clone();
            if ui
                .add(
                    egui::TextEdit::multiline(&mut notes)
                        .desired_rows(3)
                        .hint_text("Note per il tecnico di palco…"),
                )
                .changed()
            {
                song.notes = notes;
            }

            ui.add_space(14.0);
            widgets::section_label(ui, palette, "Titolo in sovrimpressione");
            title_info_fields(ui, palette, &mut song);
            if ui.button("Apri la pagina Titoli →").clicked() {
                goto_titles = true;
            }

            ui.add_space(16.0);
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(structural, egui::Button::new("Ripristina stato"))
                    .clicked()
                {
                    reset_status = true;
                }
                if ui
                    .add(
                        egui::Button::new(
                            RichText::new("Rimuovi brano")
                                .size(11.0)
                                .color(palette.danger),
                        )
                        .frame(false),
                    )
                    .clicked()
                {
                    remove = true;
                }
            });
        });

    let after_json = serde_json::to_string(&song).unwrap_or_default();
    if after_json != before_json {
        app.project.songs[index] = song.clone();
        app.commit_edit(&before_project);
        app.refresh_missing();
        app.ensure_preparations();
    }
    if let Some(path) = reveal_audio {
        app.reveal_path(&path);
    }
    if reset_status {
        if let Some(item) = app.project.songs.get_mut(index) {
            item.status = SongStatus::Pending;
        }
        app.mark_dirty();
        app.toast("Stato del brano ripristinato.", ToastKind::Info);
    }
    if goto_titles {
        app.selected_song = Some(song.id.clone());
        app.page = Page::Titles;
    }
    if remove {
        app.remove_song(index);
    }
}

fn title_info_fields(ui: &mut Ui, palette: crate::theme::Palette, song: &mut Song) {
    let mut title = song.info.title.clone();
    if ui
        .add(egui::TextEdit::singleline(&mut title).hint_text("Titolo (sovrascrive)"))
        .changed()
    {
        song.info.title = title;
    }
    let mut artist = song.info.artist.clone();
    if ui
        .add(egui::TextEdit::singleline(&mut artist).hint_text("Artista (sovrascrive)"))
        .changed()
    {
        song.info.artist = artist;
    }
    let mut authors = song.info.authors.clone();
    if ui
        .add(egui::TextEdit::singleline(&mut authors).hint_text("Autori"))
        .changed()
    {
        song.info.authors = authors;
    }
    let mut performers = song.info.performers.clone();
    if ui
        .add(egui::TextEdit::singleline(&mut performers).hint_text("Interpreti / classe"))
        .changed()
    {
        song.info.performers = performers;
    }
    let mut extra = song.info.extra_lines.join("\n");
    if ui
        .add(
            egui::TextEdit::multiline(&mut extra)
                .desired_rows(2)
                .hint_text("Righe aggiuntive (una per riga)"),
        )
        .changed()
    {
        song.info.extra_lines = extra
            .lines()
            .map(|line| line.trim().to_string())
            .filter(|line| !line.is_empty())
            .collect();
    }
    ui.label(
        RichText::new("Le righe vuote usano titolo e artista del brano.")
            .size(9.5)
            .color(palette.text_muted),
    );
}

fn truncate(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        text.to_string()
    } else {
        let mut out: String = text.chars().take(max.saturating_sub(1)).collect();
        out.push('…');
        out
    }
}

fn format_time_short(seconds: u64) -> String {
    format!("{}:{:02}", seconds / 60, seconds % 60)
}
