//! Settings: audio, playback, background, projector, updates and data.

use eframe::egui::{self, Align, ComboBox, Layout, RichText, ScrollArea, Stroke, Ui};
use nerd_audio::AudioEngine;
use nerd_core::library::AUDIO_EXTENSIONS;
use nerd_core::model::{IdleScreen, PlaybackMode, ResumeMode};
use nerd_core::prefs::{ThemeMode, UpdateChannel};

use crate::app::NerdApp;
use crate::state::ToastKind;
use crate::theme::{self, Palette};
use crate::update::UpdateStatus;
use crate::widgets;

pub fn show(app: &mut NerdApp, ui: &mut Ui) {
    egui::Frame::new()
        .fill(app.palette.background)
        .inner_margin(egui::Margin::symmetric(18, 14))
        .show(ui, |ui| {
            ScrollArea::vertical()
                .id_salt("settings-scroll")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    header(app, ui);
                    ui.add_space(12.0);
                    if ui.available_width() > 940.0 {
                        ui.columns(2, |columns| {
                            left_column(app, &mut columns[0]);
                            right_column(app, &mut columns[1]);
                        });
                    } else {
                        left_column(app, ui);
                        right_column(app, ui);
                    }
                    project_card(app, ui);
                    data_card(app, ui);
                });
        });
}

fn header(app: &mut NerdApp, ui: &mut Ui) {
    let palette = app.palette;
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(
                RichText::new(crate::strings::SETTINGS)
                    .size(22.0)
                    .strong()
                    .color(palette.text),
            );
            ui.label(
                RichText::new("Audio, scaletta, proiezione e aggiornamenti della serata.")
                    .size(10.5)
                    .color(palette.text_muted),
            );
        });
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(
                RichText::new(format!("v{}", nerd_core::APP_VERSION))
                    .size(10.0)
                    .color(palette.text_muted)
                    .monospace(),
            );
            if widgets::accent_button(ui, palette, "Salva serata").clicked() {
                app.save(false);
            }
        });
    });
}

fn left_column(app: &mut NerdApp, ui: &mut Ui) {
    audio_card(app, ui);
    playback_card(app, ui);
    appearance_card(app, ui);
}

fn right_column(app: &mut NerdApp, ui: &mut Ui) {
    background_card(app, ui);
    output_card(app, ui);
    updates_card(app, ui);
}

fn card(ui: &mut Ui, palette: Palette, title: &str, add: impl FnOnce(&mut Ui)) {
    theme::card_frame(palette).show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        widgets::section_label(ui, palette, title);
        ui.add_space(6.0);
        add(ui);
    });
    ui.add_space(12.0);
}

fn hint(ui: &mut Ui, palette: Palette, text: &str) {
    ui.label(RichText::new(text).size(9.5).color(palette.text_muted));
}

