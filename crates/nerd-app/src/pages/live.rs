//! The Live console: large transport, countdown, meters and output monitor.

use eframe::egui::{
    self, Align, Align2, Color32, CornerRadius, FontId, Layout, Pos2, Rect, RichText, ScrollArea,
    Sense, Stroke, Ui, Vec2,
};
use nerd_core::model::PlaybackMode;
use nerd_core::Phase;

use crate::app::NerdApp;
use crate::output;
use crate::state::Page;
use crate::widgets;

pub fn show(app: &mut NerdApp, ui: &mut Ui) {
    egui::Frame::new()
        .fill(app.palette.background)
        .inner_margin(egui::Margin::symmetric(18, 14))
        .show(ui, |ui| {
            ScrollArea::vertical()
                .id_salt("live-scroll")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    topbar(app, ui);
                    if app.project.songs.is_empty() {
                        empty_show(app, ui);
                        return;
                    }
                    ui.add_space(12.0);
                    let width = ui.available_width();
                    if width > 980.0 {
                        ui.horizontal_top(|ui| {
                            ui.allocate_ui_with_layout(
                                Vec2::new(width * 0.625, 0.0),
                                Layout::top_down(Align::Min),
                                |ui| {
                                    now_card(app, ui);
                                    ui.add_space(12.0);
                                    controls_card(app, ui);
                                },
                            );
                            ui.add_space(12.0);
                            ui.allocate_ui_with_layout(
                                Vec2::new(width * 0.35, 0.0),
                                Layout::top_down(Align::Min),
                                |ui| {
                                    next_card(app, ui);
                                    ui.add_space(12.0);
                                    checklist_card(app, ui);
                                },
                            );
                        });
                    } else {
                        now_card(app, ui);
                        ui.add_space(12.0);
                        controls_card(app, ui);
                        ui.add_space(12.0);
                        next_card(app, ui);
                        ui.add_space(12.0);
                        checklist_card(app, ui);
                    }
                    ui.add_space(12.0);
                    lower_cards(app, ui);
                    ui.add_space(10.0);
                    shortcuts_strip(ui, app);
                });
        });
}

fn topbar(app: &mut NerdApp, ui: &mut Ui) {
    let palette = app.palette;
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(crate::strings::PAGE_LIVE)
                .size(22.0)
                .strong()
                .color(palette.text),
        );
        let phase = app.show.phase();
        let color = match phase {
            Phase::Playing => palette.accent,
            Phase::Paused => palette.warning,
            Phase::Waiting => palette.title,
            Phase::Panic | Phase::Stopping => palette.danger,
            Phase::Finished => palette.success,
            Phase::Idle => palette.text_muted,
        };
        widgets::pill(ui, output::phase_label(phase), color);
        if app.show.background_running() {
            widgets::pill(
                ui,
                if app.show.background_paused() {
                    "Sottofondo in pausa"
                } else {
                    "Sottofondo on air"
                },
                palette.title,
            );
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if ui
                .add(
                    egui::Button::new(
                        RichText::new(if app.output_open {
                            "Chiudi uscita"
                        } else {
                            "Apri uscita video"
                        })
                        .size(11.0)
                        .color(if app.output_open {
                            palette.accent
                        } else {
                            palette.text
                        }),
                    )
                    .fill(palette.raised)
                    .stroke(Stroke::new(1.0, palette.border))
                    .corner_radius(CornerRadius::same(7)),
                )
                .clicked()
            {
                app.toggle_output(ui.ctx());
            }
            if app.project.output.show_titles {
                widgets::pill(ui, "Titoli on", palette.title);
            }
            ui.label(
                RichText::new(format!("{} · {}", app.project.name, app.project.date))
                    .size(10.5)
                    .color(palette.text_muted),
            );
        });
    });
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("Modalità")
                .size(10.5)
                .color(palette.text_muted),
        );
        for mode in [PlaybackMode::Manual, PlaybackMode::Automatic] {
            let selected = app.show.mode() == mode;
            let button =
                egui::Button::new(RichText::new(widgets::mode_label(mode)).size(11.0).color(
                    if selected {
                        Color32::from_rgb(9, 23, 21)
                    } else {
                        palette.text
                    },
                ))
                .fill(if selected {
                    palette.accent
                } else {
                    palette.raised
                })
                .stroke(Stroke::new(1.0, palette.border))
                .corner_radius(CornerRadius::same(7))
                .min_size(Vec2::new(96.0, 26.0));
            if ui.add(button).clicked() {
                app.set_show_mode(mode);
            }
        }
        if app.project.show.mode == PlaybackMode::Automatic {
            let suspended = app.show.auto_suspended();
            let label = if suspended {
                "Riprendi automatico"
            } else {
                "Sospendi automatico"
            };
            if ui
                .add(
                    egui::Button::new(RichText::new(label).size(10.5).color(if suspended {
                        palette.warning
                    } else {
                        palette.text
                    }))
                    .fill(palette.raised)
                    .stroke(Stroke::new(
                        1.0,
                        if suspended {
                            palette.warning.gamma_multiply(0.5)
                        } else {
                            palette.border
                        },
                    ))
                    .corner_radius(CornerRadius::same(7)),
                )
                .clicked()
            {
                app.toggle_auto_suspend();
            }
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if app.show.is_counting_down() {
                widgets::pill(
                    ui,
                    format!(
                        "Countdown {:.1}s",
                        app.show.gap_remaining_ms().unwrap_or(0) as f32 / 1000.0
                    ),
                    palette.warning,
                );
            }
        });
    });
}

