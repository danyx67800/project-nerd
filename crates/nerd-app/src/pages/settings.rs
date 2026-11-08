//! Settings, grouped like macOS System Settings.

use eframe::egui::{self, Align, Layout, RichText, ScrollArea, Ui, Vec2};
use nerd_audio::AudioEngine;
use nerd_core::library::AUDIO_EXTENSIONS;
use nerd_core::model::{IdleScreen, PlaybackMode, ResumeMode};
use nerd_core::prefs::{ThemeMode, UpdateChannel};

use crate::app::NerdApp;
use crate::state::Page;
use crate::strings;
use crate::theme;
use crate::update::UpdateStatus;
use nerd_ui_kit::{
    button, caption, card, icon_button, keycap, labeled_field, menu_button, menu_item,
    primary_button, section_label, segmented, slider, text_field, toggle, Button, ButtonVariant,
    Icon, Theme,
};

pub fn show(app: &mut NerdApp, ui: &mut Ui) {
    egui::Frame::new()
        .fill(app.palette.colors.window)
        .inner_margin(egui::Margin::symmetric(18, 14))
        .show(ui, |ui| {
            ScrollArea::vertical()
                .id_salt("settings-scroll")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    header(app, ui);
                    ui.add_space(14.0);
                    let width = ui.available_width().min(820.0);
                    ui.allocate_ui_with_layout(
                        Vec2::new(width, 0.0),
                        Layout::top_down(Align::Min),
                        |ui| {
                            ui.set_width(width);
                            audio(app, ui);
                            screens(app, ui);
                            background(app, ui);
                            automation(app, ui);
                            titles(app, ui);
                            appearance(app, ui);
                            updates(app, ui);
                            project(app, ui);
                            data(app, ui);
                            shortcuts(ui, &app.palette);
                        },
                    );
                });
        });
}

fn header(app: &mut NerdApp, ui: &mut Ui) {
    let theme = app.palette;
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(
                RichText::new("Impostazioni")
                    .size(22.0)
                    .strong()
                    .color(theme.colors.text),
            );
            ui.label(
                RichText::new("Audio, proiezione e comportamento della serata.")
                    .font(caption())
                    .color(theme.colors.text_secondary),
            );
        });
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(
                RichText::new(format!("v{}", nerd_core::APP_VERSION))
                    .font(nerd_ui_kit::timecode(11.0))
                    .color(theme.colors.text_tertiary),
            );
            if Button::new("Salva serata")
                .variant(ButtonVariant::Primary)
                .icon(Icon::Save)
                .show(ui, &theme)
                .clicked()
            {
                app.save(false);
            }
        });
    });
    if let Some(page) = app
        .project_path
        .as_ref()
        .and_then(|path| path.file_name())
        .and_then(|name| name.to_str())
    {
        ui.label(
            RichText::new(page)
                .font(caption())
                .color(theme.colors.text_tertiary),
        );
    }
}

fn group(ui: &mut Ui, theme: &Theme, title: &str, add: impl FnOnce(&mut Ui)) {
    card(ui, theme, |ui| {
        section_label(ui, theme, title);
        ui.add_space(6.0);
        add(ui);
    });
    ui.add_space(10.0);
}

fn row(ui: &mut Ui, theme: &Theme, label: &str, description: &str, right: impl FnOnce(&mut Ui)) {
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(RichText::new(label).size(12.5).color(theme.colors.text));
            if !description.is_empty() {
                ui.label(
                    RichText::new(description)
                        .font(caption())
                        .color(theme.colors.text_secondary),
                );
            }
        });
        ui.with_layout(Layout::right_to_left(Align::Center), right);
    });
    ui.add_space(2.0);
}