fn audio_card(app: &mut NerdApp, ui: &mut Ui) {
    let palette = app.palette;
    card(ui, palette, "Uscita audio", |ui| {
        let status = app.engine.status();
        let devices = AudioEngine::output_devices();
        let mut chosen = app.prefs.audio_device.clone();
        let selected = chosen
            .clone()
            .unwrap_or_else(|| "Predefinito di sistema".to_string());

        ComboBox::from_id_salt("settings-audio-device")
            .selected_text(selected)
            .width(ui.available_width().min(330.0))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut chosen, None, "Predefinito di sistema");
                for device in &devices {
                    let label = if device.is_default {
                        format!("{} (predefinito)", device.name)
                    } else {
                        device.name.clone()
                    };
                    ui.selectable_value(&mut chosen, Some(device.name.clone()), label);
                }
            });
        if chosen != app.prefs.audio_device {
            app.prefs.audio_device = chosen;
            app.engine
                .set_preferred_device(app.prefs.audio_device.clone());
            app.prefs_dirty = true;
            app.toast("Dispositivo audio aggiornato.", ToastKind::Info);
        }

        ui.add_space(4.0);
        let (text, color) = if let Some(error) = &status.error {
            (format!("Errore uscita: {error}"), palette.danger)
        } else if !status.available {
            (
                "Nessuna uscita audio disponibile".to_string(),
                palette.warning,
            )
        } else {
            let name = status.device_name.as_deref().unwrap_or("Predefinita");
            (
                format!(
                    "{name} · {} Hz · {} canali",
                    status.sample_rate, status.channels
                ),
                palette.text_muted,
            )
        };
        ui.label(RichText::new(text).size(10.0).color(color));
        if let Some(warning) = &status.warning {
            ui.label(RichText::new(warning).size(9.5).color(palette.warning));
        }

        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new("Master").size(10.5).color(palette.text_muted));
            if ui
                .add(egui::Slider::new(&mut app.prefs.master_volume, 0.0..=1.0).show_value(false))
                .changed()
            {
                let volume = app.prefs.master_volume;
                let _ = app.engine.set_master_volume(volume);
                app.prefs_dirty = true;
            }
            ui.label(
                RichText::new(crate::strings::format_volume(app.prefs.master_volume))
                    .size(9.5)
                    .color(palette.text)
                    .monospace(),
            );
        });
        hint(
            ui,
            palette,
            "La disconnessione e la riconnessione del dispositivo sono gestite automaticamente.",
        );
    });
}

fn playback_card(app: &mut NerdApp, ui: &mut Ui) {
    let palette = app.palette;
    card(ui, palette, "Riproduzione", |ui| {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("Modalità predefinita")
                    .size(10.5)
                    .color(palette.text_muted),
            );
            for mode in [PlaybackMode::Manual, PlaybackMode::Automatic] {
                let selected = app.project.show.mode == mode;
                if ui
                    .selectable_label(selected, widgets::mode_label(mode))
                    .clicked()
                {
                    app.set_show_mode(mode);
                }
            }
        });
        ui.add_space(4.0);

        let mut gap = app.project.show.default_gap_ms as f32 / 1000.0;
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("Pausa tra i brani")
                    .size(10.5)
                    .color(palette.text_muted),
            );
            if ui
                .add(
                    egui::Slider::new(&mut gap, 0.0..=30.0)
                        .suffix(" s")
                        .step_by(0.5),
                )
                .changed()
            {
                app.project.show.default_gap_ms = (gap * 1000.0).round() as u64;
                app.sync_show_config();
                app.mark_dirty();
            }
        });

        let mut panic_fade = app.project.show.panic_fade_ms as f32;
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("Fade Panic")
                    .size(10.5)
                    .color(palette.text_muted),
            );
            if ui
                .add(
                    egui::Slider::new(&mut panic_fade, 50.0..=2_000.0)
                        .suffix(" ms")
                        .step_by(10.0),
                )
                .changed()
            {
                app.project.show.panic_fade_ms = panic_fade.round() as u64;
                app.sync_show_config();
                app.mark_dirty();
            }
        });

        ui.add_space(4.0);
        if ui
            .checkbox(
                &mut app.prefs.confirm_quit_during_show,
                "Chiedi conferma prima di chiudere durante lo show",
            )
            .changed()
        {
            app.prefs_dirty = true;
        }
        hint(
            ui,
            palette,
            "La modalità può essere cambiata anche dal vivo dalla pagina Live.",
        );
    });
}

fn appearance_card(app: &mut NerdApp, ui: &mut Ui) {
    let palette = app.palette;
    card(ui, palette, "Aspetto", |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Tema").size(10.5).color(palette.text_muted));
            let mut mode = app.prefs.theme;
            ui.selectable_value(&mut mode, ThemeMode::Dark, "Scuro");
            ui.selectable_value(&mut mode, ThemeMode::Light, "Chiaro");
            ui.selectable_value(&mut mode, ThemeMode::System, "Sistema");
            if mode != app.prefs.theme {
                app.prefs.theme = mode;
                app.palette = theme::install(ui.ctx(), mode);
                app.prefs_dirty = true;
            }
        });
        hint(
            ui,
            palette,
            "Il tema chiaro mantiene contrasto e aree di sicurezza della proiezione.",
        );
    });
}