fn empty_show(app: &mut NerdApp, ui: &mut Ui) {
    let palette = app.palette;
    ui.add_space(60.0);
    ui.vertical_centered(|ui| {
        ui.label(
            RichText::new("Il palco è pronto")
                .size(30.0)
                .strong()
                .color(palette.text),
        );
        ui.add_space(6.0);
        ui.label(
            RichText::new("Aggiungi i brani alla scaletta per iniziare la serata.")
                .size(13.0)
                .color(palette.text_muted),
        );
        ui.add_space(16.0);
        if crate::widgets::accent_button(ui, palette, "Vai alla libreria").clicked() {
            app.page = Page::Library;
        }
        if ui.button("Importa file…").clicked() {
            app.import_dialog();
        }
    });
}

fn now_card(app: &mut NerdApp, ui: &mut Ui) {
    let palette = app.palette;
    let phase = app.show.phase();
    let live = app.live_song().cloned();
    let next = app.next_song().cloned();
    let elapsed = app.show.elapsed_ms();
    let duration = app.show.live_duration_ms();
    let progress = app.show.progress_fraction();
    let remaining = app.show.remaining_ms();
    let gap = app.show.gap_remaining_ms();

    let border = if live.is_some() {
        palette.accent.gamma_multiply(0.55)
    } else {
        palette.border
    };
    egui::Frame::new()
        .fill(palette.panel)
        .stroke(Stroke::new(1.0, border))
        .corner_radius(CornerRadius::same(14))
        .inner_margin(egui::Margin::same(20))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                widgets::section_label(ui, palette, "Ora in onda");
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if app.show.is_paused() {
                        widgets::pill(ui, "IN PAUSA", palette.warning);
                    } else if app.show.background_running() {
                        widgets::pill(ui, "SOTTOFONDO ATTIVO", palette.title);
                    }
                });
            });
            ui.add_space(4.0);

            if let Some(song) = &live {
                ui.label(
                    RichText::new(song.display_title())
                        .size(34.0)
                        .strong()
                        .color(palette.text),
                );
                ui.label(
                    RichText::new(if song.artist.trim().is_empty() {
                        "Artista non indicato"
                    } else {
                        &song.artist
                    })
                    .size(15.0)
                    .color(palette.text_muted),
                );
                let credits = [
                    (!song.info.authors.is_empty()).then_some(song.info.authors.as_str()),
                    (!song.info.performers.is_empty()).then_some(song.info.performers.as_str()),
                ]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
                .join(" · ");
                if !credits.is_empty() {
                    ui.label(RichText::new(credits).size(11.0).color(palette.text_muted));
                }
                ui.add_space(14.0);
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new("Rimanente")
                                .size(10.0)
                                .color(palette.text_muted),
                        );
                        ui.label(
                            RichText::new(format_clock(remaining))
                                .size(52.0)
                                .strong()
                                .color(if remaining < 15_000 {
                                    palette.warning
                                } else {
                                    palette.text
                                })
                                .monospace(),
                        );
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.vertical(|ui| {
                            ui.label(
                                RichText::new(format!(
                                    "{} / {}",
                                    format_clock(elapsed),
                                    format_clock(duration)
                                ))
                                .size(11.0)
                                .color(palette.text_muted)
                                .monospace(),
                            );
                            if song.has_video() {
                                widgets::pill(ui, "VIDEO", palette.video);
                            }
                        });
                    });
                });
                ui.add(
                    egui::ProgressBar::new(progress)
                        .desired_height(10.0)
                        .fill(if app.show.is_paused() {
                            palette.warning
                        } else {
                            palette.accent
                        })
                        .show_percentage(),
                );
            } else {
                let title = next
                    .as_ref()
                    .map(|song| song.display_title())
                    .unwrap_or_else(|| "Nessun brano successivo".to_string());
                ui.label(
                    RichText::new(match phase {
                        Phase::Waiting => "In attesa del GO",
                        Phase::Finished => "Scaletta terminata",
                        _ => "Pronto al via",
                    })
                    .size(12.0)
                    .color(palette.text_muted),
                );
                ui.label(RichText::new(title).size(32.0).strong().color(palette.text));
                if phase == Phase::Waiting {
                    if app.show.is_counting_down() {
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(format!(
                                    "Prossimo brano tra {:.1}",
                                    gap.unwrap_or(0) as f32 / 1000.0
                                ))
                                .size(24.0)
                                .strong()
                                .color(palette.warning),
                            );
                            ui.label(
                                RichText::new("secondi")
                                    .size(11.0)
                                    .color(palette.text_muted),
                            );
                        });
                    } else {
                        ui.label(
                            RichText::new("Premi GO o Spazio per avviare.")
                                .size(11.0)
                                .color(palette.text_muted),
                        );
                    }
                } else if phase == Phase::Finished {
                    ui.label(
                        RichText::new("Il sottofondo resta in onda fino a Stop.")
                            .size(11.0)
                            .color(palette.text_muted),
                    );
                }
            }
        });
}

