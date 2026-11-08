//! Live console: stage-sized type, GO, output preview and meters.

use eframe::egui::{
    self, Align, Align2, CornerRadius, Layout, Pos2, Rect, RichText, ScrollArea, Sense, Ui, Vec2,
};
use nerd_core::Phase;

use crate::app::NerdApp;
use crate::output;
use crate::state::Page;
use crate::strings;
use nerd_ui_kit::{
    body, caption, icon_button, keycap, primary_button, section_label, segmented, slider,
    status_pill, toggle, Button, ButtonVariant, Icon, Theme,
};

pub fn show(app: &mut NerdApp, ui: &mut Ui) {
    egui::Frame::new()
        .fill(app.palette.colors.window)
        .inner_margin(egui::Margin::same(18))
        .show(ui, |ui| {
            ScrollArea::vertical()
                .id_salt("live-scroll")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    header(app, ui);
                    if app.project.songs.is_empty() {
                        empty(app, ui);
                        return;
                    }
                    ui.add_space(12.0);
                    let width = ui.available_width();
                    if width >= 1020.0 {
                        ui.horizontal_top(|ui| {
                            ui.allocate_ui_with_layout(
                                Vec2::new(width * 0.62 - 6.0, 0.0),
                                Layout::top_down(Align::Min),
                                |ui| {
                                    now_card(app, ui);
                                    ui.add_space(12.0);
                                    controls(app, ui);
                                    ui.add_space(12.0);
                                    meters(app, ui);
                                },
                            );
                            ui.add_space(12.0);
                            ui.allocate_ui_with_layout(
                                Vec2::new(width * 0.38 - 6.0, 0.0),
                                Layout::top_down(Align::Min),
                                |ui| {
                                    next_card(app, ui);
                                    ui.add_space(12.0);
                                    output_card(app, ui);
                                    ui.add_space(12.0);
                                    background_card(app, ui);
                                },
                            );
                        });
                    } else {
                        now_card(app, ui);
                        ui.add_space(12.0);
                        controls(app, ui);
                        ui.add_space(12.0);
                        next_card(app, ui);
                        ui.add_space(12.0);
                        output_card(app, ui);
                        ui.add_space(12.0);
                        background_card(app, ui);
                    }
                    ui.add_space(12.0);
                    checklist(app, ui);
                    ui.add_space(12.0);
                    queue(app, ui);
                    ui.add_space(10.0);
                    shortcuts(ui, &app.palette);
                });
        });
}

fn header(app: &mut NerdApp, ui: &mut Ui) {
    let theme = app.palette;
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(
                RichText::new("Live")
                    .size(22.0)
                    .strong()
                    .color(theme.colors.text),
            );
            ui.label(
                RichText::new(format!("{} · {}", app.project.name, app.project.venue))
                    .font(caption())
                    .color(theme.colors.text_secondary),
            );
        });
        ui.add_space(14.0);
        if app.show.background_running() {
            status_pill(
                ui,
                &theme,
                if app.show.background_paused() {
                    "Sottofondo in pausa"
                } else {
                    "Sottofondo"
                },
                theme.colors.mint,
                Some(Icon::Waveform),
            );
        }
        if app.project.output.blackout {
            status_pill(
                ui,
                &theme,
                "Blackout",
                theme.colors.red,
                Some(Icon::Blackout),
            );
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if Button::new(if app.output_open {
                "Chiudi uscita"
            } else {
                "Apri uscita"
            })
            .variant(ButtonVariant::Secondary)
            .icon(Icon::Monitor)
            .show(ui, &theme)
            .clicked()
            {
                app.toggle_output(ui.ctx());
            }
            let mut mode = match app.show.mode() {
                nerd_core::model::PlaybackMode::Manual => 0,
                nerd_core::model::PlaybackMode::Automatic => 1,
            };
            if segmented(
                ui,
                &theme,
                "live-mode",
                &["Manuale", "Automatico"],
                &mut mode,
                190.0,
            )
            .changed()
            {
                app.set_show_mode(if mode == 0 {
                    nerd_core::model::PlaybackMode::Manual
                } else {
                    nerd_core::model::PlaybackMode::Automatic
                });
            }
        });
    });
}

