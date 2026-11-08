//! Titoli: template gallery, live 16:9 preview and text inspector.

use std::time::Instant;

use eframe::egui::{
    self, Align, Align2, CornerRadius, Layout, Pos2, Rect, RichText, ScrollArea, Sense, Ui, Vec2,
};
use nerd_core::model::{Song, TitleAnimation, TitleLine, TitlePosition, TitleTemplate};

use crate::app::NerdApp;
use crate::output;
use crate::state::Page;
use crate::strings;
use nerd_ui_kit::{
    body, caption, icon_button, labeled_field, primary_button, section, section_label, segmented,
    slider, status_pill, toggle, Button, ButtonVariant, Icon, Theme,
};

pub fn show(app: &mut NerdApp, ui: &mut Ui) {
    if app.chrome.titles_sidebar {
        egui::Panel::left("titles-browser")
            .resizable(true)
            .default_size(258.0)
            .min_size(220.0)
            .max_size(340.0)
            .frame(
                egui::Frame::new()
                    .fill(app.palette.colors.panel)
                    .inner_margin(egui::Margin::symmetric(12, 12)),
            )
            .show(ui, |ui| browser(app, ui));
    }
    if app.chrome.inspector_open {
        egui::Panel::right("titles-inspector")
            .resizable(true)
            .default_size(344.0)
            .min_size(300.0)
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
                .inner_margin(egui::Margin::same(18)),
        )
        .show(ui, |ui| preview(app, ui));
}

fn browser(app: &mut NerdApp, ui: &mut Ui) {
    let theme = app.palette;
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("Titoli")
                .size(15.0)
                .strong()
                .color(theme.colors.text),
        );
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if icon_button(
                ui,
                &theme,
                Icon::Sidebar,
                Some("Nascondi la barra"),
                app.chrome.titles_sidebar,
            )
            .clicked()
            {
                app.chrome.titles_sidebar = false;
            }
        });
    });
    ui.label(
        RichText::new("Scegli un template, poi vesti il brano.")
            .font(caption())
            .color(theme.colors.text_secondary),
    );
    ui.add_space(10.0);
    section_label(ui, &theme, "Template");
    ui.add_space(4.0);
    ScrollArea::vertical()
        .id_salt("title-templates")
        .max_height(222.0)
        .auto_shrink([false, true])
        .show(ui, |ui| {
            for (name, subtitle, preset) in [
                ("Classica", "Pannello · basso", Preset::Classic),
                ("Minimal", "Solo testo · centro", Preset::Minimal),
                ("Serata", "Accento · alto", Preset::Show),
            ] {
                template_card(app, ui, &theme, name, subtitle, preset);
            }
        });
    ui.add_space(8.0);
    section_label(ui, &theme, "Brani");
    ui.add_space(4.0);
    if app.project.songs.is_empty() {
        nerd_ui_kit::empty_state(
            ui,
            &theme,
            Icon::Music,
            "Nessun brano",
            "Aggiungi brani alla scaletta.",
            |_ui| {},
        );
        return;
    }
    let songs: Vec<Song> = app.project.songs.clone();
    ScrollArea::vertical()
        .id_salt("title-songs")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for (index, song) in songs.iter().enumerate() {
                let selected = app.selected_song.as_deref() == Some(song.id.as_str());
                let live = app.show.live_index() == Some(index);
                let custom = !song.info.is_empty();
                let width = ui.available_width();
                let (rect, response) =
                    ui.allocate_exact_size(Vec2::new(width, 46.0), Sense::click());
                if selected || response.hovered() {
                    ui.painter().rect_filled(
                        rect,
                        CornerRadius::same(6),
                        if selected {
                            theme.colors.accent_soft
                        } else {
                            theme.colors.hover
                        },
                    );
                }
                ui.painter().text(
                    Pos2::new(rect.left() + 9.0, rect.top() + 8.0),
                    Align2::LEFT_TOP,
                    truncate(&song.display_title(), 26),
                    body(),
                    theme.colors.text,
                );
                ui.painter().text(
                    Pos2::new(rect.left() + 9.0, rect.top() + 26.0),
                    Align2::LEFT_TOP,
                    if custom {
                        "Testo personalizzato".to_string()
                    } else {
                        "Titolo del brano".to_string()
                    },
                    caption(),
                    if custom {
                        theme.colors.title
                    } else {
                        theme.colors.text_tertiary
                    },
                );
                if live {
                    ui.painter().circle_filled(
                        Pos2::new(rect.right() - 12.0, rect.center().y),
                        3.5,
                        theme.colors.green,
                    );
                }
                if response.clicked() {
                    app.selected_song = Some(song.id.clone());
                    app.preview_anchor = Instant::now();
                }
            }
            ui.add_space(10.0);
        });
}