fn controls_card(app: &mut NerdApp, ui: &mut Ui) {
    let palette = app.palette;
    let phase = app.show.phase();
    let live = app.show.live_index().is_some();
    let can_pause =
        matches!(phase, Phase::Playing | Phase::Paused) || app.show.background_running();
    let can_stop = phase != Phase::Idle;
    let can_skip = live || phase == Phase::Waiting;
    let can_back = live || phase == Phase::Waiting;
    let can_panic = phase != Phase::Idle;

    egui::Frame::new()
        .fill(palette.panel)
        .stroke(Stroke::new(1.0, palette.border))
        .corner_radius(CornerRadius::same(14))
        .inner_margin(egui::Margin::same(16))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                if go_button(ui, palette, live) {
                    app.request_go();
                }
                ui.add_space(6.0);
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        let pause_label = if app.show.is_paused() || app.show.background_paused() {
                            crate::strings::RESUME
                        } else {
                            crate::strings::PAUSE
                        };
                        if ui
                            .add_enabled(
                                can_pause,
                                transport_button(palette, pause_label, palette.warning),
                            )
                            .clicked()
                        {
                            app.toggle_pause();
                        }
                        if ui
                            .add_enabled(
                                can_stop,
                                transport_button(palette, crate::strings::STOP, palette.text),
                            )
                            .clicked()
                        {
                            app.stop_show();
                        }
                        if ui
                            .add_enabled(
                                can_skip,
                                transport_button(palette, crate::strings::SKIP, palette.text),
                            )
                            .clicked()
                        {
                            app.skip_song();
                        }
                        if ui
                            .add_enabled(
                                can_back,
                                transport_button(palette, crate::strings::BACK, palette.text),
                            )
                            .clicked()
                        {
                            app.previous_song();
                        }
                    });
                    ui.add_space(6.0);
                    if ui
                        .add_enabled(
                            can_panic,
                            egui::Button::new(
                                RichText::new(crate::strings::PANIC)
                                    .size(12.0)
                                    .strong()
                                    .color(if can_panic {
                                        Color32::from_rgb(30, 8, 8)
                                    } else {
                                        palette.text_muted
                                    }),
                            )
                            .fill(if can_panic {
                                palette.danger
                            } else {
                                palette.raised
                            })
                            .stroke(Stroke::NONE)
                            .corner_radius(CornerRadius::same(8))
                            .min_size(Vec2::new(0.0, 28.0)),
                        )
                        .on_hover_text("Ferma tutto con un fade rapido (⇧⌘P)")
                        .clicked()
                    {
                        app.panic();
                    }
                    ui.label(
                        RichText::new("Spazio: GO · P: pausa · S: stop · N: salta · B: indietro")
                            .size(9.5)
                            .color(palette.text_muted),
                    );
                });
            });
        });
}