fn empty(app: &mut NerdApp, ui: &mut Ui) {
    let theme = app.palette;
    nerd_ui_kit::empty_state(
        ui,
        &theme,
        Icon::Live,
        "Il palco è pronto",
        "Aggiungi brani alla scaletta per iniziare la serata.",
        |ui| {
            if primary_button(ui, &theme, "Apri la libreria").clicked() {
                app.page = Page::Library;
            }
        },
    );
}

fn now_card(app: &mut NerdApp, ui: &mut Ui) {
    let theme = app.palette;
    let phase = app.show.phase();
    let live = app.live_song().cloned();
    let next = app.next_song().cloned();
    let elapsed = app.show.elapsed_ms();
    let duration = app.show.live_duration_ms();
    let remaining = app.show.remaining_ms();
    let progress = app.show.progress_fraction();
    let gap = app.show.gap_remaining_ms();
    let accent = match phase {
        Phase::Playing => theme.colors.green,
        Phase::Paused => theme.colors.amber,
        Phase::Waiting => theme.colors.amber,
        Phase::Panic | Phase::Stopping => theme.colors.red,
        _ => theme.colors.accent,
    };
    egui::Frame::new()
        .fill(theme.colors.panel)
        .stroke(egui::Stroke::new(
            if live.is_some() { 1.5 } else { 1.0 },
            if live.is_some() {
                accent.gamma_multiply(0.55)
            } else {
                theme.colors.separator
            },
        ))
        .corner_radius(CornerRadius::same(12))
        .inner_margin(egui::Margin::same(20))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                section_label(ui, &theme, "In onda");
                ui.add_space(6.0);
                status_pill(
                    ui,
                    &theme,
                    output::phase_label(phase),
                    accent,
                    if phase == Phase::Paused {
                        Some(Icon::Pause)
                    } else {
                        None
                    },
                );
                if app.show.auto_suspended() {
                    status_pill(ui, &theme, "Auto sospeso", theme.colors.amber, None);
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(
                        RichText::new(
                            if app.show.mode() == nerd_core::model::PlaybackMode::Automatic {
                                "Automatico"
                            } else {
                                "Manuale"
                            },
                        )
                        .font(caption())
                        .color(theme.colors.text_secondary),
                    );
                });
            });
            ui.add_space(8.0);
            if let Some(song) = &live {
                ui.label(
                    RichText::new(song.display_title())
                        .font(nerd_ui_kit::live_title())
                        .strong()
                        .color(theme.colors.text),
                );
                ui.label(
                    RichText::new(if song.artist.is_empty() {
                        "Artista non indicato"
                    } else {
                        &song.artist
                    })
                    .size(16.0)
                    .color(theme.colors.text_secondary),
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
                    ui.label(
                        RichText::new(credits)
                            .font(caption())
                            .color(theme.colors.text_secondary),
                    );
                }
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new("Rimanente")
                                .font(caption())
                                .color(theme.colors.text_secondary),
                        );
                        ui.label(
                            RichText::new(nerd_ui_kit::format_clock(remaining))
                                .font(nerd_ui_kit::live_countdown())
                                .color(if remaining < 15_000 {
                                    theme.colors.amber
                                } else {
                                    theme.colors.text
                                }),
                        );
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.vertical(|ui| {
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if song.has_video() {
                                    status_pill(
                                        ui,
                                        &theme,
                                        "Video",
                                        theme.colors.video,
                                        Some(Icon::Video),
                                    );
                                }
                            });
                            ui.label(
                                RichText::new(format!(
                                    "{} / {}",
                                    nerd_ui_kit::format_clock(elapsed),
                                    nerd_ui_kit::format_clock(duration)
                                ))
                                .font(nerd_ui_kit::timecode(12.0))
                                .color(theme.colors.text_secondary),
                            );
                        });
                    });
                });
                let (wave, _) =
                    ui.allocate_exact_size(Vec2::new(ui.available_width(), 54.0), Sense::hover());
                ui.painter()
                    .rect_filled(wave, CornerRadius::same(5), theme.colors.well);
                let peaks = song
                    .audio
                    .as_ref()
                    .and_then(|path| app.library.find_by_path(path))
                    .map(|asset| asset.waveform.clone())
                    .unwrap_or_default();
                nerd_ui_kit::waveform(
                    ui.painter(),
                    wave.shrink2(Vec2::new(10.0, 8.0)),
                    &peaks,
                    accent,
                    Some(progress),
                );
            } else {
                let title = next
                    .as_ref()
                    .map(|song| song.display_title())
                    .unwrap_or_else(|| "Scaletta terminata".to_string());
                ui.label(
                    RichText::new(match phase {
                        Phase::Finished => "Serata conclusa",
                        Phase::Waiting => "In attesa del GO",
                        _ => "Pronto al via",
                    })
                    .font(caption())
                    .color(theme.colors.text_secondary),
                );
                ui.label(
                    RichText::new(title)
                        .font(nerd_ui_kit::live_title())
                        .color(theme.colors.text),
                );
                if let Some(gap) = gap.filter(|_| app.show.is_counting_down()) {
                    let seconds = gap as f32 / 1000.0;
                    ui.label(
                        RichText::new(format!("{seconds:.1}"))
                            .font(nerd_ui_kit::live_countdown())
                            .color(theme.colors.amber),
                    );
                    ui.label(
                        RichText::new("secondi al prossimo brano")
                            .font(caption())
                            .color(theme.colors.text_secondary),
                    );
                }
            }
        });
}

