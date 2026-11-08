//! Sanremo-style title card designer with a live 16:9 preview.

use std::time::Instant;

use eframe::egui::{
    self, Align, Align2, Color32, CornerRadius, FontId, Layout, Pos2, Rect, RichText, ScrollArea,
    Sense, Stroke, Ui, Vec2,
};
use nerd_core::model::{Song, TitleAnimation, TitleLine, TitlePosition, TitleTemplate};

use crate::app::NerdApp;
use crate::output;
use crate::state::{Page, ToastKind};
use crate::widgets;

pub fn show(app: &mut NerdApp, ui: &mut Ui) {
    egui::Panel::left("titles-songs")
        .resizable(true)
        .default_size(268.0)
        .min_size(220.0)
        .max_size(340.0)
        .frame(
            egui::Frame::new()
                .fill(app.palette.panel)
                .inner_margin(egui::Margin::same(14))
                .stroke(Stroke::new(1.0, app.palette.border)),
        )
        .show(ui, |ui| song_list(app, ui));

    egui::Panel::right("titles-inspector")
        .resizable(true)
        .default_size(348.0)
        .min_size(300.0)
        .max_size(470.0)
        .frame(
            egui::Frame::new()
                .fill(app.palette.panel)
                .inner_margin(egui::Margin::same(16))
                .stroke(Stroke::new(1.0, app.palette.border)),
        )
        .show(ui, |ui| {
            ScrollArea::vertical()
                .id_salt("titles-inspector-scroll")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    template_editor(app, ui);
                    ui.add_space(18.0);
                    song_editor(app, ui);
                });
        });

    egui::CentralPanel::default()
        .frame(
            egui::Frame::new()
                .fill(app.palette.background)
                .inner_margin(egui::Margin::same(20)),
        )
        .show(ui, |ui| preview(app, ui));
}

fn song_list(app: &mut NerdApp, ui: &mut Ui) {
    let palette = app.palette;
    ui.label(
        RichText::new(crate::strings::PAGE_TITLES)
            .size(22.0)
            .strong()
            .color(palette.text),
    );
    ui.label(
        RichText::new("Scegli il brano da vestire, poi modella il template.")
            .size(10.5)
            .color(palette.text_muted),
    );
    ui.add_space(10.0);
    egui::Frame::new()
        .fill(palette.accent_soft)
        .stroke(Stroke::new(1.0, palette.accent.gamma_multiply(0.4)))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(egui::Margin::same(9))
        .show(ui, |ui| {
            ui.label(
                RichText::new(&app.project.title_template.name)
                    .size(11.0)
                    .strong()
                    .color(palette.accent),
            );
            ui.label(
                RichText::new("Template attivo")
                    .size(9.5)
                    .color(palette.text_muted),
            );
        });
    ui.add_space(8.0);

    if app.project.songs.is_empty() {
        widgets::empty_state(
            ui,
            palette,
            "Nessun brano",
            "Aggiungi brani alla scaletta per creare i titoli.",
        );
        return;
    }

    ScrollArea::vertical()
        .id_salt("titles-song-list")
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
                let live = app.show.live_index() == Some(index);
                let custom = !song.info.is_empty();
                let width = ui.available_width();
                let (rect, response) =
                    ui.allocate_exact_size(Vec2::new(width, 52.0), Sense::click());
                let hovered = response.hovered();
                ui.painter().rect(
                    rect,
                    CornerRadius::same(8),
                    if selected {
                        palette.raised
                    } else if hovered {
                        palette.raised.gamma_multiply(0.55)
                    } else {
                        Color32::TRANSPARENT
                    },
                    if selected {
                        Stroke::new(1.0, palette.accent.gamma_multiply(0.5))
                    } else {
                        Stroke::NONE
                    },
                    egui::StrokeKind::Inside,
                );
                ui.painter().text(
                    Pos2::new(rect.left() + 9.0, rect.top() + 9.0),
                    Align2::LEFT_TOP,
                    truncate(&song.display_title(), 25),
                    FontId::proportional(12.0),
                    palette.text,
                );
                ui.painter().text(
                    Pos2::new(rect.left() + 9.0, rect.top() + 28.0),
                    Align2::LEFT_TOP,
                    if custom {
                        "Testo personalizzato".to_string()
                    } else {
                        "Titolo e artista del brano".to_string()
                    },
                    FontId::proportional(9.5),
                    if custom {
                        palette.title
                    } else {
                        palette.text_muted
                    },
                );
                if live {
                    ui.painter().circle_filled(
                        Pos2::new(rect.right() - 12.0, rect.center().y),
                        3.5,
                        palette.accent,
                    );
                }
                if response.clicked() {
                    app.selected_song = Some(song.id.clone());
                    app.preview_anchor = Instant::now();
                }
            }
        });
}