fn go_button(ui: &mut Ui, palette: crate::theme::Palette, live: bool) -> bool {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(210.0, 92.0), Sense::click());
    let fill = if response.hovered() {
        palette.accent.gamma_multiply(1.12)
    } else {
        palette.accent
    };
    ui.painter().rect(
        rect,
        CornerRadius::same(14),
        fill,
        Stroke::new(2.0, palette.accent.gamma_multiply(1.4)),
        egui::StrokeKind::Inside,
    );
    ui.painter().text(
        Pos2::new(rect.center().x, rect.top() + 18.0),
        Align2::CENTER_TOP,
        crate::strings::GO,
        FontId::proportional(38.0),
        Color32::from_rgb(9, 23, 21),
    );
    ui.painter().text(
        Pos2::new(rect.center().x, rect.bottom() - 19.0),
        Align2::CENTER_BOTTOM,
        if live {
            "conferma salto"
        } else {
            "avvia il prossimo"
        },
        FontId::proportional(10.0),
        Color32::from_rgba_unmultiplied(9, 23, 21, 190),
    );
    response.clicked()
}

fn transport_button(
    palette: crate::theme::Palette,
    text: &str,
    color: Color32,
) -> egui::Button<'static> {
    egui::Button::new(RichText::new(text).size(12.0).color(color))
        .fill(palette.raised)
        .stroke(Stroke::new(1.0, palette.border))
        .corner_radius(CornerRadius::same(8))
        .min_size(Vec2::new(68.0, 36.0))
}

fn next_card(app: &mut NerdApp, ui: &mut Ui) {
    let palette = app.palette;
    let next = app.next_song().cloned();
    let phase = app.show.phase();
    egui::Frame::new()
        .fill(palette.panel)
        .stroke(Stroke::new(1.0, palette.border))
        .corner_radius(CornerRadius::same(14))
        .inner_margin(egui::Margin::same(18))
        .show(ui, |ui| {
            widgets::section_label(ui, palette, crate::strings::NEXT);
            ui.add_space(6.0);
            if let Some(song) = next {
                ui.label(
                    RichText::new(song.display_title())
                        .size(22.0)
                        .strong()
                        .color(palette.text),
                );
                ui.label(
                    RichText::new(if song.artist.is_empty() {
                        "Artista non indicato"
                    } else {
                        &song.artist
                    })
                    .size(11.5)
                    .color(palette.text_muted),
                );
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    widgets::pill(
                        ui,
                        crate::strings::format_timecode(song.effective_duration_ms()),
                        palette.text_muted,
                    );
                    if song.has_video() {
                        widgets::pill(ui, "VIDEO", palette.video);
                    }
                    widgets::pill(ui, widgets::status_label(song.status), palette.title);
                });
                if !song.notes.trim().is_empty() {
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new(&song.notes)
                            .size(10.5)
                            .color(palette.text_muted),
                    );
                }
            } else if phase == Phase::Finished {
                ui.label(
                    RichText::new("Nessun altro brano")
                        .size(18.0)
                        .color(palette.text_muted),
                );
                ui.label(
                    RichText::new("Il sottofondo accompagna l'uscita del pubblico.")
                        .size(10.5)
                        .color(palette.text_muted),
                );
            } else {
                ui.label(
                    RichText::new("Fine della scaletta")
                        .size(18.0)
                        .color(palette.text_muted),
                );
            }
            ui.add_space(12.0);
            widgets::divider(ui, palette);
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("Sottofondo")
                        .size(10.5)
                        .color(palette.text_muted),
                );
                widgets::pill(
                    ui,
                    if app.show.background_paused() {
                        "In pausa"
                    } else if app.show.background_running() {
                        "In onda"
                    } else {
                        "Fermo"
                    },
                    if app.show.background_running() {
                        palette.title
                    } else {
                        palette.text_muted
                    },
                );
            });
        });
}