fn controls(app: &mut NerdApp, ui: &mut Ui) {
    let theme = app.palette;
    let phase = app.show.phase();
    let can_pause =
        matches!(phase, Phase::Playing | Phase::Paused) || app.show.background_running();
    let can_stop = phase != Phase::Idle;
    let can_move = app.show.live_index().is_some() || phase == Phase::Waiting;
    let can_panic = phase != Phase::Idle;
    egui::Frame::new()
        .fill(theme.colors.panel)
        .stroke(egui::Stroke::new(1.0, theme.colors.separator))
        .corner_radius(CornerRadius::same(12))
        .inner_margin(egui::Margin::same(16))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                if go_button(app, ui, &theme) {
                    app.request_go();
                }
                ui.add_space(10.0);
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        transport(
                            app,
                            ui,
                            &theme,
                            if app.show.is_paused() || app.show.background_paused() {
                                "Riprendi"
                            } else {
                                "Pausa"
                            },
                            can_pause,
                            |app| app.toggle_pause(),
                        );
                        transport(app, ui, &theme, "Stop", can_stop, |app| app.stop_show());
                        transport(app, ui, &theme, "Salta", can_move, |app| app.skip_song());
                        transport(app, ui, &theme, "Indietro", can_move, |app| {
                            app.previous_song()
                        });
                    });
                    ui.add_space(7.0);
                    let panic = Button::new(strings::PANIC)
                        .variant(ButtonVariant::Destructive)
                        .icon(Icon::Panic)
                        .size(nerd_ui_kit::ControlSize::Large)
                        .min_width(150.0)
                        .keycap("⇧⌘P")
                        .enabled(can_panic)
                        .show(ui, &theme);
                    if panic.clicked() {
                        app.panic();
                    }
                    ui.add_space(4.0);
                    keycap(ui, &theme, "Spazio");
                    ui.label(
                        RichText::new("GO · P pausa · S stop · N salta · B indietro")
                            .font(caption())
                            .color(theme.colors.text_secondary),
                    );
                });
            });
        });
}

