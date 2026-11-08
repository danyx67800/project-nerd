//! Scaletta: viewer, inspector and a multitrack timeline.

use eframe::egui::{
    self, Align, Align2, CornerRadius, Layout, Pos2, Rect, RichText, ScrollArea, Sense, Stroke, Ui,
    Vec2,
};
use nerd_core::library::MediaKind;
use nerd_core::model::{Song, SongStatus};

use crate::app::NerdApp;
use crate::state::{Page, ToastKind};
use crate::strings;
use nerd_ui_kit::{
    body, caption, icon_button, labeled_field, menu_button, menu_item, metadata_row,
    paint_texture_fit, primary_button, section, section_label, segmented, slider, status_pill,
    toggle, Button, ButtonVariant, Icon, ListRow,
};

const GUTTER: f32 = 86.0;
const TRACK_GAP: f32 = 6.0;
const RULER: f32 = 24.0;

pub fn show(app: &mut NerdApp, ui: &mut Ui) {
    toolbar(app, ui);
    if app.chrome.inspector_open {
        egui::Panel::right("song-inspector")
            .resizable(true)
            .default_size(336.0)
            .min_size(286.0)
            .max_size(460.0)
            .frame(
                egui::Frame::new()
                    .fill(app.palette.colors.panel)
                    .inner_margin(egui::Margin::same(0)),
            )
            .show(ui, |ui| inspector(app, ui));
    }
    egui::CentralPanel::default()
        .frame(
            egui::Frame::new()
                .fill(app.palette.colors.window)
                .inner_margin(egui::Margin::same(16)),
        )
        .show(ui, |ui| {
            if app.chrome.timeline_list {
                list_mode(app, ui);
            } else {
                let available = ui.available_height();
                let viewer_height = (available * 0.48).clamp(170.0, 330.0);
                viewer(app, ui, viewer_height);
                ui.add_space(12.0);
                timeline(app, ui);
            }
        });
}

fn toolbar(app: &mut NerdApp, ui: &mut Ui) {
    let theme = app.palette;
    egui::Panel::top("timeline-toolbar")
        .frame(
            egui::Frame::new()
                .fill(theme.colors.window)
                .inner_margin(egui::Margin::symmetric(18, 10)),
        )
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new("Scaletta")
                            .size(22.0)
                            .strong()
                            .color(theme.colors.text),
                    );
                    ui.label(
                        RichText::new(format!(
                            "{} brani · {} · {}",
                            app.project.songs.len(),
                            strings::format_short_duration(app.project.total_duration_ms()),
                            crate::widgets::mode_label(app.project.show.mode)
                        ))
                        .font(caption())
                        .color(theme.colors.text_secondary),
                    );
                });
                ui.add_space(16.0);
                let mut mode = if app.chrome.timeline_list { 1 } else { 0 };
                if segmented(
                    ui,
                    &theme,
                    "timeline-mode",
                    &["Timeline", "Lista"],
                    &mut mode,
                    160.0,
                )
                .changed()
                {
                    app.chrome.timeline_list = mode == 1;
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if Button::new("Salva")
                        .variant(ButtonVariant::Primary)
                        .icon(Icon::Save)
                        .show(ui, &theme)
                        .clicked()
                    {
                        app.save(false);
                    }
                    menu_button(
                        ui,
                        &theme,
                        "timeline-add",
                        "Aggiungi",
                        Some(Icon::Plus),
                        ButtonVariant::Secondary,
                        112.0,
                        |ui| {
                            let mut assets = app.library.assets.clone();
                            assets.sort_by(|a, b| {
                                a.display_title()
                                    .to_lowercase()
                                    .cmp(&b.display_title().to_lowercase())
                            });
                            if assets.is_empty() {
                                ui.label(
                                    RichText::new("La libreria è vuota.")
                                        .font(caption())
                                        .color(theme.colors.text_secondary),
                                );
                            }
                            for asset in assets {
                                let label = format!(
                                    "{}  ·  {}",
                                    asset.display_title(),
                                    strings::format_timecode(asset.duration_ms)
                                );
                                if menu_item(ui, &theme, &label, Some(kind_icon(asset.kind)))
                                    .clicked()
                                {
                                    app.add_asset_to_show(&asset.id);
                                }
                            }
                            nerd_ui_kit::divider(ui, &theme);
                            if menu_item(ui, &theme, "Importa media…", Some(Icon::File)).clicked()
                            {
                                app.import_dialog();
                            }
                        },
                    );
                });
            });
        });
}