fn checklist_card(app: &mut NerdApp, ui: &mut Ui) {
    let palette = app.palette;
    let completed = app.checklist.complete();
    egui::Frame::new()
        .fill(palette.panel)
        .stroke(Stroke::new(
            1.0,
            if completed {
                palette.success.gamma_multiply(0.5)
            } else {
                palette.border
            },
        ))
        .corner_radius(CornerRadius::same(14))
        .inner_margin(egui::Margin::same(18))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                widgets::section_label(ui, palette, "Checklist pre-show");
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if completed {
                        widgets::pill(ui, crate::strings::READY.to_uppercase(), palette.success);
                    } else {
                        let done = [
                            app.checklist.audio_output,
                            app.checklist.projector,
                            app.checklist.volumes,
                        ]
                        .iter()
                        .filter(|value| **value)
                        .count();
                        widgets::pill(ui, format!("{done}/3"), palette.warning);
                    }
                });
            });
            ui.add_space(4.0);
            ui.checkbox(
                &mut app.checklist.audio_output,
                "Uscita audio collegata e testata",
            );
            ui.checkbox(
                &mut app.checklist.projector,
                "Proiettore acceso sulla scena",
            );
            ui.checkbox(&mut app.checklist.volumes, "Volumi e sottofondo verificati");
            if ui
                .add(
                    egui::Button::new(
                        RichText::new("Azzera checklist")
                            .size(9.5)
                            .color(palette.text_muted),
                    )
                    .frame(false),
                )
                .clicked()
            {
                app.checklist = crate::state::Checklist::default();
            }
        });
}

fn lower_cards(app: &mut NerdApp, ui: &mut Ui) {
    let palette = app.palette;
    let width = ui.available_width();
    if width > 1_000.0 {
        ui.horizontal_top(|ui| {
            ui.allocate_ui_with_layout(
                Vec2::new(width * 0.32, 0.0),
                Layout::top_down(Align::Min),
                |ui| background_card(app, ui),
            );
            ui.add_space(12.0);
            ui.allocate_ui_with_layout(
                Vec2::new(width * 0.30, 0.0),
                Layout::top_down(Align::Min),
                |ui| meters_card(app, ui),
            );
            ui.add_space(12.0);
            ui.allocate_ui_with_layout(
                Vec2::new(width * 0.33, 0.0),
                Layout::top_down(Align::Min),
                |ui| output_card(app, ui),
            );
        });
    } else {
        background_card(app, ui);
        ui.add_space(12.0);
        meters_card(app, ui);
        ui.add_space(12.0);
        output_card(app, ui);
    }
    let _ = palette;
}