fn audio(app: &mut NerdApp, ui: &mut Ui) {
    let theme = app.palette;
    let status = app.engine.status();
    let devices = AudioEngine::output_devices();
    let current = app
        .prefs
        .audio_device
        .clone()
        .unwrap_or_else(|| "Predefinito di sistema".to_string());
    group(ui, &theme, "Audio", |ui| {
        row(
            ui,
            &theme,
            "Uscita audio",
            "Dispositivo usato dalla regia",
            |ui| {
                menu_button(
                    ui,
                    &theme,
                    "audio-device",
                    &current,
                    Some(Icon::Volume),
                    ButtonVariant::Secondary,
                    260.0,
                    |ui| {
                        if menu_item(
                            ui,
                            &theme,
                            "Predefinito di sistema",
                            (!app.prefs.audio_device.is_some()).then_some(Icon::Check),
                        )
                        .clicked()
                        {
                            app.prefs.audio_device = None;
                            app.engine.set_preferred_device(None);
                            app.prefs_dirty = true;
                        }
                        for device in &devices {
                            let selected =
                                app.prefs.audio_device.as_deref() == Some(device.name.as_str());
                            let label = if device.is_default {
                                format!("{} (predefinito)", device.name)
                            } else {
                                device.name.clone()
                            };
                            if menu_item(ui, &theme, &label, selected.then_some(Icon::Check))
                                .clicked()
                            {
                                app.prefs.audio_device = Some(device.name.clone());
                                app.engine.set_preferred_device(Some(device.name.clone()));
                                app.prefs_dirty = true;
                            }
                        }
                    },
                );
            },
        );
        let (text, color) = if let Some(error) = &status.error {
            (error.clone(), theme.colors.red)
        } else if !status.available {
            (
                "Nessuna uscita audio disponibile".to_string(),
                theme.colors.amber,
            )
        } else {
            (
                format!(
                    "{} · {} Hz · {} canali",
                    status.device_name.as_deref().unwrap_or("Predefinita"),
                    status.sample_rate,
                    status.channels
                ),
                theme.colors.text_secondary,
            )
        };
        ui.label(RichText::new(text).font(caption()).color(color));
        if let Some(warning) = &status.warning {
            ui.label(
                RichText::new(warning)
                    .font(caption())
                    .color(theme.colors.amber),
            );
        }
        ui.add_space(6.0);
        nerd_ui_kit::divider(ui, &theme);
        let mut master = app.prefs.master_volume;
        row(
            ui,
            &theme,
            "Volume master",
            "Applicato sopra il mix della serata",
            |ui| {
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
            },
        );
    });
}