fn background_card(app: &mut NerdApp, ui: &mut Ui) {
    let palette = app.palette;
    card(ui, palette, "Sottofondo", |ui| {
        let tracks = app.project.background.tracks.clone();
        if tracks.is_empty() {
            ui.label(
                RichText::new("Nessun brano di sottofondo configurato.")
                    .size(10.5)
                    .color(palette.text_muted),
            );
        } else {
            let mut remove = None;
            ScrollArea::vertical()
                .id_salt("settings-background-tracks")
                .max_height(132.0)
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    for (index, path) in tracks.iter().enumerate() {
                        ui.horizontal(|ui| {
                            let name = path
                                .file_name()
                                .and_then(|name| name.to_str())
                                .unwrap_or("Brano");
                            ui.label(RichText::new(name).size(10.5).color(palette.text));
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if ui
                                    .add(
                                        egui::Button::new(
                                            RichText::new("Rimuovi")
                                                .size(9.5)
                                                .color(palette.text_muted),
                                        )
                                        .frame(false),
                                    )
                                    .clicked()
                                {
                                    remove = Some(index);
                                }
                            });
                        });
                    }
                });
            if let Some(index) = remove {
                let before = app.project.clone();
                app.project.background.tracks.remove(index);
                app.commit_edit(&before);
                app.reload_background();
            }
            ui.add_space(2.0);
        }

        if widgets::ghost_button(ui, palette, "Aggiungi brani…").clicked() {
            if let Some(paths) = rfd::FileDialog::new()
                .set_title("Aggiungi al sottofondo")
                .add_filter("Audio", AUDIO_EXTENSIONS)
                .pick_files()
            {
                let before = app.project.clone();
                for path in paths {
                    if !app.project.background.tracks.contains(&path) {
                        app.project.background.tracks.push(path);
                    }
                }
                app.commit_edit(&before);
                app.reload_background();
            }
        }

        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new("Volume").size(10.5).color(palette.text_muted));
            let mut volume = app.project.background.volume;
            if ui
                .add(egui::Slider::new(&mut volume, 0.0..=1.5).show_value(false))
                .changed()
            {
                app.project.background.volume = volume;
                let _ = app.engine.set_background_volume(volume);
                app.mark_dirty();
            }
            ui.label(
                RichText::new(crate::strings::format_volume(app.project.background.volume))
                    .size(9.5)
                    .color(palette.text)
                    .monospace(),
            );
        });

        ui.horizontal(|ui| {
            if ui
                .checkbox(&mut app.project.background.looping, "Ripeti la playlist")
                .changed()
            {
                app.mark_dirty();
            }
            if ui
                .checkbox(&mut app.project.background.shuffle, "Ordine casuale")
                .changed()
            {
                app.mark_dirty();
            }
        });

        let mut fade_in = app.project.background.fade_in_ms as f32 / 1000.0;
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("Fade in")
                    .size(10.5)
                    .color(palette.text_muted),
            );
            if ui
                .add(
                    egui::Slider::new(&mut fade_in, 0.0..=10.0)
                        .suffix(" s")
                        .step_by(0.1),
                )
                .changed()
            {
                app.project.background.fade_in_ms = (fade_in * 1000.0).round() as u64;
                app.sync_show_config();
                app.mark_dirty();
            }
        });

        let mut fade_out = app.project.background.fade_out_ms as f32 / 1000.0;
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("Fade out")
                    .size(10.5)
                    .color(palette.text_muted),
            );
            if ui
                .add(
                    egui::Slider::new(&mut fade_out, 0.0..=10.0)
                        .suffix(" s")
                        .step_by(0.1),
                )
                .changed()
            {
                app.project.background.fade_out_ms = (fade_out * 1000.0).round() as u64;
                app.sync_show_config();
                app.mark_dirty();
            }
        });

        ui.horizontal(|ui| {
            ui.label(
                RichText::new("Ripresa")
                    .size(10.5)
                    .color(palette.text_muted),
            );
            let mut resume = app.project.background.resume_mode;
            ComboBox::from_id_salt("settings-background-resume")
                .selected_text(resume_label(resume))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut resume, ResumeMode::Resume, "Riprendi da dove era");
                    ui.selectable_value(&mut resume, ResumeMode::Restart, "Ricomincia la playlist");
                });
            if resume != app.project.background.resume_mode {
                app.project.background.resume_mode = resume;
                app.mark_dirty();
            }
        });
        hint(
            ui,
            palette,
            "Le playlist vengono ricaricate in background; le modifiche non interrompono lo show.",
        );
    });
}