fn preview(app: &mut NerdApp, ui: &mut Ui) {
    let palette = app.palette;
    let Some(song) = app.selected_song().cloned().or_else(|| {
        app.project
            .songs
            .get(app.show.live_index().unwrap_or(0))
            .cloned()
    }) else {
        widgets::empty_state(
            ui,
            palette,
            "Nessun titolo da mostrare",
            "Aggiungi un brano alla scaletta.",
        );
        return;
    };

    let cycle = app.project.title_template.delay_ms
        + if app.project.title_template.duration_ms == 0 {
            8_000
        } else {
            app.project.title_template.duration_ms
        }
        + 1_500;
    let mut elapsed = app.preview_anchor.elapsed().as_millis() as u64;
    if elapsed > cycle {
        app.preview_anchor = Instant::now();
        elapsed = 0;
    }
    let state = app.projector_preview(ui.ctx(), &song, elapsed);

    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(
                RichText::new("Anteprima in tempo reale")
                    .size(22.0)
                    .strong()
                    .color(palette.text),
            );
            ui.label(
                RichText::new(
                    "Titoli, posizione, animazione e colori sono gli stessi dell'uscita video.",
                )
                .size(10.5)
                .color(palette.text_muted),
            );
        });
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if ui
                .add(
                    egui::Button::new("Riavvia anteprima")
                        .fill(palette.raised)
                        .stroke(Stroke::new(1.0, palette.border))
                        .corner_radius(CornerRadius::same(7)),
                )
                .clicked()
            {
                app.preview_anchor = Instant::now();
            }
            widgets::pill(ui, "LOOP", palette.title);
        });
    });
    ui.add_space(10.0);

    let available = ui.available_size();
    let height = (available.y * 0.68).clamp(220.0, 520.0);
    let width = (height * 16.0 / 9.0).min(available.x);
    let top = ui.cursor().top();
    let image_rect = Rect::from_min_size(
        Pos2::new(
            ui.available_rect_before_wrap().center().x - width / 2.0,
            top,
        ),
        Vec2::new(width, height),
    );
    let (_, _) = ui.allocate_exact_size(Vec2::new(available.x, height + 8.0), Sense::hover());
    ui.painter().rect(
        image_rect.expand(1.0),
        CornerRadius::same(15),
        Color32::BLACK,
        Stroke::new(1.0, palette.border),
        egui::StrokeKind::Inside,
    );
    let painter = ui.painter().with_clip_rect(image_rect);
    output::paint_output(&painter, image_rect, &state);

    ui.add_space(10.0);
    let template = &app.project.title_template;
    let delay = template.delay_ms as f32;
    let visible = if template.duration_ms == 0 {
        cycle as f32
    } else {
        template.duration_ms as f32
    };
    let total = (delay + visible + 1_500.0).max(1.0);
    let progress = (elapsed as f32 / total).clamp(0.0, 1.0);
    egui::Frame::new()
        .fill(palette.panel)
        .stroke(Stroke::new(1.0, palette.border))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(egui::Margin::symmetric(14, 10))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!(
                        "{} · delay {} · {}",
                        song.display_title(),
                        format_seconds(template.delay_ms),
                        if template.duration_ms == 0 {
                            "visibile per tutto il brano".to_string()
                        } else {
                            format!("visibile {}", format_seconds(template.duration_ms))
                        }
                    ))
                    .size(10.5)
                    .color(palette.text_muted),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(
                        RichText::new(format!("{:.1}s", elapsed as f32 / 1000.0))
                            .size(10.0)
                            .color(palette.text_muted),
                    );
                });
            });
            let (rect, _) =
                ui.allocate_exact_size(Vec2::new(ui.available_width(), 5.0), Sense::hover());
            ui.painter()
                .rect_filled(rect, CornerRadius::same(3), palette.background);
            ui.painter().rect_filled(
                Rect::from_min_size(rect.min, Vec2::new(rect.width() * progress, rect.height())),
                CornerRadius::same(3),
                palette.title,
            );
            let delay_x = rect.left() + rect.width() * (delay / total);
            ui.painter().line_segment(
                [
                    Pos2::new(delay_x, rect.top() - 2.0),
                    Pos2::new(delay_x, rect.bottom() + 2.0),
                ],
                Stroke::new(1.0, palette.accent),
            );
        });

    ui.add_space(8.0);
    ui.horizontal(|ui| {
        if widgets::accent_button(ui, palette, "Apri uscita video").clicked() && !app.output_open {
            app.toggle_output(ui.ctx());
        }
        if app.project.output.show_titles {
            widgets::pill(ui, "TITOLI ATTIVI", palette.success);
        } else {
            widgets::pill(ui, "TITOLI DISATTIVATI", palette.warning);
            if ui.button("Attiva").clicked() {
                app.project.output.show_titles = true;
                app.mark_dirty();
            }
        }
    });
}