fn background_card(app: &mut NerdApp, ui: &mut Ui) {
    let palette = app.palette;
    let running = app.show.background_running();
    let paused = app.show.background_paused();
    let phase = app.show.phase();
    let can_start = !matches!(
        phase,
        Phase::Playing | Phase::Paused | Phase::Stopping | Phase::Panic
    );
    let pos = app.levels.background_pos_ms;
    let duration = app.levels.background_duration_ms;
    let progress = if duration > 0 {
        (pos as f32 / duration as f32).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let mut volume = app.project.background.volume;

    egui::Frame::new()
        .fill(palette.panel)
        .stroke(Stroke::new(1.0, palette.border))
        .corner_radius(CornerRadius::same(14))
        .inner_margin(egui::Margin::same(16))
        .show(ui, |ui| {
            widgets::section_label(ui, palette, crate::strings::BACKGROUND);
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(if paused {
                        "In pausa"
                    } else if running {
                        "In onda"
                    } else {
                        "Fermo"
                    })
                    .size(15.0)
                    .strong()
                    .color(if running {
                        palette.title
                    } else {
                        palette.text_muted
                    }),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(
                        RichText::new(format!("{} brani", app.project.background.tracks.len()))
                            .size(10.0)
                            .color(palette.text_muted),
                    );
                });
            });
            if let Some(error) = &app.background_error {
                ui.label(RichText::new(error).size(10.0).color(palette.danger));
            }
            if duration > 0 {
                let (rect, _) =
                    ui.allocate_exact_size(Vec2::new(ui.available_width(), 5.0), Sense::hover());
                ui.painter()
                    .rect_filled(rect, CornerRadius::same(3), palette.background);
                ui.painter().rect_filled(
                    Rect::from_min_size(
                        rect.min,
                        Vec2::new(rect.width() * progress, rect.height()),
                    ),
                    CornerRadius::same(3),
                    palette.title,
                );
                ui.label(
                    RichText::new(format!(
                        "{} / {}",
                        format_clock(pos),
                        format_clock(duration)
                    ))
                    .size(9.5)
                    .color(palette.text_muted)
                    .monospace(),
                );
            } else if app.project.background.tracks.is_empty() {
                ui.label(
                    RichText::new("Nessuna playlist configurata.")
                        .size(10.5)
                        .color(palette.text_muted),
                );
                if ui.button("Configura in Impostazioni").clicked() {
                    app.page = Page::Settings;
                }
            }
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(
                        can_start && !running,
                        egui::Button::new("Avvia")
                            .fill(palette.title.gamma_multiply(0.2))
                            .stroke(Stroke::new(1.0, palette.title.gamma_multiply(0.5)))
                            .corner_radius(CornerRadius::same(7)),
                    )
                    .clicked()
                {
                    app.start_background_manual();
                }
                if ui
                    .add_enabled(
                        running,
                        egui::Button::new("Ferma")
                            .fill(palette.raised)
                            .stroke(Stroke::new(1.0, palette.border))
                            .corner_radius(CornerRadius::same(7)),
                    )
                    .clicked()
                {
                    app.stop_background_manual();
                }
                if running
                    && ui
                        .button(if paused { "Riprendi" } else { "Pausa" })
                        .clicked()
                {
                    app.toggle_pause();
                }
            });
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("Volume").size(10.0).color(palette.text_muted));
                if ui
                    .add(
                        egui::Slider::new(&mut volume, 0.0..=1.5)
                            .show_value(false)
                            .clamping(egui::SliderClamping::Always),
                    )
                    .changed()
                {
                    app.project.background.volume = volume;
                    let _ = app.engine.set_background_volume(volume);
                    app.mark_dirty();
                }
                ui.label(
                    RichText::new(crate::strings::format_volume(volume))
                        .size(9.5)
                        .color(palette.text),
                );
            });
        });
}

fn meters_card(app: &mut NerdApp, ui: &mut Ui) {
    let palette = app.palette;
    let mut master = app.prefs.master_volume;
    let mut song_volume = app.live_song().map(|song| song.volume).unwrap_or_default();
    let has_song = app.live_song().is_some();

    egui::Frame::new()
        .fill(palette.panel)
        .stroke(Stroke::new(1.0, palette.border))
        .corner_radius(CornerRadius::same(14))
        .inner_margin(egui::Margin::same(16))
        .show(ui, |ui| {
            widgets::section_label(ui, palette, "Mix");
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("Brano")
                        .size(10.5)
                        .color(palette.accent)
                        .strong(),
                );
                widgets::vu_meter(
                    ui,
                    palette,
                    app.song_meter,
                    &format!("{:.0}%", app.song_meter * 100.0),
                    palette.accent,
                );
            });
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("Sottofondo")
                        .size(10.5)
                        .color(palette.title)
                        .strong(),
                );
                widgets::vu_meter(
                    ui,
                    palette,
                    app.bg_meter,
                    &format!("{:.0}%", app.bg_meter * 100.0),
                    palette.title,
                );
            });
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("Master").size(10.0).color(palette.text_muted));
                if ui
                    .add(egui::Slider::new(&mut master, 0.0..=1.0).show_value(false))
                    .changed()
                {
                    app.prefs.master_volume = master;
                    app.prefs_dirty = true;
                    let _ = app.engine.set_master_volume(master);
                }
                ui.label(
                    RichText::new(crate::strings::format_volume(master))
                        .size(9.5)
                        .color(palette.text),
                );
            });
            if has_song {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Brano").size(10.0).color(palette.text_muted));
                    if ui
                        .add(
                            egui::Slider::new(&mut song_volume, 0.0..=1.5)
                                .show_value(false)
                                .clamping(egui::SliderClamping::Always),
                        )
                        .changed()
                    {
                        if let Some(index) = app.show.live_index() {
                            if let Some(song) = app.project.songs.get_mut(index) {
                                song.volume = song_volume;
                            }
                            let _ = app.engine.set_song_volume(song_volume);
                            app.mark_dirty();
                        }
                    }
                    ui.label(
                        RichText::new(crate::strings::format_volume(song_volume))
                            .size(9.5)
                            .color(palette.text),
                    );
                });
            }
            ui.add_space(4.0);
            ui.label(
                RichText::new(format!(
                    "Uscita: {}",
                    app.engine
                        .status()
                        .device_name
                        .unwrap_or_else(|| "predefinita".to_string())
                ))
                .size(9.0)
                .color(palette.text_muted),
            );
        });
}