fn go_button(app: &mut NerdApp, ui: &mut Ui, theme: &Theme) -> bool {
    let enabled = !app.show.is_empty();
    let (rect, response) = ui.allocate_exact_size(
        Vec2::new(190.0, 112.0),
        if enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    let fill = if !enabled {
        theme.colors.card
    } else if response.hovered() {
        theme.colors.green.gamma_multiply(1.12)
    } else {
        theme.colors.green
    };
    ui.painter().rect(
        rect,
        CornerRadius::same(13),
        fill,
        egui::Stroke::new(
            if enabled { 2.0 } else { 1.0 },
            if enabled {
                theme.colors.green.gamma_multiply(1.4)
            } else {
                theme.colors.separator
            },
        ),
        egui::StrokeKind::Inside,
    );
    let go_foreground = if !theme.dark {
        egui::Color32::WHITE
    } else {
        theme.colors.blackout
    };
    ui.painter().text(
        Pos2::new(rect.center().x, rect.top() + 16.0),
        Align2::CENTER_TOP,
        strings::GO,
        egui::FontId::proportional(42.0),
        if enabled {
            go_foreground
        } else {
            theme.colors.text_tertiary
        },
    );
    ui.painter().text(
        Pos2::new(rect.center().x, rect.bottom() - 28.0),
        Align2::CENTER_BOTTOM,
        if app.show.live_index().is_some() {
            "conferma salto"
        } else {
            "avvia il prossimo"
        },
        caption(),
        if enabled {
            go_foreground.gamma_multiply(0.82)
        } else {
            theme.colors.text_tertiary
        },
    );
    ui.painter().text(
        Pos2::new(rect.center().x, rect.bottom() - 12.0),
        Align2::CENTER_BOTTOM,
        "Spazio",
        nerd_ui_kit::micro(),
        if enabled {
            theme.colors.blackout.gamma_multiply(0.55)
        } else {
            theme.colors.text_tertiary
        },
    );
    response.clicked()
}

fn transport(
    app: &mut NerdApp,
    ui: &mut Ui,
    theme: &Theme,
    label: &str,
    enabled: bool,
    action: impl FnOnce(&mut NerdApp),
) {
    let icon_kind = match label {
        "Pausa" | "Riprendi" => Icon::Pause,
        "Stop" => Icon::Stop,
        "Salta" => Icon::SkipForward,
        "Indietro" => Icon::SkipBack,
        _ => Icon::Play,
    };
    if Button::new(label)
        .variant(ButtonVariant::Secondary)
        .icon(icon_kind)
        .size(nerd_ui_kit::ControlSize::Large)
        .min_width(102.0)
        .enabled(enabled)
        .show(ui, theme)
        .clicked()
    {
        action(app);
    }
}

fn next_card(app: &mut NerdApp, ui: &mut Ui) {
    let theme = app.palette;
    let next = app.next_song().cloned();
    let phase = app.show.phase();
    egui::Frame::new()
        .fill(theme.colors.panel)
        .stroke(egui::Stroke::new(1.0, theme.colors.separator))
        .corner_radius(CornerRadius::same(12))
        .inner_margin(egui::Margin::same(18))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                section_label(ui, &theme, "Prossimo");
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if app.show.is_counting_down() {
                        if let Some(gap) = app.show.gap_remaining_ms() {
                            status_pill(
                                ui,
                                &theme,
                                &format!("{:.1}s", gap as f32 / 1000.0),
                                theme.colors.amber,
                                Some(Icon::Clock),
                            );
                        }
                    }
                });
            });
            ui.add_space(5.0);
            if let Some(song) = next {
                ui.label(
                    RichText::new(song.display_title())
                        .size(22.0)
                        .strong()
                        .color(theme.colors.text),
                );
                ui.label(
                    RichText::new(if song.artist.is_empty() {
                        "Artista non indicato"
                    } else {
                        &song.artist
                    })
                    .font(body())
                    .color(theme.colors.text_secondary),
                );
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    status_pill(
                        ui,
                        &theme,
                        &strings::format_timecode(song.effective_duration_ms()),
                        theme.colors.text_secondary,
                        Some(Icon::Clock),
                    );
                    if song.has_video() {
                        status_pill(ui, &theme, "Video", theme.colors.video, Some(Icon::Video));
                    }
                });
                if !song.notes.trim().is_empty() {
                    ui.add_space(5.0);
                    ui.label(
                        RichText::new(&song.notes)
                            .font(caption())
                            .color(theme.colors.text_secondary),
                    );
                }
            } else {
                ui.label(
                    RichText::new("Fine della scaletta")
                        .size(18.0)
                        .color(theme.colors.text_secondary),
                );
                ui.label(
                    RichText::new(if phase == Phase::Finished {
                        "Il sottofondo accompagna l'uscita."
                    } else {
                        "Nessun brano in attesa."
                    })
                    .font(caption())
                    .color(theme.colors.text_secondary),
                );
            }
        });
}