fn screens(app: &mut NerdApp, ui: &mut Ui) {
    let theme = app.palette;
    let displays = app.displays.clone();
    let current = app.display_name();
    group(ui, &theme, "Schermi e uscita video", |ui| {
        row(
            ui,
            &theme,
            "Schermo di uscita",
            "Finestra indipendente per il proiettore",
            |ui| {
                menu_button(
                    ui,
                    &theme,
                    "output-display",
                    &current,
                    Some(Icon::Monitor),
                    ButtonVariant::Secondary,
                    250.0,
                    |ui| {
                        for display in &displays {
                            let selected = display.index == app.project.output.display_index;
                            let label =
                                format!("{} · {}×{}", display.name, display.width, display.height);
                            if menu_item(ui, &theme, &label, selected.then_some(Icon::Check))
                                .clicked()
                            {
                                app.project.output.display_index = display.index;
                                app.mark_dirty();
                            }
                        }
                        nerd_ui_kit::divider(ui, &theme);
                        if menu_item(ui, &theme, "Rileggi schermi", Some(Icon::Refresh)).clicked() {
                            app.refresh_displays();
                        }
                    },
                );
            },
        );
        let mut video = app.project.output.video_enabled;
        if toggle(ui, &theme, &mut video, "Riproduci i video musicali").changed() {
            app.project.output.video_enabled = video;
            app.mark_dirty();
        }
        let mut show_titles = app.project.output.show_titles;
        if toggle(ui, &theme, &mut show_titles, "Titoli in sovrimpressione").changed() {
            app.project.output.show_titles = show_titles;
            app.mark_dirty();
        }
        nerd_ui_kit::divider(ui, &theme);
        ui.label(
            RichText::new("Schermata di attesa")
                .font(caption())
                .color(theme.colors.text_secondary),
        );
        let mut idle = match app.project.output.idle_screen {
            IdleScreen::Black => 0,
            IdleScreen::Courtesy => 1,
        };
        if segmented(
            ui,
            &theme,
            "idle-screen",
            &["Nero", "Cortesia"],
            &mut idle,
            160.0,
        )
        .changed()
        {
            app.project.output.idle_screen = if idle == 0 {
                IdleScreen::Black
            } else {
                IdleScreen::Courtesy
            };
            app.mark_dirty();
        }
        if app.project.output.idle_screen == IdleScreen::Courtesy {
            let mut message = app.project.output.courtesy_message.clone();
            if text_field(ui, &theme, &mut message, "Prossimo brano").changed() {
                app.project.output.courtesy_message = message;
                app.mark_dirty();
            }
        }
        ui.add_space(4.0);
        let logo_name = app
            .project
            .output
            .logo_path
            .as_ref()
            .and_then(|path| path.file_name())
            .and_then(|name| name.to_str())
            .unwrap_or("Nessun logo")
            .to_string();
        row(ui, &theme, "Logo", &logo_name, |ui| {
            if button(ui, &theme, "Scegli…").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .set_title("Logo uscita video")
                    .add_filter("Immagini", &["png", "jpg", "jpeg"])
                    .pick_file()
                {
                    app.project.output.logo_path = Some(path);
                    app.mark_dirty();
                }
            }
            if app.project.output.logo_path.is_some()
                && icon_button(ui, &theme, Icon::Trash, Some("Rimuovi logo"), false).clicked()
            {
                app.project.output.logo_path = None;
                app.mark_dirty();
            }
        });
        ui.add_space(4.0);
        ui.horizontal(|ui| {
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
            if Button::new(if app.project.output.blackout {
                "Fine blackout"
            } else {
                "Blackout"
            })
            .variant(ButtonVariant::Destructive)
            .icon(Icon::Blackout)
            .show(ui, &theme)
            .clicked()
            {
                app.toggle_blackout();
            }
        });
    });
}