fn resume_label(mode: ResumeMode) -> &'static str {
    match mode {
        ResumeMode::Resume => "Riprendi da dove era",
        ResumeMode::Restart => "Ricomincia",
    }
}

fn output_card(app: &mut NerdApp, ui: &mut Ui) {
    let palette = app.palette;
    card(ui, palette, crate::strings::OUTPUT, |ui| {
        let displays = app.displays.clone();
        let current = app.display_name();
        let mut chosen = app.project.output.display_index;
        ComboBox::from_id_salt("settings-output-display")
            .selected_text(current)
            .width(ui.available_width().min(330.0))
            .show_ui(ui, |ui| {
                for display in &displays {
                    let label = format!("{} · {}×{}", display.name, display.width, display.height);
                    ui.selectable_value(&mut chosen, display.index, label);
                }
            });
        if chosen != app.project.output.display_index {
            app.project.output.display_index = chosen;
            app.mark_dirty();
            if app.output_open {
                app.toggle_output(ui.ctx());
                app.toast(
                    "Uscita chiusa: riaprila per usare il nuovo schermo.",
                    ToastKind::Info,
                );
            }
        }

        ui.horizontal(|ui| {
            if ui
                .checkbox(&mut app.project.output.video_enabled, "Video dei brani")
                .changed()
            {
                app.mark_dirty();
            }
            if ui
                .checkbox(
                    &mut app.project.output.show_titles,
                    "Titoli in sovrimpressione",
                )
                .changed()
            {
                app.mark_dirty();
            }
        });

        ui.horizontal(|ui| {
            ui.label(
                RichText::new("Schermata di attesa")
                    .size(10.5)
                    .color(palette.text_muted),
            );
            let mut idle = app.project.output.idle_screen;
            ComboBox::from_id_salt("settings-output-idle")
                .selected_text(match idle {
                    IdleScreen::Courtesy => "Cortesia",
                    IdleScreen::Black => "Nero",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut idle, IdleScreen::Courtesy, "Cortesia");
                    ui.selectable_value(&mut idle, IdleScreen::Black, "Nero");
                });
            if idle != app.project.output.idle_screen {
                app.project.output.idle_screen = idle;
                app.mark_dirty();
            }
        });

        if app.project.output.idle_screen == IdleScreen::Courtesy {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("Messaggio")
                        .size(10.5)
                        .color(palette.text_muted),
                );
                if ui
                    .add(
                        egui::TextEdit::singleline(&mut app.project.output.courtesy_message)
                            .desired_width(220.0),
                    )
                    .changed()
                {
                    app.mark_dirty();
                }
            });
        }

        let logo = app.project.output.logo_path.clone();
        ui.horizontal(|ui| {
            let name = logo
                .as_ref()
                .and_then(|path| path.file_name())
                .and_then(|name| name.to_str())
                .unwrap_or("Nessun logo");
            ui.label(RichText::new(name).size(10.5).color(palette.text_muted));
            if widgets::ghost_button(ui, palette, "Scegli…").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .set_title("Logo dell'uscita video")
                    .add_filter("Immagini", &["png", "jpg", "jpeg"])
                    .pick_file()
                {
                    app.project.output.logo_path = Some(path);
                    app.mark_dirty();
                }
            }
            if logo.is_some() && ui.small_button("Rimuovi").clicked() {
                app.project.output.logo_path = None;
                app.mark_dirty();
            }
        });

        ui.add_space(4.0);
        ui.horizontal(|ui| {
            if widgets::ghost_button(ui, palette, "Rileggi schermi").clicked() {
                app.refresh_displays();
                app.mark_dirty();
                app.toast("Elenco schermi aggiornato.", ToastKind::Info);
            }
            if ui
                .add(
                    egui::Button::new(
                        RichText::new(if app.project.output.blackout {
                            "Fine blackout"
                        } else {
                            crate::strings::BLACKOUT
                        })
                        .size(10.5)
                        .color(if app.project.output.blackout {
                            palette.blackout
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
                    .corner_radius(egui::CornerRadius::same(7)),
                )
                .clicked()
            {
                app.toggle_blackout();
            }
            if widgets::ghost_button(
                ui,
                palette,
                if app.output_open {
                    "Chiudi uscita"
                } else {
                    "Apri uscita"
                },
            )
            .clicked()
            {
                app.toggle_output(ui.ctx());
            }
        });
        hint(
            ui,
            palette,
            "L'uscita è indipendente dalla finestra di regia e si adatta all'hot-plug.",
        );
    });
}