fn output_card(app: &mut NerdApp, ui: &mut Ui) {
    let theme = app.palette;
    let state = app.projector_state(ui.ctx());
    egui::Frame::new()
        .fill(theme.colors.panel)
        .stroke(egui::Stroke::new(1.0, theme.colors.separator))
        .corner_radius(CornerRadius::same(12))
        .inner_margin(egui::Margin::same(16))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                section_label(ui, &theme, "Uscita video");
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    status_pill(
                        ui,
                        &theme,
                        if app.output_open { "Aperta" } else { "Chiusa" },
                        if app.output_open {
                            theme.colors.green
                        } else {
                            theme.colors.text_secondary
                        },
                        None,
                    );
                });
            });
            let preview_width = ui.available_width();
            let (rect, _) = ui.allocate_exact_size(
                Vec2::new(
                    preview_width,
                    (preview_width * 9.0 / 16.0).clamp(120.0, 210.0),
                ),
                Sense::hover(),
            );
            ui.painter()
                .rect_filled(rect, CornerRadius::same(6), theme.colors.blackout);
            output::paint_output(ui.painter(), rect, &state);
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(app.display_name())
                        .font(caption())
                        .color(theme.colors.text_secondary),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if Button::new(if app.project.output.blackout {
                        "Fine blackout"
                    } else {
                        "Blackout"
                    })
                    .variant(if app.project.output.blackout {
                        ButtonVariant::Destructive
                    } else {
                        ButtonVariant::Plain
                    })
                    .icon(Icon::Blackout)
                    .show(ui, &theme)
                    .clicked()
                    {
                        app.toggle_blackout();
                    }
                    if icon_button(
                        ui,
                        &theme,
                        Icon::Monitor,
                        Some("Apri o chiudi uscita"),
                        app.output_open,
                    )
                    .clicked()
                    {
                        app.toggle_output(ui.ctx());
                    }
                });
            });
        });
}