#[derive(Clone, Copy)]
enum Preset {
    Classic,
    Minimal,
    Show,
}

fn template_card(
    app: &mut NerdApp,
    ui: &mut Ui,
    theme: &Theme,
    name: &str,
    subtitle: &str,
    preset: Preset,
) {
    let width = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 64.0), Sense::click());
    let active = app.project.title_template.name == name;
    if active || response.hovered() {
        ui.painter().rect(
            rect,
            CornerRadius::same(7),
            if active {
                theme.colors.accent_soft
            } else {
                theme.colors.hover
            },
            egui::Stroke::new(
                1.0,
                if active {
                    theme.colors.accent
                } else {
                    theme.colors.separator
                },
            ),
            egui::StrokeKind::Inside,
        );
    }
    let preview = Rect::from_min_size(
        Pos2::new(rect.left() + 7.0, rect.top() + 7.0),
        Vec2::new(56.0, 32.0),
    );
    ui.painter()
        .rect_filled(preview, CornerRadius::same(3), theme.colors.well);
    let line_y = match preset {
        Preset::Classic => preview.top() + 20.0,
        Preset::Minimal => preview.center().y,
        Preset::Show => preview.top() + 8.0,
    };
    ui.painter().rect_filled(
        Rect::from_min_size(
            Pos2::new(preview.left() + 7.0, line_y),
            Vec2::new(42.0, 2.0),
        ),
        CornerRadius::same(1),
        theme.colors.text,
    );
    ui.painter().rect_filled(
        Rect::from_min_size(
            Pos2::new(preview.left() + 7.0, line_y + 5.0),
            Vec2::new(27.0, 1.5),
        ),
        CornerRadius::same(1),
        theme.colors.text_secondary,
    );
    ui.painter().text(
        Pos2::new(preview.right() + 8.0, rect.top() + 14.0),
        Align2::LEFT_TOP,
        name,
        body(),
        theme.colors.text,
    );
    ui.painter().text(
        Pos2::new(preview.right() + 8.0, rect.top() + 33.0),
        Align2::LEFT_TOP,
        subtitle,
        caption(),
        theme.colors.text_secondary,
    );
    if response.clicked() {
        let before = app.project.clone();
        let mut template = app.project.title_template.clone();
        template.name = name.to_string();
        match preset {
            Preset::Classic => {
                template.position = TitlePosition::Lower;
                template.panel = true;
                template.panel_opacity = 0.55;
                template.title_size_pct = 5.2;
                template.body_size_pct = 2.3;
                template.animation = TitleAnimation::SlideUp;
            }
            Preset::Minimal => {
                template.position = TitlePosition::Center;
                template.panel = false;
                template.panel_opacity = 0.0;
                template.title_size_pct = 4.2;
                template.body_size_pct = 1.8;
                template.animation = TitleAnimation::Fade;
            }
            Preset::Show => {
                template.position = TitlePosition::Upper;
                template.panel = true;
                template.panel_opacity = 0.36;
                template.title_size_pct = 6.4;
                template.body_size_pct = 2.6;
                template.animation = TitleAnimation::Scale;
            }
        }
        app.project.title_template = template;
        app.commit_edit(&before);
        app.preview_anchor = Instant::now();
    }
}