fn background(app: &mut NerdApp, ui: &mut Ui) {
    let theme = app.palette;
    let tracks = app.project.background.tracks.clone();
    group(ui, &theme, "Sottofondo", |ui| {
        if tracks.is_empty() {
            ui.label(
                RichText::new("Nessun brano. Il sottofondo accompagna pause e uscita.")
                    .font(caption())
                    .color(theme.colors.text_secondary),
            );
        } else {
            ScrollArea::vertical()
                .id_salt("settings-bg-tracks")
                .max_height(110.0)
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    for (index, path) in tracks.iter().enumerate() {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(
                                    path.file_name()
                                        .and_then(|name| name.to_str())
                                        .unwrap_or("Brano"),
                                )
                                .size(12.0)
                                .color(theme.colors.text),
                            );
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if icon_button(ui, &theme, Icon::Trash, Some("Rimuovi"), false)
                                    .clicked()
                                {
                                    let before = app.project.clone();
                                    app.project.background.tracks.remove(index);
                                    app.commit_edit(&before);
                                    app.reload_background();
                                }
                            });
                        });
                    }
                });
        }
        if Button::new("Aggiungi brani…")
            .variant(ButtonVariant::Secondary)
            .icon(Icon::Plus)
            .show(ui, &theme)
            .clicked()
        {
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
        nerd_ui_kit::divider(ui, &theme);
        let mut volume = app.project.background.volume;
        row(ui, &theme, "Volume sottofondo", "", |ui| {
            if slider(ui, &theme, &mut volume, 0.0..=1.5, "", 180.0).changed() {
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
        let mut looping = app.project.background.looping;
        if toggle(ui, &theme, &mut looping, "Ripeti la playlist").changed() {
            app.project.background.looping = looping;
            app.mark_dirty();
        }
        let mut shuffle = app.project.background.shuffle;
        if toggle(ui, &theme, &mut shuffle, "Ordine casuale").changed() {
            app.project.background.shuffle = shuffle;
            app.mark_dirty();
        }
        let mut fade_in = app.project.background.fade_in_ms as f32 / 1000.0;
        row(ui, &theme, "Fade in", "", |ui| {
            if slider(ui, &theme, &mut fade_in, 0.0..=10.0, " s", 160.0).changed() {
                app.project.background.fade_in_ms = (fade_in * 1000.0) as u64;
                app.sync_show_config();
                app.mark_dirty();
            }
        });
        let mut fade_out = app.project.background.fade_out_ms as f32 / 1000.0;
        row(ui, &theme, "Fade out", "", |ui| {
            if slider(ui, &theme, &mut fade_out, 0.0..=10.0, " s", 160.0).changed() {
                app.project.background.fade_out_ms = (fade_out * 1000.0) as u64;
                app.sync_show_config();
                app.mark_dirty();
            }
        });
        let mut resume = match app.project.background.resume_mode {
            ResumeMode::Resume => 0,
            ResumeMode::Restart => 1,
        };
        ui.label(
            RichText::new("Ripresa")
                .font(caption())
                .color(theme.colors.text_secondary),
        );
        if segmented(
            ui,
            &theme,
            "resume-mode",
            &["Da dove era", "Ricomincia"],
            &mut resume,
            220.0,
        )
        .changed()
        {
            app.project.background.resume_mode = if resume == 0 {
                ResumeMode::Resume
            } else {
                ResumeMode::Restart
            };
            app.mark_dirty();
        }
    });
}

fn automation(app: &mut NerdApp, ui: &mut Ui) {
    let theme = app.palette;
    group(ui, &theme, "Automazione", |ui| {
        let mut mode = match app.project.show.mode {
            PlaybackMode::Manual => 0,
            PlaybackMode::Automatic => 1,
        };
        ui.label(
            RichText::new("Modalità predefinita")
                .font(caption())
                .color(theme.colors.text_secondary),
        );
        if segmented(
            ui,
            &theme,
            "default-mode",
            &["Manuale", "Automatico"],
            &mut mode,
            220.0,
        )
        .changed()
        {
            app.set_show_mode(if mode == 0 {
                PlaybackMode::Manual
            } else {
                PlaybackMode::Automatic
            });
        }
        nerd_ui_kit::divider(ui, &theme);
        let mut gap = app.project.show.default_gap_ms as f64 / 1000.0;
        nerd_ui_kit::number_field(
            ui,
            &theme,
            "Pausa tra i brani",
            &mut gap,
            0.0..=600.0,
            " s",
            0.5,
        );
        app.project.show.default_gap_ms = (gap * 1000.0) as u64;
        if ui.ctx().input(|input| input.pointer.any_released()) {
            app.sync_show_config();
            app.mark_dirty();
        }
        let mut panic = app.project.show.panic_fade_ms as f64;
        nerd_ui_kit::number_field(
            ui,
            &theme,
            "Fade Panic",
            &mut panic,
            50.0..=2_000.0,
            " ms",
            10.0,
        );
        app.project.show.panic_fade_ms = panic as u64;
        let mut confirm = app.prefs.confirm_quit_during_show;
        if toggle(
            ui,
            &theme,
            &mut confirm,
            "Chiedi conferma prima di chiudere durante lo show",
        )
        .changed()
        {
            app.prefs.confirm_quit_during_show = confirm;
            app.prefs_dirty = true;
        }
    });
}

fn titles(app: &mut NerdApp, ui: &mut Ui) {
    let theme = app.palette;
    group(ui, &theme, "Titoli", |ui| {
        let mut enabled = app.project.output.show_titles;
        if toggle(
            ui,
            &theme,
            &mut enabled,
            "Mostra i titoli durante lo spettacolo",
        )
        .changed()
        {
            app.project.output.show_titles = enabled;
            app.mark_dirty();
        }
        row(
            ui,
            &theme,
            "Template attivo",
            "Posizione, colori e animazione",
            |ui| {
                if Button::new("Modifica in Titoli")
                    .variant(ButtonVariant::Secondary)
                    .icon(Icon::Titles)
                    .show(ui, &theme)
                    .clicked()
                {
                    app.page = Page::Titles;
                }
            },
        );
        ui.label(
            RichText::new(format!(
                "{} · {}",
                app.project.title_template.name,
                strings::format_timecode(app.project.title_template.duration_ms)
            ))
            .font(caption())
            .color(theme.colors.text_secondary),
        );
    });
}

fn appearance(app: &mut NerdApp, ui: &mut Ui) {
    let theme = app.palette;
    group(ui, &theme, "Aspetto", |ui| {
        let mut mode = match app.prefs.theme {
            ThemeMode::Dark => 0,
            ThemeMode::Light => 1,
            ThemeMode::System => 2,
        };
        ui.label(
            RichText::new("Tema")
                .font(caption())
                .color(theme.colors.text_secondary),
        );
        if segmented(
            ui,
            &theme,
            "theme-mode",
            &["Scuro", "Chiaro", "Sistema"],
            &mut mode,
            260.0,
        )
        .changed()
        {
            app.prefs.theme = match mode {
                1 => ThemeMode::Light,
                2 => ThemeMode::System,
                _ => ThemeMode::Dark,
            };
            app.palette = theme::install_with_system(ui.ctx(), app.prefs.theme, app.system);
            app.prefs_dirty = true;
        }
        ui.label(
            RichText::new("Auto segue l'aspetto di macOS; l'accento è il colore di sistema.")
                .font(caption())
                .color(theme.colors.text_secondary),
        );
        ui.add_space(4.0);
        ui.label(
            RichText::new(format!(
                "Riduci movimento: {} · Aumenta contrasto: {}",
                if app.system.reduce_motion {
                    "attivo"
                } else {
                    "spento"
                },
                if app.system.increase_contrast {
                    "attivo"
                } else {
                    "spento"
                }
            ))
            .font(caption())
            .color(theme.colors.text_tertiary),
        );
    });
}

fn updates(app: &mut NerdApp, ui: &mut Ui) {
    let theme = app.palette;
    group(ui, &theme, "Aggiornamenti", |ui| {
        row(ui, &theme, "Versione", "", |ui| {
            ui.label(
                RichText::new(nerd_core::APP_VERSION)
                    .font(nerd_ui_kit::timecode(11.0))
                    .color(theme.colors.text_secondary),
            );
        });
        let mut channel = match app.prefs.update_channel {
            UpdateChannel::Stable => 0,
            UpdateChannel::Alpha => 1,
        };
        ui.label(
            RichText::new("Canale")
                .font(caption())
                .color(theme.colors.text_secondary),
        );
        if segmented(
            ui,
            &theme,
            "update-channel",
            &["Stabile", "Alpha"],
            &mut channel,
            180.0,
        )
        .changed()
        {
            app.prefs.update_channel = if channel == 0 {
                UpdateChannel::Stable
            } else {
                UpdateChannel::Alpha
            };
            app.prefs_dirty = true;
        }
        let mut startup = app.prefs.check_updates_on_startup;
        if toggle(ui, &theme, &mut startup, "Controlla all'avvio").changed() {
            app.prefs.check_updates_on_startup = startup;
            app.prefs_dirty = true;
        }
        let mut repo = app.prefs.update_repo.clone();
        if labeled_field(ui, &theme, "Repository GitHub", &mut repo, "owner/repo").changed() {
            app.prefs.update_repo = repo;
            app.prefs_dirty = true;
        }
        let color = match &app.updater.status {
            UpdateStatus::Error(_) => theme.colors.red,
            UpdateStatus::Available(_) | UpdateStatus::Ready(_) | UpdateStatus::UpToDate => {
                theme.colors.green
            }
            UpdateStatus::Checking
            | UpdateStatus::Downloading { .. }
            | UpdateStatus::Installing => theme.colors.amber,
            UpdateStatus::Idle => theme.colors.text_secondary,
        };
        ui.label(
            RichText::new(app.updater.status.description())
                .font(caption())
                .color(color),
        );
        ui.horizontal(|ui| {
            if Button::new("Cerca aggiornamenti")
                .variant(ButtonVariant::Secondary)
                .icon(Icon::Refresh)
                .enabled(!app.updater.status.is_busy())
                .show(ui, &theme)
                .clicked()
            {
                app.check_updates(true);
            }
            if let UpdateStatus::Available(release) = &app.updater.status {
                if Button::new("Scarica")
                    .variant(ButtonVariant::Tinted)
                    .icon(Icon::Download)
                    .show(ui, &theme)
                    .clicked()
                {
                    let repo = app.prefs.update_repo.clone();
                    let release = release.clone();
                    app.updater.download(&repo, release);
                }
            }
            if matches!(app.updater.status, UpdateStatus::Ready(_))
                && primary_button(ui, &theme, "Installa e riavvia").clicked()
            {
                app.install_update(ui.ctx());
            }
        });
    });
}

fn project(app: &mut NerdApp, ui: &mut Ui) {
    let theme = app.palette;
    group(ui, &theme, "Serata", |ui| {
        let before = app.project.clone();
        let mut name = app.project.name.clone();
        let mut changed = labeled_field(ui, &theme, "Nome", &mut name, "Nome serata").changed();
        app.project.name = name;
        let mut venue = app.project.venue.clone();
        changed |= labeled_field(ui, &theme, "Luogo", &mut venue, "Auditorium").changed();
        app.project.venue = venue;
        let mut date = app.project.date.clone();
        changed |= labeled_field(ui, &theme, "Data", &mut date, "AAAA-MM-GG").changed();
        app.project.date = date;
        let mut notes = app.project.notes.clone();
        if text_field(ui, &theme, &mut notes, "Note per la regia").changed() {
            app.project.notes = notes;
            changed = true;
        }
        if changed {
            app.commit_edit(&before);
        }
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            if button(ui, &theme, "Salva").clicked() {
                app.save(false);
            }
            if button(ui, &theme, "Salva con nome…").clicked() {
                app.save(true);
            }
        });
    });
}