fn background_card(app: &mut NerdApp, ui: &mut Ui) {
    let theme = app.palette;
    let running = app.show.background_running();
    let paused = app.show.background_paused();
    let phase = app.show.phase();
    let can_start = !matches!(
        phase,
        Phase::Playing | Phase::Paused | Phase::Stopping | Phase::Panic
    );
    let volume = app.project.background.volume;
    egui::Frame::new()
        .fill(theme.colors.panel)
        .stroke(egui::Stroke::new(1.0, theme.colors.separator))
        .corner_radius(CornerRadius::same(12))
        .inner_margin(egui::Margin::same(16))
        .show(ui, |ui| {
            section_label(ui, &theme, "Sottofondo");
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
                        theme.colors.mint
                    } else {
                        theme.colors.text_secondary
                    }),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(
                        RichText::new(format!("{} brani", app.project.background.tracks.len()))
                            .font(caption())
                            .color(theme.colors.text_secondary),
                    );
                });
            });
            if app.project.background.tracks.is_empty() {
                ui.add_space(4.0);
                if Button::new("Configura in Impostazioni")
                    .variant(ButtonVariant::Plain)
                    .show(ui, &theme)
                    .clicked()
                {
                    app.page = Page::Settings;
                }
            }
            let mut volume = volume;
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("Volume")
                        .font(caption())
                        .color(theme.colors.text_secondary),
                );
                if slider(ui, &theme, &mut volume, 0.0..=1.5, "", 150.0).changed() {
                    app.project.background.volume = volume;
                    let _ = app.engine.set_background_volume(volume);
                    app.mark_dirty();
                }
                ui.label(
                    RichText::new(strings::format_volume(volume))
                        .font(caption())
                        .color(theme.colors.text),
                );
            });
            ui.horizontal(|ui| {
                if Button::new("Avvia")
                    .variant(ButtonVariant::Tinted)
                    .icon(Icon::Play)
                    .enabled(can_start && !running)
                    .show(ui, &theme)
                    .clicked()
                {
                    app.start_background_manual();
                }
                if Button::new("Ferma")
                    .variant(ButtonVariant::Plain)
                    .icon(Icon::Stop)
                    .enabled(running)
                    .show(ui, &theme)
                    .clicked()
                {
                    app.stop_background_manual();
                }
                if Button::new(if paused { "Riprendi" } else { "Pausa" })
                    .variant(ButtonVariant::Plain)
                    .enabled(running)
                    .show(ui, &theme)
                    .clicked()
                {
                    app.toggle_pause();
                }
            });
        });
}

fn meters(app: &mut NerdApp, ui: &mut Ui) {
    let theme = app.palette;
    let mut master = app.prefs.master_volume;
    egui::Frame::new()
        .fill(theme.colors.panel)
        .stroke(egui::Stroke::new(1.0, theme.colors.separator))
        .corner_radius(CornerRadius::same(12))
        .inner_margin(egui::Margin::same(16))
        .show(ui, |ui| {
            section_label(ui, &theme, "Mix");
            ui.add_space(3.0);
            nerd_ui_kit::vu_meter(
                ui,
                &theme,
                "Brano",
                app.song_meter,
                app.song_meter,
                (app.song_meter + 0.08).min(1.0),
                (app.song_meter + 0.08).min(1.0),
            );
            ui.add_space(2.0);
            nerd_ui_kit::vu_meter(
                ui,
                &theme,
                "Sottofondo",
                app.bg_meter,
                app.bg_meter,
                (app.bg_meter + 0.06).min(1.0),
                (app.bg_meter + 0.06).min(1.0),
            );
            ui.add_space(5.0);
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("Master")
                        .font(caption())
                        .color(theme.colors.text_secondary),
                );
                if slider(ui, &theme, &mut master, 0.0..=1.0, "", 180.0).changed() {
                    app.prefs.master_volume = master;
                    app.prefs_dirty = true;
                    let _ = app.engine.set_master_volume(master);
                }
                ui.label(
                    RichText::new(strings::format_volume(master))
                        .font(caption())
                        .color(theme.colors.text),
                );
            });
        });
}

fn checklist(app: &mut NerdApp, ui: &mut Ui) {
    let theme = app.palette;
    let complete = app.checklist.complete();
    egui::Frame::new()
        .fill(theme.colors.panel)
        .stroke(egui::Stroke::new(
            1.0,
            if complete {
                theme.colors.green.gamma_multiply(0.5)
            } else {
                theme.colors.separator
            },
        ))
        .corner_radius(CornerRadius::same(12))
        .inner_margin(egui::Margin::same(16))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                section_label(ui, &theme, "Checklist pre-show");
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    status_pill(
                        ui,
                        &theme,
                        if complete { "Pronto" } else { "Da verificare" },
                        if complete {
                            theme.colors.green
                        } else {
                            theme.colors.amber
                        },
                        Some(if complete { Icon::Check } else { Icon::Warning }),
                    );
                });
            });
            let mut audio = app.checklist.audio_output;
            let mut projector = app.checklist.projector;
            let mut volumes = app.checklist.volumes;
            if toggle(ui, &theme, &mut audio, "Uscita audio collegata e testata").changed() {
                app.checklist.audio_output = audio;
            }
            if toggle(ui, &theme, &mut projector, "Proiettore acceso sulla scena").changed() {
                app.checklist.projector = projector;
            }
            if toggle(ui, &theme, &mut volumes, "Volumi e sottofondo verificati").changed() {
                app.checklist.volumes = volumes;
            }
            if !complete && primary_button(ui, &theme, "Segna tutto pronto").clicked() {
                app.checklist.audio_output = true;
                app.checklist.projector = true;
                app.checklist.volumes = true;
            }
            if complete
                && Button::new("Azzera checklist")
                    .variant(ButtonVariant::Plain)
                    .show(ui, &theme)
                    .clicked()
            {
                app.checklist = crate::state::Checklist::default();
            }
        });
}