fn preview(app: &mut NerdApp, ui: &mut Ui) {
    let theme = app.palette;
    let Some(song) = app.selected_song().cloned().or_else(|| {
        app.project
            .songs
            .get(app.show.live_index().unwrap_or(0))
            .cloned()
    }) else {
        nerd_ui_kit::empty_state(
            ui,
            &theme,
            Icon::Titles,
            "Nessun titolo",
            "Aggiungi un brano alla scaletta per progettare la sua intestazione.",
            |ui| {
                if primary_button(ui, &theme, "Apri la libreria").clicked() {
                    app.page = Page::Library;
                }
            },
        );
        return;
    };

    let cycle = app.project.title_template.delay_ms
        + if app.project.title_template.duration_ms == 0 {
            9_000
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
    let template = app.project.title_template.clone();

    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(
                RichText::new("Anteprima titolo")
                    .size(22.0)
                    .strong()
                    .color(theme.colors.text),
            );
            ui.label(
                RichText::new(format!("{} · {}", song.display_title(), template.name))
                    .font(caption())
                    .color(theme.colors.text_secondary),
            );
        });
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if Button::new("Riavvia")
                .variant(ButtonVariant::Plain)
                .icon(Icon::Refresh)
                .show(ui, &theme)
                .clicked()
            {
                app.preview_anchor = Instant::now();
            }
            if app.project.output.show_titles {
                status_pill(ui, &theme, "Titoli attivi", theme.colors.green, None);
            } else {
                status_pill(ui, &theme, "Titoli spenti", theme.colors.amber, None);
            }
        });
    });
    ui.add_space(8.0);

    let available = ui.available_size();
    let height = (available.y * 0.67).clamp(170.0, 470.0);
    let width = (height * 16.0 / 9.0).min(available.x);
    let top = ui.cursor().top();
    let canvas = Rect::from_min_size(
        Pos2::new(
            ui.available_rect_before_wrap().center().x - width / 2.0,
            top,
        ),
        Vec2::new(width, height),
    );
    let (_, response) =
        ui.allocate_exact_size(Vec2::new(available.x, height + 6.0), Sense::hover());
    ui.painter()
        .rect_filled(canvas, CornerRadius::same(10), theme.colors.blackout);
    output::paint_output(ui.painter(), canvas, &state);
    for corner in [
        canvas.left_top(),
        canvas.right_top(),
        canvas.left_bottom(),
        canvas.right_bottom(),
    ] {
        ui.painter().rect_filled(
            Rect::from_center_size(corner, Vec2::splat(7.0)),
            CornerRadius::same(2),
            theme.colors.text,
        );
    }
    let handle = Rect::from_center_size(
        canvas.right_bottom() - Vec2::new(26.0, 26.0),
        Vec2::splat(14.0),
    );
    let handle_response = ui.interact(
        handle,
        ui.scope_id().with("title-size-handle"),
        Sense::drag(),
    );
    ui.painter().circle_filled(
        handle.center(),
        5.0,
        if handle_response.hovered() || handle_response.dragged() {
            theme.colors.accent
        } else {
            theme.colors.text
        },
    );
    if handle_response.dragged() {
        let before = app.project.clone();
        let mut next = app.project.title_template.clone();
        next.title_size_pct =
            (next.title_size_pct + handle_response.drag_delta().y * 0.02).clamp(1.5, 18.0);
        next.validate();
        app.project.title_template = next;
        app.commit_edit(&before);
    }
    let _ = response;

    ui.add_space(8.0);
    title_strip(app, ui, &template, elapsed, cycle, &theme);
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        if Button::new(if app.output_open {
            "Uscita aperta"
        } else {
            "Apri uscita video"
        })
        .variant(ButtonVariant::Primary)
        .icon(Icon::Monitor)
        .show(ui, &theme)
        .clicked()
        {
            app.toggle_output(ui.ctx());
        }
        if !app.project.output.show_titles {
            if Button::new("Attiva titoli")
                .variant(ButtonVariant::Tinted)
                .show(ui, &theme)
                .clicked()
            {
                app.project.output.show_titles = true;
                app.mark_dirty();
            }
        } else if Button::new("Nascondi i titoli")
            .variant(ButtonVariant::Plain)
            .show(ui, &theme)
            .clicked()
        {
            app.project.output.show_titles = false;
            app.mark_dirty();
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(
                RichText::new(format!("{:.1}s", elapsed as f32 / 1000.0))
                    .font(nerd_ui_kit::timecode(11.0))
                    .color(theme.colors.text_secondary),
            );
        });
    });
}

fn title_strip(
    app: &mut NerdApp,
    ui: &mut Ui,
    template: &TitleTemplate,
    elapsed: u64,
    cycle: u64,
    theme: &Theme,
) {
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 54.0), Sense::hover());
    ui.painter()
        .rect_filled(rect, CornerRadius::same(7), theme.colors.panel);
    let inner = rect.shrink2(Vec2::new(12.0, 14.0));
    ui.painter()
        .rect_filled(inner, CornerRadius::same(4), theme.colors.well);
    let total = cycle.max(1) as f32;
    let delay = template.delay_ms as f32;
    let visible = if template.duration_ms == 0 {
        total - delay - 1_500.0
    } else {
        template.duration_ms as f32
    };
    let start = inner.left() + inner.width() * (delay / total);
    let block = inner.width() * (visible.max(300.0) / total);
    ui.painter().rect_filled(
        Rect::from_min_size(
            Pos2::new(start, inner.top()),
            Vec2::new(block, inner.height()),
        ),
        CornerRadius::same(3),
        theme.colors.title.gamma_multiply(0.42),
    );
    ui.painter().text(
        Pos2::new(start + 6.0, inner.center().y),
        Align2::LEFT_CENTER,
        "Titolo in onda",
        caption(),
        theme.colors.title,
    );
    let playhead = inner.left() + inner.width() * (elapsed as f32 / total).clamp(0.0, 1.0);
    ui.painter().line_segment(
        [
            Pos2::new(playhead, rect.top() + 4.0),
            Pos2::new(playhead, rect.bottom() - 4.0),
        ],
        egui::Stroke::new(1.5, theme.colors.accent),
    );
    ui.painter().text(
        Pos2::new(rect.left() + 2.0, rect.bottom() - 2.0),
        Align2::LEFT_BOTTOM,
        if template.duration_ms == 0 {
            "Visibile per tutto il brano".to_string()
        } else {
            format!(
                "{} · {}",
                strings::format_timecode(template.delay_ms),
                strings::format_timecode(template.duration_ms)
            )
        },
        caption(),
        theme.colors.text_tertiary,
    );
    let _ = app;
}