fn viewer(app: &mut NerdApp, ui: &mut Ui, height: f32) {
    let theme = app.palette;
    let selected = app
        .selected_song()
        .cloned()
        .or_else(|| app.project.songs.first().cloned());
    let Some(song) = selected else {
        nerd_ui_kit::underlay(ui, &theme, |ui| {
            ui.set_min_height(height);
            nerd_ui_kit::empty_state(
                ui,
                &theme,
                Icon::Timeline,
                "La scaletta è vuota",
                "Aggiungi un brano dalla libreria o importa i file della serata.",
                |ui| {
                    if primary_button(ui, &theme, "Apri la libreria").clicked() {
                        app.page = Page::Library;
                    }
                },
            );
        });
        return;
    };

    let width = ui.available_width();
    let canvas_height = (height - 52.0).max(120.0);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, canvas_height), Sense::hover());
    let canvas = nerd_ui_kit::fit_rect(rect, 16.0 / 9.0);
    ui.painter()
        .rect_filled(canvas, CornerRadius::same(9), theme.colors.blackout);
    let asset = song
        .audio
        .as_ref()
        .and_then(|path| app.library.find_by_path(path))
        .cloned();
    let texture = asset
        .as_ref()
        .and_then(|asset| app.thumbnail_for(ui.ctx(), asset));
    if let Some(texture) = &texture {
        paint_texture_fit(ui.painter(), texture, canvas);
    } else {
        let peaks = asset
            .as_ref()
            .map(|asset| asset.waveform.as_slice())
            .unwrap_or(&[]);
        let wave = Rect::from_center_size(
            canvas.center(),
            Vec2::new(canvas.width() * 0.78, canvas.height() * 0.34),
        );
        nerd_ui_kit::waveform(
            ui.painter(),
            wave,
            peaks,
            if song.has_video() {
                theme.colors.video
            } else {
                theme.colors.accent
            },
            Some(app.show.progress_fraction()),
        );
        ui.painter().text(
            Pos2::new(canvas.left() + 18.0, canvas.top() + 16.0),
            Align2::LEFT_TOP,
            if song.has_video() {
                "VIDEO MUSICALE"
            } else {
                "ANTEPRIMA AUDIO"
            },
            caption(),
            theme.colors.text_tertiary,
        );
        ui.painter().text(
            Pos2::new(canvas.left() + 18.0, canvas.bottom() - 24.0),
            Align2::LEFT_BOTTOM,
            song.display_title(),
            nerd_ui_kit::section_title(),
            theme.colors.text,
        );
    }
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(format!(
                "{}  /  {}",
                strings::format_timecode(if app.show.live_index().is_some() {
                    app.show.elapsed_ms()
                } else {
                    0
                }),
                strings::format_timecode(song.effective_duration_ms())
            ))
            .font(nerd_ui_kit::timecode(11.0))
            .color(theme.colors.text_secondary),
        );
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if Button::new("Titoli")
                .variant(ButtonVariant::Plain)
                .icon(Icon::Titles)
                .show(ui, &theme)
                .clicked()
            {
                app.selected_song = Some(song.id.clone());
                app.page = Page::Titles;
            }
            if song.has_video() {
                status_pill(ui, &theme, "Video", theme.colors.video, Some(Icon::Video));
            }
            if !song.audio.as_ref().is_some_and(|path| path.is_file()) {
                status_pill(
                    ui,
                    &theme,
                    "File mancante",
                    theme.colors.red,
                    Some(Icon::Warning),
                );
            }
        });
    });
}