fn template_editor(app: &mut NerdApp, ui: &mut Ui) {
    let palette = app.palette;
    let before = app.project.title_template.clone();
    let before_project = app.project.clone();
    let mut template = before.clone();

    ui.label(
        RichText::new("Template")
            .size(11.0)
            .strong()
            .color(palette.text_muted),
    );
    ui.add_space(6.0);
    ui.add(egui::TextEdit::singleline(&mut template.name).hint_text("Nome template"));
    ui.add_space(6.0);

    ui.horizontal(|ui| {
        ui.label(
            RichText::new("Posizione")
                .size(10.5)
                .color(palette.text_muted),
        );
        egui::ComboBox::from_id_salt("title-position")
            .selected_text(position_label(template.position))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut template.position, TitlePosition::Upper, "Alta");
                ui.selectable_value(&mut template.position, TitlePosition::Center, "Centro");
                ui.selectable_value(&mut template.position, TitlePosition::Lower, "Bassa");
            });
    });
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("Animazione")
                .size(10.5)
                .color(palette.text_muted),
        );
        egui::ComboBox::from_id_salt("title-animation")
            .selected_text(animation_label(template.animation))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut template.animation, TitleAnimation::None, "Nessuna");
                ui.selectable_value(&mut template.animation, TitleAnimation::Fade, "Fade");
                ui.selectable_value(&mut template.animation, TitleAnimation::SlideUp, "Salita");
                ui.selectable_value(&mut template.animation, TitleAnimation::Scale, "Scala");
            });
    });

    ui.add_space(8.0);
    ui.add(
        egui::Slider::new(&mut template.title_size_pct, 2.0..=12.0)
            .text("Titolo")
            .suffix(" %"),
    );
    ui.add(
        egui::Slider::new(&mut template.body_size_pct, 1.0..=6.0)
            .text("Testo")
            .suffix(" %"),
    );
    ui.add_space(6.0);
    color_row(ui, palette, "Titolo", &mut template.title_color);
    color_row(ui, palette, "Testo", &mut template.text_color);
    color_row(ui, palette, "Accento", &mut template.accent_color);

    ui.add_space(8.0);
    ui.checkbox(&mut template.panel, "Pannello dietro al testo");
    ui.add_enabled(
        template.panel,
        egui::Slider::new(&mut template.panel_opacity, 0.0..=1.0)
            .text("Opacità")
            .show_value(true),
    );

    ui.add_space(6.0);
    let mut delay = template.delay_ms as f64 / 1000.0;
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("Ritardo")
                .size(10.5)
                .color(palette.text_muted),
        );
        if ui
            .add(
                egui::DragValue::new(&mut delay)
                    .range(0.0..=30.0)
                    .speed(0.05)
                    .suffix(" s"),
            )
            .changed()
        {
            template.delay_ms = (delay * 1000.0) as u64;
        }
    });
    let mut whole_song = template.duration_ms == 0;
    if ui
        .checkbox(&mut whole_song, "Visibile per tutto il brano")
        .changed()
    {
        template.duration_ms = if whole_song { 0 } else { 8_500 };
    }
    if !whole_song {
        let mut duration = template.duration_ms as f64 / 1000.0;
        ui.horizontal(|ui| {
            ui.label(RichText::new("Durata").size(10.5).color(palette.text_muted));
            if ui
                .add(
                    egui::DragValue::new(&mut duration)
                        .range(0.5..=120.0)
                        .speed(0.1)
                        .suffix(" s"),
                )
                .changed()
            {
                template.duration_ms = (duration * 1000.0) as u64;
            }
        });
    }

    template.validate();
    if serde_json::to_string(&template).unwrap_or_default()
        != serde_json::to_string(&before).unwrap_or_default()
    {
        app.project.title_template = template.clone();
        app.commit_edit(&before_project);
        app.preview_anchor = Instant::now();
    }

    ui.add_space(8.0);
    if ui
        .add(
            egui::Button::new(
                RichText::new("Ripristina template di fabbrica")
                    .size(10.5)
                    .color(palette.text_muted),
            )
            .frame(false),
        )
        .clicked()
    {
        app.project.title_template = TitleTemplate::default();
        app.commit_edit(&before_project);
        app.mark_dirty();
        app.toast("Template titoli ripristinato.", ToastKind::Info);
    }
}