fn output_card(app: &mut NerdApp, ui: &mut Ui) {
    let palette = app.palette;
    let state = app.projector_state(ui.ctx());
    egui::Frame::new()
        .fill(palette.panel)
        .stroke(Stroke::new(1.0, palette.border))
        .corner_radius(CornerRadius::same(14))
        .inner_margin(egui::Margin::same(16))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                widgets::section_label(ui, palette, crate::strings::OUTPUT);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    widgets::pill(
                        ui,
                        if app.output_open { "APERTA" } else { "CHIUSA" },
                        if app.output_open {
                            palette.accent
                        } else {
                            palette.text_muted
                        },
                    );
                });
            });
            let width = ui.available_width().min(320.0);
            let preview_h = width * 9.0 / 16.0;
            let (rect, _) =
                ui.allocate_exact_size(Vec2::new(ui.available_width(), preview_h), Sense::hover());
            ui.painter().rect(
                rect,
                CornerRadius::same(7),
                Color32::BLACK,
                Stroke::new(1.0, palette.border),
                egui::StrokeKind::Inside,
            );
            let painter = ui.painter().with_clip_rect(rect);
            output::paint_output(&painter, rect, &state);
            ui.label(
                RichText::new(format!(
                    "{} · {}",
                    app.display_name(),
                    if app.project.output.blackout {
                        crate::strings::BLACKOUT
                    } else {
                        crate::strings::ON_AIR
                    }
                ))
                .size(9.5)
                .color(palette.text_muted),
            );
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if widgets::ghost_button(
                    ui,
                    palette,
                    if app.output_open {
                        "Chiudi"
                    } else {
                        "Apri uscita"
                    },
                )
                .clicked()
                {
                    app.toggle_output(ui.ctx());
                }
                if ui
                    .add(
                        egui::Button::new(
                            RichText::new(if app.project.output.blackout {
                                "Fine blackout"
                            } else {
                                "Blackout"
                            })
                            .size(10.0)
                            .color(if app.project.output.blackout {
                                Color32::WHITE
                            } else {
                                palette.danger
                            }),
                        )
                        .fill(if app.project.output.blackout {
                            palette.danger
                        } else {
                            palette.raised
                        })
                        .stroke(Stroke::new(1.0, palette.border))
                        .corner_radius(CornerRadius::same(7)),
                    )
                    .clicked()
                {
                    app.toggle_blackout();
                }
            });
        });
}

fn shortcuts_strip(ui: &mut Ui, app: &mut NerdApp) {
    let palette = app.palette;
    ui.horizontal_wrapped(|ui| {
        widgets::shortcut_chip(ui, palette, "Spazio", "GO");
        widgets::shortcut_chip(ui, palette, "P", "Pausa");
        widgets::shortcut_chip(ui, palette, "S", "Stop");
        widgets::shortcut_chip(ui, palette, "N", "Salta");
        widgets::shortcut_chip(ui, palette, "B", "Indietro");
        widgets::shortcut_chip(ui, palette, "⇧⌘P", "Panic");
        if ui.button("Tutte le scorciatoie").clicked() {
            app.show_shortcuts = true;
        }
    });
}

fn format_clock(ms: u64) -> String {
    let total = ms / 1000;
    format!("{:02}:{:02}", total / 60, total % 60)
}