fn timeline(app: &mut NerdApp, ui: &mut Ui) {
    let theme = app.palette;
    ui.horizontal(|ui| {
        section_label(ui, &theme, "Timeline");
        ui.add_space(8.0);
        ui.label(
            RichText::new(strings::format_short_duration(
                app.project.total_duration_ms(),
            ))
            .font(nerd_ui_kit::timecode(11.0))
            .color(theme.colors.text_secondary),
        );
        ui.add_space(10.0);
        let mut snap = app.chrome.timeline_snap;
        if toggle(ui, &theme, &mut snap, "Snap").changed() {
            app.chrome.timeline_snap = snap;
        }
        ui.add_space((ui.available_width() - 178.0).max(6.0));
        let mut zoom = app.chrome.timeline_zoom;
        if icon_button(ui, &theme, Icon::Minus, Some("Riduci zoom"), false).clicked() {
            zoom = (zoom - 0.15).max(0.6);
        }
        ui.label(
            RichText::new(format!("{:.0}%", zoom * 100.0))
                .font(caption())
                .color(theme.colors.text_secondary),
        );
        if icon_button(ui, &theme, Icon::Plus, Some("Aumenta zoom"), false).clicked() {
            zoom = (zoom + 0.15).min(2.4);
        }
        app.chrome.timeline_zoom = zoom;
    });
    ui.add_space(6.0);

    if app.project.songs.is_empty() {
        nerd_ui_kit::empty_state(
            ui,
            &theme,
            Icon::Timeline,
            "Nessun brano in scaletta",
            "Aggiungi media per costruire la serata.",
            |ui| {
                if primary_button(ui, &theme, "Apri la libreria").clicked() {
                    app.page = Page::Library;
                }
            },
        );
        return;
    }

    let total_ms = app.project.total_duration_ms().max(1);
    let total_s = (total_ms as f32 / 1000.0).max(1.0);
    let available = ui.available_width().max(420.0);
    let pps = ((available - GUTTER) / total_s).clamp(1.2, 22.0) * app.chrome.timeline_zoom;
    let content_width = (GUTTER + total_s * pps).max(available);
    let height = 236.0;
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

    let mut clicked = None;
    ScrollArea::horizontal()
        .id_salt("timeline-scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.set_min_width(content_width);
            let (response, painter) =
                ui.allocate_painter(Vec2::new(content_width, height), Sense::click_and_drag());
            let rect = response.rect;
            painter.rect_filled(rect, CornerRadius::same(9), theme.colors.panel);
            painter.rect_filled(
                Rect::from_min_size(rect.min, Vec2::new(GUTTER, rect.height())),
                CornerRadius::same(9),
                theme.colors.card,
            );
            painter.line_segment(
                [
                    Pos2::new(rect.left() + GUTTER, rect.top()),
                    Pos2::new(rect.left() + GUTTER, rect.bottom()),
                ],
                Stroke::new(1.0, theme.colors.separator_strong),
            );

            let video_y = rect.top() + RULER + 8.0;
            let video_h = 34.0;
            let songs_y = video_y + video_h + TRACK_GAP;
            let songs_h = 64.0;
            let bg_y = songs_y + songs_h + TRACK_GAP;
            let bg_h = 28.0;
            let title_y = bg_y + bg_h + TRACK_GAP;
            let title_h = 28.0;

            for (label, y, h, tint) in [
                ("VIDEO", video_y, video_h, theme.colors.video),
                ("BRANI", songs_y, songs_h, theme.colors.green),
                ("SOTTOFONDO", bg_y, bg_h, theme.colors.mint),
                ("TITOLI", title_y, title_h, theme.colors.title),
            ] {
                let band = Rect::from_min_size(
                    Pos2::new(rect.left() + 8.0, y),
                    Vec2::new(rect.width() - 16.0, h),
                );
                painter.rect_filled(band, CornerRadius::same(5), theme.colors.window);
                painter.text(
                    Pos2::new(rect.left() + 16.0, y + 7.0),
                    Align2::LEFT_TOP,
                    label,
                    caption(),
                    tint,
                );
            }

            let step = if total_s <= 90.0 {
                5.0
            } else if total_s <= 360.0 {
                15.0
            } else {
                60.0
            };
            let mut tick = 0.0;
            while tick <= total_s {
                let x = rect.left() + GUTTER + tick * pps;
                if x <= rect.right() {
                    painter.line_segment(
                        [
                            Pos2::new(x, rect.top() + 8.0),
                            Pos2::new(x, rect.bottom() - 8.0),
                        ],
                        Stroke::new(1.0, theme.colors.separator),
                    );
                    painter.text(
                        Pos2::new(x + 4.0, rect.top() + 7.0),
                        Align2::LEFT_TOP,
                        short_time(tick as u64),
                        nerd_ui_kit::timecode(9.0),
                        theme.colors.text_tertiary,
                    );
                }
                tick += step;
            }

            let mut x = rect.left() + GUTTER;
            let mut cursor = 0u64;
            for (index, song) in app.project.songs.iter().enumerate() {
                let duration = song.effective_duration_ms().max(1);
                let clip_w = (duration as f32 / 1000.0 * pps).max(2.0);
                let selected = app.selected_song.as_deref() == Some(song.id.as_str());
                let song_rect = Rect::from_min_size(
                    Pos2::new(x, songs_y + 3.0),
                    Vec2::new(clip_w - 2.0, songs_h - 6.0),
                );
                let color = match song.status {
                    SongStatus::Live => theme.colors.green,
                    SongStatus::Done => theme.colors.green,
                    SongStatus::Skipped => theme.colors.amber,
                    SongStatus::Pending => theme.colors.card,
                };
                painter.rect(
                    song_rect,
                    CornerRadius::same(6),
                    if song.status == SongStatus::Pending {
                        theme.colors.card
                    } else {
                        color.gamma_multiply(0.22)
                    },
                    Stroke::new(
                        if selected { 2.0 } else { 1.0 },
                        if selected {
                            theme.colors.accent
                        } else {
                            color.gamma_multiply(0.55)
                        },
                    ),
                    egui::StrokeKind::Inside,
                );
                let inner = painter.with_clip_rect(song_rect);
                if clip_w > 58.0 {
                    nerd_ui_kit::paint_truncated(
                        &inner,
                        Pos2::new(song_rect.left() + 7.0, song_rect.top() + 5.0),
                        &format!("{:02}  {}", index + 1, song.display_title()),
                        body(),
                        theme.colors.text,
                        (clip_w - 14.0).max(10.0),
                    );
                }
                if let Some(peaks) = waveforms.get(index).filter(|peaks| !peaks.is_empty()) {
                    let wave = Rect::from_min_max(
                        Pos2::new(song_rect.left() + 4.0, song_rect.top() + 22.0),
                        Pos2::new(song_rect.right() - 4.0, song_rect.bottom() - 4.0),
                    );
                    nerd_ui_kit::waveform(&inner, wave, peaks, color.gamma_multiply(0.75), None);
                }
                if song.has_video() {
                    painter.rect(
                        Rect::from_min_size(
                            Pos2::new(x, video_y + 4.0),
                            Vec2::new(clip_w - 2.0, video_h - 8.0),
                        ),
                        CornerRadius::same(4),
                        theme.colors.video.gamma_multiply(0.3),
                        Stroke::new(1.0, theme.colors.video.gamma_multiply(0.6)),
                        egui::StrokeKind::Inside,
                    );
                }
                if app.project.output.show_titles {
                    let title_ms = if app.project.title_template.duration_ms == 0 {
                        duration
                    } else {
                        app.project.title_template.duration_ms.min(duration)
                    };
                    let title_w = (title_ms as f32 / 1000.0 * pps).max(4.0);
                    painter.rect_filled(
                        Rect::from_min_size(
                            Pos2::new(x, title_y + 7.0),
                            Vec2::new(title_w, title_h - 14.0),
                        ),
                        CornerRadius::same(3),
                        theme.colors.title.gamma_multiply(0.36),
                    );
                }
                if let Some(pos) = response.interact_pointer_pos() {
                    if response.clicked()
                        && pos.x >= x
                        && pos.x <= x + clip_w
                        && pos.y >= songs_y
                        && pos.y <= songs_y + songs_h
                    {
                        clicked = Some(song.id.clone());
                    }
                }
                x += clip_w;
                cursor += duration;
            }
            let _ = cursor;

            let bg_rect = Rect::from_min_size(
                Pos2::new(rect.left() + GUTTER, bg_y + 4.0),
                Vec2::new((rect.right() - GUTTER - 8.0).max(10.0), bg_h - 8.0),
            );
            painter.rect_filled(
                bg_rect,
                CornerRadius::same(4),
                theme.colors.mint.gamma_multiply(0.16),
            );
            painter.text(
                Pos2::new(bg_rect.left() + 7.0, bg_rect.center().y),
                Align2::LEFT_CENTER,
                if app.project.background.tracks.is_empty() {
                    "Nessun sottofondo configurato".to_string()
                } else {
                    format!(
                        "{} brani{}",
                        app.project.background.tracks.len(),
                        if app.project.background.looping {
                            " · loop"
                        } else {
                            ""
                        }
                    )
                },
                caption(),
                theme.colors.mint,
            );

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
                    0
                };
                let playhead_x = rect.left() + GUTTER + ((before + elapsed) as f32 / 1000.0 * pps);
                painter.line_segment(
                    [
                        Pos2::new(playhead_x, rect.top() + 6.0),
                        Pos2::new(playhead_x, rect.bottom() - 6.0),
                    ],
                    Stroke::new(1.5, theme.colors.accent),
                );
                painter.add(egui::Shape::convex_polygon(
                    vec![
                        Pos2::new(playhead_x - 4.0, rect.top() + 6.0),
                        Pos2::new(playhead_x + 4.0, rect.top() + 6.0),
                        Pos2::new(playhead_x, rect.top() + 13.0),
                    ],
                    theme.colors.accent,
                    Stroke::NONE,
                ));
            }
            if response.has_focus() {
                painter.rect_stroke(
                    rect.shrink(1.0),
                    CornerRadius::same(9),
                    Stroke::new(2.0, theme.colors.accent),
                    egui::StrokeKind::Inside,
                );
            }
        });
    if let Some(id) = clicked {
        app.selected_song = Some(id);
    }
}