fn song_editor(app: &mut NerdApp, ui: &mut Ui) {
    let palette = app.palette;
    let Some(index) = app.selected_song_index() else {
        return;
    };
    let before_project = app.project.clone();
    let mut song = app.project.songs[index].clone();
    let before = serde_json::to_string(&song).unwrap_or_default();

    ui.label(
        RichText::new("Testi del brano")
            .size(11.0)
            .strong()
            .color(palette.text_muted),
    );
    ui.add_space(4.0);
    ui.label(
        RichText::new(format!("Per: {}", song.display_title()))
            .size(10.5)
            .color(palette.accent),
    );
    ui.add_space(6.0);
    ui.add(
        egui::TextEdit::singleline(&mut song.info.title)
            .hint_text("Titolo (vuoto = titolo del brano)"),
    );
    ui.add(
        egui::TextEdit::singleline(&mut song.info.artist)
            .hint_text("Artista (vuoto = artista del brano)"),
    );
    ui.add(egui::TextEdit::singleline(&mut song.info.authors).hint_text("Autori / compositori"));
    ui.add(egui::TextEdit::singleline(&mut song.info.performers).hint_text("Interpreti / classe"));
    let mut extra = song.info.extra_lines.join("\n");
    if ui
        .add(
            egui::TextEdit::multiline(&mut extra)
                .desired_rows(3)
                .hint_text("Righe aggiuntive, una per riga"),
        )
        .changed()
    {
        song.info.extra_lines = extra
            .lines()
            .map(|line| line.trim().to_string())
            .filter(|line| !line.is_empty())
            .collect();
    }

    let lines = song.info.resolved_lines(&song.title, &song.artist);
    if !lines.is_empty() {
        ui.add_space(6.0);
        ui.label(
            RichText::new("Righe in onda")
                .size(9.5)
                .color(palette.text_muted),
        );
        for line in &lines {
            let text = match line {
                TitleLine::Title(text) => text,
                TitleLine::Artist(text) => text,
                TitleLine::Meta(text) => text,
            };
            ui.label(
                RichText::new(format!("· {text}"))
                    .size(10.5)
                    .color(palette.text),
            );
        }
    }

    if serde_json::to_string(&song).unwrap_or_default() != before {
        app.project.songs[index] = song.clone();
        app.commit_edit(&before_project);
        app.preview_anchor = Instant::now();
    }

    ui.add_space(10.0);
    ui.horizontal(|ui| {
        if ui.button("Vai all'inspector del brano").clicked() {
            app.page = Page::Timeline;
        }
        if ui.button("Apri la scaletta").clicked() {
            app.page = Page::Timeline;
        }
    });
}

fn color_row(ui: &mut Ui, palette: crate::theme::Palette, label: &str, rgba: &mut [u8; 4]) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).size(10.5).color(palette.text_muted));
        let mut color = Color32::from_rgba_unmultiplied(rgba[0], rgba[1], rgba[2], rgba[3]);
        if ui.color_edit_button_srgba(&mut color).changed() {
            *rgba = color.to_array();
        }
        ui.label(
            RichText::new(format!("#{:02X}{:02X}{:02X}", rgba[0], rgba[1], rgba[2]))
                .size(9.5)
                .color(palette.text_muted)
                .monospace(),
        );
    });
}

fn position_label(position: TitlePosition) -> &'static str {
    match position {
        TitlePosition::Upper => "Alta",
        TitlePosition::Center => "Centro",
        TitlePosition::Lower => "Bassa",
    }
}

fn animation_label(animation: TitleAnimation) -> &'static str {
    match animation {
        TitleAnimation::None => "Nessuna",
        TitleAnimation::Fade => "Fade",
        TitleAnimation::SlideUp => "Salita",
        TitleAnimation::Scale => "Scala",
    }
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

fn format_seconds(ms: u64) -> String {
    format!("{:.1}s", ms as f32 / 1000.0)
}