fn data(app: &mut NerdApp, ui: &mut Ui) {
    let theme = app.palette;
    group(ui, &theme, "Dati e diagnostica", |ui| {
        ui.horizontal_wrapped(|ui| {
            for (label, path) in [
                ("Cartella dati", app.paths.support.clone()),
                ("Log", app.paths.logs.clone()),
                ("Cache", app.paths.cache.clone()),
            ] {
                if Button::new(label)
                    .variant(ButtonVariant::Plain)
                    .icon(Icon::Folder)
                    .show(ui, &theme)
                    .clicked()
                {
                    app.open_folder(&path);
                }
            }
            if Button::new("Scorciatoie")
                .variant(ButtonVariant::Plain)
                .icon(Icon::Keyboard)
                .show(ui, &theme)
                .clicked()
            {
                app.show_shortcuts = true;
            }
        });
        ui.add_space(4.0);
        ui.label(
            RichText::new(format!(
                "schema {} · macOS {}+ · {} brani · {} media",
                nerd_core::CURRENT_SCHEMA_VERSION,
                nerd_core::branding::MIN_MACOS_VERSION,
                app.project.songs.len(),
                app.library.assets.len()
            ))
            .font(caption())
            .color(theme.colors.text_tertiary),
        );
        if let Some(error) = &app.menu.error {
            ui.label(
                RichText::new(format!("Menu nativo non disponibile: {error}"))
                    .font(caption())
                    .color(theme.colors.red),
            );
        }
    });
}

fn shortcuts(ui: &mut Ui, theme: &Theme) {
    group(ui, theme, "Scorciatoie", |ui| {
        ui.horizontal_wrapped(|ui| {
            for (key, action) in [
                ("Spazio", "GO"),
                ("P", "Pausa"),
                ("S", "Stop"),
                ("N", "Salta"),
                ("B", "Indietro"),
                ("⇧⌘P", "Panic"),
                ("⌘B", "Blackout"),
                ("⌘1…5", "Pagine"),
                ("⌘,", "Impostazioni"),
            ] {
                keycap(ui, theme, key);
                ui.label(
                    RichText::new(action)
                        .font(caption())
                        .color(theme.colors.text_secondary),
                );
                ui.add_space(6.0);
            }
        });
    });
}