fn queue(app: &mut NerdApp, ui: &mut Ui) {
    let theme = app.palette;
    egui::Frame::new()
        .fill(theme.colors.panel)
        .stroke(egui::Stroke::new(1.0, theme.colors.separator))
        .corner_radius(CornerRadius::same(12))
        .inner_margin(egui::Margin::same(16))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                section_label(ui, &theme, "Scaletta");
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if Button::new("Apri scaletta")
                        .variant(ButtonVariant::Plain)
                        .icon(Icon::Timeline)
                        .show(ui, &theme)
                        .clicked()
                    {
                        app.page = Page::Timeline;
                    }
                });
            });
            ui.add_space(4.0);
            let live_index = app.show.live_index();
            let songs: Vec<_> = app.project.songs.iter().cloned().enumerate().collect();
            for (index, song) in songs.into_iter().take(8) {
                let current = live_index == Some(index);
                let next = app.show.cursor_index() == Some(index);
                let width = ui.available_width();
                let (rect, response) =
                    ui.allocate_exact_size(Vec2::new(width, 32.0), Sense::click());
                if current || next || response.hovered() {
                    ui.painter().rect_filled(
                        rect,
                        CornerRadius::same(5),
                        if current {
                            theme.colors.green.gamma_multiply(0.14)
                        } else if next {
                            theme.colors.accent_soft
                        } else {
                            theme.colors.hover
                        },
                    );
                }
                ui.painter().text(
                    Pos2::new(rect.left() + 8.0, rect.center().y),
                    Align2::LEFT_CENTER,
                    format!("{:02}", index + 1),
                    nerd_ui_kit::timecode(10.0),
                    theme.colors.text_tertiary,
                );
                ui.painter().text(
                    Pos2::new(rect.left() + 34.0, rect.center().y),
                    Align2::LEFT_CENTER,
                    song.display_title(),
                    body(),
                    if current || next {
                        theme.colors.text
                    } else {
                        theme.colors.text_secondary
                    },
                );
                ui.painter().text(
                    Pos2::new(rect.right() - 8.0, rect.center().y),
                    Align2::RIGHT_CENTER,
                    strings::format_timecode(song.effective_duration_ms()),
                    nerd_ui_kit::timecode(10.0),
                    theme.colors.text_tertiary,
                );
                if current {
                    ui.painter().rect_filled(
                        Rect::from_min_size(rect.min, Vec2::new(2.0, rect.height())),
                        CornerRadius::same(1),
                        theme.colors.green,
                    );
                }
                if response.clicked() {
                    app.selected_song = Some(song.id.clone());
                    app.page = Page::Timeline;
                }
            }
        });
}

fn shortcuts(ui: &mut Ui, theme: &Theme) {
    ui.horizontal_wrapped(|ui| {
        for (key, label) in [
            ("Spazio", "GO"),
            ("P", "Pausa"),
            ("S", "Stop"),
            ("N", "Salta"),
            ("B", "Indietro"),
            ("⇧⌘P", "Panic"),
            ("⌘B", "Blackout"),
        ] {
            keycap(ui, theme, key);
            ui.label(
                RichText::new(label)
                    .font(caption())
                    .color(theme.colors.text_secondary),
            );
            ui.add_space(4.0);
        }
    });
}