fn list_mode(app: &mut NerdApp, ui: &mut Ui) {
    let theme = app.palette;
    if app.project.songs.is_empty() {
        nerd_ui_kit::empty_state(
            ui,
            &theme,
            Icon::List,
            "Nessun brano",
            "La lista mostra lo stesso ordine della timeline.",
            |_ui| {},
        );
        return;
    }
    ScrollArea::vertical()
        .id_salt("running-order-scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let songs: Vec<(usize, Song)> = app
                .project
                .songs
                .iter()
                .enumerate()
                .map(|(index, song)| (index, song.clone()))
                .collect();
            for (index, song) in songs {
                let selected = app.selected_song.as_deref() == Some(song.id.as_str());
                let subtitle = format!(
                    "{:02} · {} · {}",
                    index + 1,
                    if song.artist.is_empty() {
                        "Senza artista"
                    } else {
                        &song.artist
                    },
                    crate::widgets::status_label(song.status)
                );
                let title = song.display_title();
                let response = ListRow {
                    title: &title,
                    subtitle: &subtitle,
                    duration: &strings::format_timecode(song.effective_duration_ms()),
                    icon: if song.has_video() {
                        Icon::Video
                    } else {
                        Icon::Music
                    },
                    accent: if song.has_video() {
                        theme.colors.video
                    } else {
                        theme.colors.green
                    },
                    selected,
                    badge: (app.show.live_index() == Some(index)).then_some("In onda"),
                    waveform: None,
                    thumbnail: None,
                    trailing_icon: Some(Icon::More),
                    missing: song.audio.as_ref().is_none_or(|path| !path.is_file()),
                }
                .show(ui, &theme);
                if response.clicked() {
                    app.selected_song = Some(song.id.clone());
                }
                if response.drag_started() {
                    app.order_drag = Some(crate::app::OrderDrag {
                        from: index,
                        start_y: response.rect.top(),
                        current_y: response.rect.top(),
                    });
                }
                if let Some(drag) = &mut app.order_drag {
                    if let Some(pos) = ui.ctx().input(|input| input.pointer.interact_pos()) {
                        drag.current_y = pos.y;
                    }
                }
                let id = song.id.clone();
                response.context_menu(|ui| {
                    if menu_item(ui, &theme, "Sposta su", Some(Icon::ChevronUp)).clicked()
                        && index > 0
                    {
                        app.move_song(index, index - 1);
                    }
                    if menu_item(ui, &theme, "Sposta giù", Some(Icon::ChevronDown)).clicked() {
                        app.move_song(index, index + 1);
                    }
                    nerd_ui_kit::divider(ui, &theme);
                    if menu_item(ui, &theme, "Rimuovi dalla scaletta", Some(Icon::Trash)).clicked()
                    {
                        app.remove_song(index);
                    }
                    let _ = &id;
                });
            }
            if let Some(drag) = app.order_drag {
                if ui.ctx().input(|input| input.pointer.any_released()) {
                    let stride = theme.metrics.row + 4.0;
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
            ui.add_space(10.0);
        });
}

fn inspector(app: &mut NerdApp, ui: &mut Ui) {
    let theme = app.palette;
    if !app.chrome.inspector_open {
        return;
    }
    let Some(index) = app.selected_song_index() else {
        nerd_ui_kit::empty_state(
            ui,
            &theme,
            Icon::Inspector,
            "Nessun brano",
            "Seleziona un brano nella timeline o nell'elenco.",
            |_ui| {},
        );
        return;
    };
    let before = app.project.clone();
    let mut song = app.project.songs[index].clone();
    let before_json = serde_json::to_string(&song).unwrap_or_default();
    let active = app.show.phase() != nerd_core::Phase::Idle || app.show.background_running();
    let mut remove = false;
    let mut reset = false;

    ScrollArea::vertical()
        .id_salt("song-inspector-scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                ui.add_space(16.0);
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(format!("Brano {:02}", index + 1))
                            .size(15.0)
                            .strong()
                            .color(theme.colors.text),
                    );
                    ui.horizontal(|ui| {
                        status_pill(
                            ui,
                            &theme,
                            crate::widgets::status_label(song.status),
                            crate::widgets::status_color(theme, song.status),
                            None,
                        );
                        if active {
                            status_pill(ui, &theme, "Bloccato", theme.colors.amber, None);
                        }
                    });
                });
            });
            ui.add_space(8.0);
            nerd_ui_kit::divider(ui, &theme);

            section(ui, &theme, "timeline-info", "Info", true, |ui| {
                ui.horizontal(|ui| {
                    ui.add_space(16.0);
                    ui.vertical(|ui| {
                        ui.set_width((ui.available_width() - 16.0).max(130.0));
                        labeled_field(ui, &theme, "Titolo", &mut song.title, "Titolo");
                        ui.add_space(6.0);
                        labeled_field(ui, &theme, "Artista", &mut song.artist, "Artista");
                        ui.add_space(6.0);
                        let mut duration = song.duration_ms as f64 / 1000.0;
                        nerd_ui_kit::number_field(
                            ui,
                            &theme,
                            "Durata file",
                            &mut duration,
                            0.0..=3_600.0,
                            " s",
                            0.5,
                        );
                        song.duration_ms = (duration * 1000.0) as u64;
                    });
                });
            });

            section(ui, &theme, "timeline-times", "Tempi", true, |ui| {
                ui.horizontal(|ui| {
                    ui.add_space(16.0);
                    ui.vertical(|ui| {
                        ui.set_width((ui.available_width() - 16.0).max(130.0));
                        let mut start = song.effective_start_ms() as f64 / 1000.0;
                        let max = song.duration_ms as f64 / 1000.0;
                        nerd_ui_kit::number_field(
                            ui,
                            &theme,
                            "Inizio",
                            &mut start,
                            0.0..=max,
                            " s",
                            0.1,
                        );
                        song.start_ms = (start * 1000.0) as u64;
                        ui.add_space(6.0);
                        let mut end = song.effective_end_ms() as f64 / 1000.0;
                        nerd_ui_kit::number_field(
                            ui,
                            &theme,
                            "Fine",
                            &mut end,
                            start..=max,
                            " s",
                            0.1,
                        );
                        song.end_ms = Some((end * 1000.0) as u64);
                        ui.add_space(4.0);
                        metadata_row(
                            ui,
                            &theme,
                            "Durata effettiva",
                            &strings::format_timecode(song.effective_duration_ms()),
                        );
                    });
                });
            });

            section(ui, &theme, "timeline-audio", "Audio", true, |ui| {
                ui.horizontal(|ui| {
                    ui.add_space(16.0);
                    ui.vertical(|ui| {
                        ui.set_width((ui.available_width() - 16.0).max(130.0));
                        let name = song
                            .audio
                            .as_ref()
                            .and_then(|path| path.file_name())
                            .and_then(|name| name.to_str())
                            .unwrap_or("Nessun audio");
                        ui.label(RichText::new(name).font(caption()).color(
                            if song.audio.is_some() {
                                theme.colors.text
                            } else {
                                theme.colors.red
                            },
                        ));
                        if !active
                            && Button::new("Scegli audio…")
                                .variant(ButtonVariant::Secondary)
                                .show(ui, &theme)
                                .clicked()
                        {
                            if let Some(path) = rfd::FileDialog::new()
                                .set_title("Scegli audio del brano")
                                .add_filter("Audio", &["mp3", "m4a", "wav", "aiff", "aif", "flac"])
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
                        let mut volume = song.volume;
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("Volume")
                                    .font(caption())
                                    .color(theme.colors.text_secondary),
                            );
                            if slider(ui, &theme, &mut volume, 0.0..=1.5, "", 120.0).changed() {
                                song.volume = volume;
                                if app.show.live_index() == Some(index) {
                                    let _ = app.engine.set_song_volume(volume);
                                }
                            }
                            ui.label(
                                RichText::new(strings::format_volume(volume))
                                    .font(caption())
                                    .color(theme.colors.text),
                            );
                        });
                        ui.add_space(4.0);
                        let mut fade_in = song.fade_in_ms as f32 / 1000.0;
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("Fade in")
                                    .font(caption())
                                    .color(theme.colors.text_secondary),
                            );
                            if slider(ui, &theme, &mut fade_in, 0.0..=10.0, " s", 110.0).changed() {
                                song.fade_in_ms = (fade_in * 1000.0) as u64;
                            }
                        });
                        let mut fade_out = song.fade_out_ms as f32 / 1000.0;
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("Fade out")
                                    .font(caption())
                                    .color(theme.colors.text_secondary),
                            );
                            if slider(ui, &theme, &mut fade_out, 0.0..=10.0, " s", 110.0).changed()
                            {
                                song.fade_out_ms = (fade_out * 1000.0) as u64;
                            }
                        });
                    });
                });
            });

            section(ui, &theme, "timeline-note", "Note", true, |ui| {
                ui.horizontal(|ui| {
                    ui.add_space(16.0);
                    ui.vertical(|ui| {
                        ui.set_width((ui.available_width() - 16.0).max(130.0));
                        let mut notes = song.notes.clone();
                        if nerd_ui_kit::text_field(ui, &theme, &mut notes, "Note per il palco")
                            .changed()
                        {
                            song.notes = notes;
                        }
                        ui.add_space(6.0);
                        let mut has_gap = song.gap_after_ms.is_some();
                        if toggle(ui, &theme, &mut has_gap, "Pausa personalizzata").changed() {
                            song.gap_after_ms = has_gap.then_some(app.project.show.default_gap_ms);
                        }
                    });
                });
            });

            ui.add_space(12.0);
            ui.horizontal(|ui| {
                ui.add_space(16.0);
                if Button::new("Ripristina stato")
                    .variant(ButtonVariant::Plain)
                    .show(ui, &theme)
                    .clicked()
                {
                    reset = true;
                }
                if Button::new("Rimuovi")
                    .variant(ButtonVariant::Destructive)
                    .icon(Icon::Trash)
                    .show(ui, &theme)
                    .clicked()
                {
                    remove = true;
                }
            });
            ui.add_space(16.0);
        });

    if serde_json::to_string(&song).unwrap_or_default() != before_json {
        app.project.songs[index] = song.clone();
        app.commit_edit(&before);
        app.refresh_missing();
        app.ensure_preparations();
    }
    if reset {
        if let Some(item) = app.project.songs.get_mut(index) {
            item.status = SongStatus::Pending;
        }
        app.mark_dirty();
    }
    if remove {
        app.remove_song(index);
    }
}

fn kind_icon(kind: MediaKind) -> Icon {
    match kind {
        MediaKind::Audio => Icon::Music,
        MediaKind::Video => Icon::Video,
    }
}

fn short_time(seconds: u64) -> String {
    format!("{}:{:02}", seconds / 60, seconds % 60)
}