fn inspector(app: &mut NerdApp, ui: &mut Ui) {
    let theme = app.palette;
    let before = app.project.clone();
    let mut template = app.project.title_template.clone();
    let template_before = template.clone();
    let mut song = app.selected_song().cloned();
    let song_before = song.clone();

    ScrollArea::vertical()
        .id_salt("titles-inspector-scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                ui.add_space(16.0);
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new("Titolo")
                            .size(15.0)
                            .strong()
                            .color(theme.colors.text),
                    );
                    ui.label(
                        RichText::new("Stile e testi dell'uscita")
                            .font(caption())
                            .color(theme.colors.text_secondary),
                    );
                });
            });
            ui.add_space(8.0);
            nerd_ui_kit::divider(ui, &theme);

            section(ui, &theme, "title-style", "Stile", true, |ui| {
                ui.horizontal(|ui| {
                    ui.add_space(16.0);
                    ui.vertical(|ui| {
                        ui.set_width((ui.available_width() - 16.0).max(140.0));
                        labeled_field(ui, &theme, "Nome template", &mut template.name, "Template");
                        ui.add_space(6.0);
                        let mut pos = match template.position {
                            TitlePosition::Upper => 0,
                            TitlePosition::Center => 1,
                            TitlePosition::Lower => 2,
                        };
                        if segmented(
                            ui,
                            &theme,
                            "title-pos",
                            &["Alta", "Centro", "Bassa"],
                            &mut pos,
                            200.0,
                        )
                        .changed()
                        {
                            template.position = match pos {
                                0 => TitlePosition::Upper,
                                1 => TitlePosition::Center,
                                _ => TitlePosition::Lower,
                            };
                        }
                        ui.add_space(5.0);
                        let mut anim = match template.animation {
                            TitleAnimation::None => 0,
                            TitleAnimation::Fade => 1,
                            TitleAnimation::SlideUp => 2,
                            TitleAnimation::Scale => 3,
                        };
                        if segmented(
                            ui,
                            &theme,
                            "title-anim",
                            &["No", "Fade", "Salita", "Scala"],
                            &mut anim,
                            252.0,
                        )
                        .changed()
                        {
                            template.animation = match anim {
                                0 => TitleAnimation::None,
                                1 => TitleAnimation::Fade,
                                2 => TitleAnimation::SlideUp,
                                _ => TitleAnimation::Scale,
                            };
                        }
                        ui.add_space(6.0);
                        let mut title_size = template.title_size_pct;
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("Titolo")
                                    .font(caption())
                                    .color(theme.colors.text_secondary),
                            );
                            if slider(ui, &theme, &mut title_size, 1.5..=18.0, "%", 120.0).changed()
                            {
                                template.title_size_pct = title_size;
                            }
                        });
                        let mut body_size = template.body_size_pct;
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("Testo")
                                    .font(caption())
                                    .color(theme.colors.text_secondary),
                            );
                            if slider(ui, &theme, &mut body_size, 1.0..=10.0, "%", 120.0).changed()
                            {
                                template.body_size_pct = body_size;
                            }
                        });
                        ui.add_space(5.0);
                        toggle(ui, &theme, &mut template.panel, "Pannello dietro al testo");
                        if template.panel {
                            let mut opacity = template.panel_opacity;
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new("Opacità")
                                        .font(caption())
                                        .color(theme.colors.text_secondary),
                                );
                                if slider(ui, &theme, &mut opacity, 0.0..=1.0, "", 120.0).changed()
                                {
                                    template.panel_opacity = opacity;
                                }
                            });
                        }
                        ui.add_space(5.0);
                        let mut delay = template.delay_ms as f64 / 1000.0;
                        nerd_ui_kit::number_field(
                            ui,
                            &theme,
                            "Ritardo",
                            &mut delay,
                            0.0..=30.0,
                            " s",
                            0.1,
                        );
                        template.delay_ms = (delay * 1000.0) as u64;
                        let mut whole = template.duration_ms == 0;
                        if toggle(ui, &theme, &mut whole, "Visibile per tutto il brano").changed() {
                            template.duration_ms = if whole { 0 } else { 8_500 };
                        }
                        if !whole {
                            let mut duration = template.duration_ms as f64 / 1000.0;
                            nerd_ui_kit::number_field(
                                ui,
                                &theme,
                                "Durata",
                                &mut duration,
                                0.5..=120.0,
                                " s",
                                0.1,
                            );
                            template.duration_ms = (duration * 1000.0) as u64;
                        }
                    });
                });
            });

            if let Some(song_ref) = song.as_mut() {
                section(ui, &theme, "title-text", "Testi del brano", true, |ui| {
                    ui.horizontal(|ui| {
                        ui.add_space(16.0);
                        ui.vertical(|ui| {
                            ui.set_width((ui.available_width() - 16.0).max(140.0));
                            ui.label(
                                RichText::new(song_ref.display_title())
                                    .font(caption())
                                    .color(theme.colors.accent),
                            );
                            ui.add_space(3.0);
                            let mut title = song_ref.info.title.clone();
                            if nerd_ui_kit::text_field(
                                ui,
                                &theme,
                                &mut title,
                                "Titolo (vuoto = brano)",
                            )
                            .changed()
                            {
                                song_ref.info.title = title;
                            }
                            ui.add_space(4.0);
                            let mut artist = song_ref.info.artist.clone();
                            if nerd_ui_kit::text_field(
                                ui,
                                &theme,
                                &mut artist,
                                "Artista (vuoto = brano)",
                            )
                            .changed()
                            {
                                song_ref.info.artist = artist;
                            }
                            ui.add_space(4.0);
                            let mut authors = song_ref.info.authors.clone();
                            if nerd_ui_kit::text_field(ui, &theme, &mut authors, "Autori").changed()
                            {
                                song_ref.info.authors = authors;
                            }
                            ui.add_space(4.0);
                            let mut performers = song_ref.info.performers.clone();
                            if nerd_ui_kit::text_field(
                                ui,
                                &theme,
                                &mut performers,
                                "Interpreti / classe",
                            )
                            .changed()
                            {
                                song_ref.info.performers = performers;
                            }
                            ui.add_space(6.0);
                            let mut extra = song_ref.info.extra_lines.join("\n");
                            if nerd_ui_kit::text_field(ui, &theme, &mut extra, "Righe aggiuntive")
                                .changed()
                            {
                                song_ref.info.extra_lines = extra
                                    .lines()
                                    .map(|line| line.trim().to_string())
                                    .filter(|line| !line.is_empty())
                                    .collect();
                            }
                            ui.add_space(6.0);
                            let lines = song_ref
                                .info
                                .resolved_lines(&song_ref.title, &song_ref.artist);
                            for line in &lines {
                                let text = match line {
                                    TitleLine::Title(text) => text,
                                    TitleLine::Artist(text) => text,
                                    TitleLine::Meta(text) => text,
                                };
                                ui.label(
                                    RichText::new(format!("· {text}"))
                                        .font(caption())
                                        .color(theme.colors.text),
                                );
                            }
                        });
                    });
                });
            }

            ui.add_space(12.0);
            ui.horizontal(|ui| {
                ui.add_space(16.0);
                if Button::new("Ripristina template")
                    .variant(ButtonVariant::Plain)
                    .show(ui, &theme)
                    .clicked()
                {
                    app.project.title_template = TitleTemplate::default();
                    app.commit_edit(&before);
                    app.preview_anchor = Instant::now();
                }
                if Button::new("Vai alla scaletta")
                    .variant(ButtonVariant::Secondary)
                    .icon(Icon::Timeline)
                    .show(ui, &theme)
                    .clicked()
                {
                    app.page = Page::Timeline;
                }
            });
            ui.add_space(16.0);
        });

    template.validate();
    if serde_json::to_string(&template).unwrap_or_default()
        != serde_json::to_string(&template_before).unwrap_or_default()
    {
        app.project.title_template = template;
        app.commit_edit(&before);
        app.preview_anchor = Instant::now();
    }
    if let (Some(song), Some(before_song)) = (song, song_before) {
        if serde_json::to_string(&song).unwrap_or_default()
            != serde_json::to_string(&before_song).unwrap_or_default()
        {
            if let Some(index) = app.project.songs.iter().position(|s| s.id == song.id) {
                app.project.songs[index] = song;
                app.commit_edit(&before);
                app.preview_anchor = Instant::now();
            }
        }
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