fn updates_card(app: &mut NerdApp, ui: &mut Ui) {
    let palette = app.palette;
    card(ui, palette, "Aggiornamenti", |ui| {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("Repository")
                    .size(10.5)
                    .color(palette.text_muted),
            );
            if ui
                .add(
                    egui::TextEdit::singleline(&mut app.prefs.update_repo)
                        .hint_text("owner/repo")
                        .desired_width(200.0),
                )
                .changed()
            {
                app.prefs_dirty = true;
            }
        });

        ui.horizontal(|ui| {
            ui.label(RichText::new("Canale").size(10.5).color(palette.text_muted));
            let mut channel = app.prefs.update_channel;
            ui.selectable_value(&mut channel, UpdateChannel::Stable, "Stabile");
            ui.selectable_value(&mut channel, UpdateChannel::Alpha, "Alpha");
            if channel != app.prefs.update_channel {
                app.prefs.update_channel = channel;
                app.prefs_dirty = true;
            }
        });

        if ui
            .checkbox(
                &mut app.prefs.check_updates_on_startup,
                "Controlla gli aggiornamenti all'avvio",
            )
            .changed()
        {
            app.prefs_dirty = true;
        }

        let status_text = app.updater.status.description();
        let status_color = match &app.updater.status {
            UpdateStatus::Error(_) => palette.danger,
            UpdateStatus::Available(_) | UpdateStatus::Ready(_) => palette.success,
            UpdateStatus::Checking
            | UpdateStatus::Downloading { .. }
            | UpdateStatus::Installing => palette.warning,
            _ => palette.text_muted,
        };
        ui.label(RichText::new(status_text).size(10.0).color(status_color));

        let busy = app.updater.is_busy();
        let ready = matches!(app.updater.status, UpdateStatus::Ready(_));
        let available = matches!(app.updater.status, UpdateStatus::Available(_));
        ui.horizontal(|ui| {
            if ui
                .add_enabled(!busy, egui::Button::new("Controlla ora"))
                .clicked()
            {
                app.check_updates(true);
            }
            if available && ui.button("Scarica").clicked() {
                let repo = app.prefs.update_repo.clone();
                if let Some(release) = app.updater.available().cloned() {
                    app.updater.download(&repo, release);
                }
            }
            if ready && widgets::accent_button(ui, palette, "Installa e riavvia").clicked() {
                app.install_update(ui.ctx());
            }
        });
        hint(
            ui,
            palette,
            "Gli aggiornamenti sono firmati e non vengono mai installati durante lo show.",
        );
    });
}

fn project_card(app: &mut NerdApp, ui: &mut Ui) {
    let palette = app.palette;
    card(ui, palette, "Serata e progetto", |ui| {
        let before = app.project.clone();
        let mut changed = false;

        ui.horizontal(|ui| {
            ui.label(RichText::new("Nome").size(10.5).color(palette.text_muted));
            changed |= ui
                .add(
                    egui::TextEdit::singleline(&mut app.project.name)
                        .desired_width(ui.available_width().min(320.0)),
                )
                .changed();
            ui.label(RichText::new("Data").size(10.5).color(palette.text_muted));
            changed |= ui
                .add(
                    egui::TextEdit::singleline(&mut app.project.date)
                        .hint_text("AAAA-MM-GG")
                        .desired_width(110.0),
                )
                .changed();
        });

        ui.horizontal(|ui| {
            ui.label(RichText::new("Luogo").size(10.5).color(palette.text_muted));
            changed |= ui
                .add(
                    egui::TextEdit::singleline(&mut app.project.venue)
                        .desired_width(ui.available_width().min(420.0)),
                )
                .changed();
        });

        changed |= ui
            .add(
                egui::TextEdit::multiline(&mut app.project.notes)
                    .hint_text("Note per la regia")
                    .desired_rows(2),
            )
            .changed();

        if changed {
            app.commit_edit(&before);
        }

        ui.add_space(4.0);
        ui.horizontal(|ui| {
            if widgets::ghost_button(ui, palette, "Salva").clicked() {
                app.save(false);
            }
            if widgets::ghost_button(ui, palette, "Salva con nome…").clicked() {
                app.save(true);
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let path = app
                    .project_path
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|| "Non ancora salvata".to_string());
                ui.label(RichText::new(path).size(9.5).color(palette.text_muted));
            });
        });
    });
}

fn data_card(app: &mut NerdApp, ui: &mut Ui) {
    let palette = app.palette;
    card(ui, palette, "Dati e diagnostica", |ui| {
        ui.horizontal_wrapped(|ui| {
            if widgets::ghost_button(ui, palette, "Cartella dati").clicked() {
                app.open_folder(&app.paths.support);
            }
            if widgets::ghost_button(ui, palette, "Log").clicked() {
                app.open_folder(&app.paths.logs);
            }
            if widgets::ghost_button(ui, palette, "Cache").clicked() {
                app.open_folder(&app.paths.cache);
            }
            if widgets::ghost_button(ui, palette, "Scorciatoie").clicked() {
                app.show_shortcuts = true;
            }
        });
        ui.add_space(4.0);
        ui.label(
            RichText::new(format!(
                "{} {} · schema {} · macOS {}+",
                nerd_core::APP_NAME,
                nerd_core::APP_VERSION,
                nerd_core::CURRENT_SCHEMA_VERSION,
                nerd_core::branding::MIN_MACOS_VERSION
            ))
            .size(9.5)
            .color(palette.text_muted)
            .monospace(),
        );
        ui.label(
            RichText::new(format!(
                "{} brani · {} media in libreria · sessione attiva da {} min",
                app.project.songs.len(),
                app.library.assets.len(),
                app.started.elapsed().as_secs() / 60
            ))
            .size(9.5)
            .color(palette.text_muted),
        );
        if let Some(error) = &app.menu.error {
            ui.label(
                RichText::new(format!("Menu nativo non disponibile: {error}"))
                    .size(9.5)
                    .color(palette.danger),
            );
        }
    });
}
