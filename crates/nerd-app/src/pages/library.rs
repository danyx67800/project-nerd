//! Library: media browser, real viewer and collapsible inspector.

use eframe::egui::{self, Align, Layout, Pos2, Rect, RichText, ScrollArea, Sense, Ui, Vec2};
use nerd_core::library::{MediaAsset, MediaKind};

use crate::app::{LibrarySort, NerdApp};
use crate::state::ToastKind;
use crate::strings;
use nerd_ui_kit::{
    body, caption, card, empty_state, labeled_field, menu_button, menu_item, metadata_row,
    paint_texture_fit, primary_button, screen_placeholder, section, section_label, segmented,
    status_pill, Button, ButtonVariant, Icon, ListRow, Theme,
};

pub fn show(app: &mut NerdApp, ui: &mut Ui) {
    if app.chrome.library_sidebar {
        egui::Panel::left("library-browser")
            .resizable(true)
            .default_size(280.0)
            .min_size(220.0)
            .max_size(420.0)
            .frame(
                egui::Frame::new()
                    .fill(app.palette.colors.panel)
                    .inner_margin(egui::Margin::symmetric(14, 12)),
            )
            .show(ui, |ui| sidebar(app, ui));
    }
    if app.chrome.inspector_open {
        egui::Panel::right("library-inspector")
            .resizable(true)
            .default_size(302.0)
            .min_size(260.0)
            .max_size(430.0)
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
        .show(ui, |ui| viewer(app, ui));
}

fn sidebar(app: &mut NerdApp, ui: &mut Ui) {
    let theme = app.palette;
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("Libreria")
                .size(15.0)
                .strong()
                .color(theme.colors.text),
        );
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let response = menu_button(
                ui,
                &theme,
                "library-add",
                "",
                Some(Icon::Plus),
                ButtonVariant::Plain,
                28.0,
                |ui| {
                    if menu_item(ui, &theme, "Importa file…", Some(Icon::File)).clicked() {
                        app.import_dialog();
                    }
                    if menu_item(ui, &theme, "Importa cartella…", Some(Icon::Folder)).clicked() {
                        if let Some(folder) = rfd::FileDialog::new()
                            .set_title("Importa cartella")
                            .pick_folder()
                        {
                            let mut paths = Vec::new();
                            collect_files(&folder, &mut paths);
                            app.import_paths(paths);
                        }
                    }
                },
            );
            response.on_hover_text("Importa file o cartella");
        });
    });
    ui.label(
        RichText::new(format!(
            "{} file · {} in scaletta",
            app.library.assets.len(),
            app.project.songs.len()
        ))
        .font(caption())
        .color(theme.colors.text_secondary),
    );
    ui.add_space(10.0);
    if nerd_ui_kit::search_field(ui, &theme, &mut app.search, "Cerca titolo o artista").clicked() {
        // Focus is handled by the field itself.
    }
    ui.add_space(7.0);
    ui.horizontal(|ui| {
        let mut filter = app.chrome.library_filter.min(2);
        if segmented(
            ui,
            &theme,
            "library-filter",
            &["Tutti", "Audio", "Video"],
            &mut filter,
            156.0,
        )
        .changed()
        {
            app.chrome.library_filter = filter;
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            menu_button(
                ui,
                &theme,
                "library-sort",
                "",
                Some(Icon::Sort),
                ButtonVariant::Plain,
                28.0,
                |ui| {
                    for (sort, label) in [
                        (LibrarySort::Title, "Titolo"),
                        (LibrarySort::Duration, "Durata"),
                        (LibrarySort::Recent, "Recenti"),
                    ] {
                        if menu_item(
                            ui,
                            &theme,
                            label,
                            if app.sort == sort {
                                Some(Icon::Check)
                            } else {
                                None
                            },
                        )
                        .clicked()
                        {
                            app.sort = sort;
                        }
                    }
                },
            )
            .on_hover_text("Ordina la libreria");
        });
    });
    if app.imports_in_flight > 0 {
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label(
                RichText::new(format!(
                    "Analisi di {} file in corso…",
                    app.imports_in_flight
                ))
                .font(caption())
                .color(theme.colors.text_secondary),
            );
        });
    }
    ui.add_space(8.0);

    let query = app.search.trim().to_lowercase();
    let filter = app.chrome.library_filter;
    let mut assets: Vec<MediaAsset> = app
        .library
        .assets
        .iter()
        .filter(|asset| {
            (filter == 0
                || (filter == 1 && asset.kind == MediaKind::Audio)
                || (filter == 2 && asset.kind == MediaKind::Video))
                && (query.is_empty()
                    || asset.display_title().to_lowercase().contains(&query)
                    || asset.artist.to_lowercase().contains(&query))
        })
        .cloned()
        .collect();
    match app.sort {
        LibrarySort::Title => assets.sort_by(|a, b| {
            a.display_title()
                .to_lowercase()
                .cmp(&b.display_title().to_lowercase())
        }),
        LibrarySort::Duration => assets.sort_by_key(|asset| asset.duration_ms),
        LibrarySort::Recent => assets.sort_by_key(|asset| std::cmp::Reverse(asset.added_at_unix)),
    }

    if assets.is_empty() {
        if app.library.assets.is_empty() {
            empty_state(
                ui,
                &theme,
                Icon::Music,
                "Nessun media",
                "Trascina qui i file della serata, oppure importali dal Finder.",
                |ui| {
                    if primary_button(ui, &theme, "Importa…").clicked() {
                        app.import_dialog();
                    }
                },
            );
        } else {
            empty_state(
                ui,
                &theme,
                Icon::Search,
                "Nessun risultato",
                "Prova con un altro titolo, artista o filtro.",
                |_ui| {},
            );
        }
        return;
    }

    ScrollArea::vertical()
        .id_salt("library-assets")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for asset in assets {
                let selected = app.selected_asset.as_deref() == Some(asset.id.as_str());
                let used = app.project.songs.iter().any(|song| {
                    song.audio.as_ref() == Some(&asset.path)
                        || song.video.as_ref() == Some(&asset.path)
                });
                let missing = !asset.exists();
                let thumbnail = app.thumbnail_for(ui.ctx(), &asset);
                let subtitle = if asset.artist.is_empty() {
                    format!(
                        "{} · {}",
                        kind_label(asset.kind),
                        duration(asset.duration_ms)
                    )
                } else {
                    format!("{} · {}", asset.artist, duration(asset.duration_ms))
                };
                let response = ListRow {
                    title: asset.display_title(),
                    subtitle: &subtitle,
                    duration: &duration(asset.duration_ms),
                    icon: kind_icon(asset.kind),
                    accent: if asset.kind == MediaKind::Video {
                        theme.colors.video
                    } else {
                        theme.colors.accent
                    },
                    selected,
                    badge: used.then_some("In scaletta"),
                    waveform: (asset.kind == MediaKind::Audio).then_some(asset.waveform.as_slice()),
                    thumbnail: thumbnail.as_ref(),
                    trailing_icon: None,
                    missing,
                }
                .show(ui, &theme);
                if response.clicked() {
                    app.selected_asset = Some(asset.id.clone());
                }
                if response.double_clicked() {
                    app.selected_asset = Some(asset.id.clone());
                    app.add_asset_to_show(&asset.id);
                }
                let id = asset.id.clone();
                let path = asset.path.clone();
                let title = asset.display_title().to_string();
                response.context_menu(|ui| {
                    if menu_item(ui, &theme, "Aggiungi alla scaletta", Some(Icon::Plus)).clicked() {
                        app.selected_asset = Some(id.clone());
                        app.add_asset_to_show(&id);
                        ui.close();
                    }
                    if menu_item(ui, &theme, "Rivela nel Finder", Some(Icon::Folder)).clicked() {
                        app.reveal_path(&path);
                        ui.close();
                    }
                    nerd_ui_kit::divider(ui, &theme);
                    if menu_item(ui, &theme, "Rimuovi dalla libreria", Some(Icon::Trash)).clicked()
                    {
                        app.library.remove(&id);
                        if app.selected_asset.as_deref() == Some(id.as_str()) {
                            app.selected_asset = app.library.assets.first().map(|a| a.id.clone());
                        }
                        app.save_library();
                        app.toast(format!("Rimosso: {title}"), ToastKind::Info);
                        ui.close();
                    }
                });
            }
            ui.add_space(12.0);
        });
}

fn viewer(app: &mut NerdApp, ui: &mut Ui) {
    let theme = app.palette;
    let Some(asset) = app
        .selected_asset
        .as_ref()
        .and_then(|id| app.library.find(id))
        .cloned()
    else {
        nerd_ui_kit::underlay(ui, &theme, |ui| {
            ui.set_min_height(ui.available_height().max(240.0));
            empty_state(
                ui,
                &theme,
                Icon::Library,
                "Seleziona un media",
                "Il viewer mostra qui video, onda e metadati. Trascina i file per importarli.",
                |ui| {
                    if primary_button(ui, &theme, "Importa…").clicked() {
                        app.import_dialog();
                    }
                },
            );
        });
        return;
    };

    let used = app.project.songs.iter().any(|song| {
        song.audio.as_ref() == Some(&asset.path) || song.video.as_ref() == Some(&asset.path)
    });
    ui.label(
        RichText::new(asset.display_title())
            .size(22.0)
            .strong()
            .color(theme.colors.text),
    );
    ui.horizontal_wrapped(|ui| {
        ui.label(
            RichText::new(if asset.artist.is_empty() {
                "Artista non indicato"
            } else {
                &asset.artist
            })
            .font(body())
            .color(theme.colors.text_secondary),
        );
        status_pill(
            ui,
            &theme,
            kind_label(asset.kind),
            if asset.kind == MediaKind::Video {
                theme.colors.video
            } else {
                theme.colors.accent
            },
            Some(kind_icon(asset.kind)),
        );
        if !asset.exists() {
            status_pill(
                ui,
                &theme,
                "File mancante",
                theme.colors.red,
                Some(Icon::Warning),
            );
        }
    });
    ui.add_space(7.0);
    ui.horizontal_wrapped(|ui| {
        if Button::new(if used {
            "Aggiungi di nuovo"
        } else {
            "Aggiungi alla scaletta"
        })
        .variant(ButtonVariant::Primary)
        .icon(Icon::Plus)
        .show(ui, &theme)
        .clicked()
        {
            app.add_asset_to_show(&asset.id);
        }
        if nerd_ui_kit::Button::new("Rivela nel Finder")
            .variant(ButtonVariant::Secondary)
            .icon(Icon::Folder)
            .show(ui, &theme)
            .clicked()
        {
            app.reveal_path(&asset.path);
        }
    });
    ui.add_space(10.0);

    let texture = app.thumbnail_for(ui.ctx(), &asset);
    let available = ui.available_size();
    let height = (available.y * 0.48).clamp(190.0, 380.0);
    let (rect, response) = ui.allocate_exact_size(Vec2::new(available.x, height), Sense::hover());
    let canvas = nerd_ui_kit::fit_rect(rect, 16.0 / 9.0);
    ui.painter()
        .rect_filled(canvas, egui::CornerRadius::same(10), theme.colors.blackout);
    if let Some(texture) = &texture {
        paint_texture_fit(ui.painter(), texture, canvas);
    } else if asset.kind == MediaKind::Video {
        screen_placeholder(
            ui.painter(),
            canvas.shrink(18.0),
            &theme,
            Icon::Video,
            "Anteprima video non disponibile",
        );
    } else {
        paint_audio_viewer(ui, canvas, &asset, &theme);
    }
    let _ = response;
    ui.add_space(10.0);

    ui.horizontal(|ui| {
        ui.label(
            RichText::new(format!(
                "{}  /  {}",
                strings::format_timecode(0),
                strings::format_timecode(asset.duration_ms)
            ))
            .font(nerd_ui_kit::timecode(11.0))
            .color(theme.colors.text_secondary),
        );
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(
                RichText::new(if asset.kind == MediaKind::Video {
                    "Video musicale"
                } else {
                    "Traccia audio"
                })
                .font(caption())
                .color(theme.colors.text_tertiary),
            );
        });
    });
    ui.add_space(12.0);

    card(ui, &theme, |ui| {
        ui.horizontal(|ui| {
            section_label(ui, &theme, "Forma d'onda");
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(
                    RichText::new(format!(
                        "{} · {} · {}",
                        file_format(&asset),
                        strings::format_timecode(asset.duration_ms),
                        size_label(asset.file_size)
                    ))
                    .font(caption())
                    .color(theme.colors.text_secondary),
                );
            });
        });
        let (wave_rect, _) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), 84.0), Sense::hover());
        ui.painter()
            .rect_filled(wave_rect, egui::CornerRadius::same(6), theme.colors.well);
        nerd_ui_kit::waveform(
            ui.painter(),
            wave_rect.shrink2(Vec2::new(14.0, 14.0)),
            &asset.waveform,
            if asset.kind == MediaKind::Video {
                theme.colors.video
            } else {
                theme.colors.accent
            },
            None,
        );
    });
}

fn paint_audio_viewer(ui: &mut Ui, rect: Rect, asset: &MediaAsset, theme: &Theme) {
    let painter = ui.painter().with_clip_rect(rect);
    painter.rect_filled(rect, egui::CornerRadius::same(10), theme.colors.well);
    let label_rect = Rect::from_min_size(
        Pos2::new(rect.left() + 22.0, rect.top() + 18.0),
        Vec2::new(rect.width() - 44.0, 44.0),
    );
    painter.text(
        label_rect.left_top(),
        egui::Align2::LEFT_TOP,
        "FORMA D'ONDA",
        caption(),
        theme.colors.text_tertiary,
    );
    painter.text(
        Pos2::new(label_rect.left(), label_rect.top() + 20.0),
        egui::Align2::LEFT_TOP,
        asset.display_title(),
        nerd_ui_kit::section_title(),
        theme.colors.text,
    );
    let wave = Rect::from_center_size(
        Pos2::new(rect.center().x, rect.center().y + 22.0),
        Vec2::new(rect.width() * 0.78, rect.height() * 0.36),
    );
    nerd_ui_kit::waveform(
        &painter,
        wave,
        &asset.waveform,
        theme.colors.accent,
        Some(0.0),
    );
    painter.line_segment(
        [
            Pos2::new(wave.left(), wave.top() - 4.0),
            Pos2::new(wave.left(), wave.bottom() + 4.0),
        ],
        egui::Stroke::new(1.0, theme.colors.text_secondary),
    );
}

fn inspector(app: &mut NerdApp, ui: &mut Ui) {
    let theme = app.palette;
    ui.painter()
        .rect_filled(ui.max_rect(), egui::CornerRadius::ZERO, theme.colors.panel);
    if !app.chrome.inspector_open {
        return;
    }
    let Some(asset) = app
        .selected_asset
        .as_ref()
        .and_then(|id| app.library.find(id))
        .cloned()
    else {
        empty_state(
            ui,
            &theme,
            Icon::Info,
            "Inspector",
            "Seleziona un file per vedere e modificare i suoi dati.",
            |_ui| {},
        );
        return;
    };

    ScrollArea::vertical()
        .id_salt("library-inspector-scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.add_space(14.0);
            ui.horizontal(|ui| {
                ui.add_space(16.0);
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new("Inspector")
                            .size(15.0)
                            .strong()
                            .color(theme.colors.text),
                    );
                    ui.label(
                        RichText::new(format!(
                            "{} · {}",
                            kind_label(asset.kind),
                            file_format(&asset)
                        ))
                        .font(caption())
                        .color(theme.colors.text_secondary),
                    );
                });
            });
            ui.add_space(10.0);
            nerd_ui_kit::divider(ui, &theme);
            ui.add_space(6.0);

            let id = "library-info";
            let mut title = asset.title.clone();
            let mut artist = asset.artist.clone();
            let mut album = asset.album.clone();
            let mut changed = false;
            section(ui, &theme, id, "Info", true, |ui| {
                ui.horizontal(|ui| {
                    ui.add_space(16.0);
                    ui.vertical(|ui| {
                        ui.set_width((ui.available_width() - 16.0).max(120.0));
                        labeled_field(ui, &theme, "Titolo", &mut title, "Titolo");
                        ui.add_space(6.0);
                        labeled_field(ui, &theme, "Artista", &mut artist, "Artista");
                        ui.add_space(6.0);
                        labeled_field(ui, &theme, "Album / progetto", &mut album, "Album");
                        changed =
                            title != asset.title || artist != asset.artist || album != asset.album;
                    });
                });
            });
            if changed {
                if let Some(item) = app.library.assets.iter_mut().find(|a| a.id == asset.id) {
                    item.title = title;
                    item.artist = artist;
                    item.album = album;
                }
                app.save_library();
            }

            section(ui, &theme, "library-file", "File", true, |ui| {
                ui.horizontal(|ui| {
                    ui.add_space(16.0);
                    ui.vertical(|ui| {
                        ui.set_width((ui.available_width() - 16.0).max(120.0));
                        metadata_row(ui, &theme, "Percorso", &asset.path.to_string_lossy());
                        metadata_row(
                            ui,
                            &theme,
                            "Durata",
                            &strings::format_timecode(asset.duration_ms),
                        );
                        metadata_row(ui, &theme, "Dimensione", &size_label(asset.file_size));
                        metadata_row(ui, &theme, "Formato", &file_format(&asset));
                        if asset.width > 0 {
                            metadata_row(
                                ui,
                                &theme,
                                "Risoluzione",
                                &format!("{} × {}", asset.width, asset.height),
                            );
                        }
                        if !asset.exists() {
                            ui.add_space(5.0);
                            ui.label(
                                RichText::new("Il file non è più al suo percorso.")
                                    .font(caption())
                                    .color(theme.colors.red),
                            );
                            if Button::new("Ricollega…")
                                .variant(ButtonVariant::Secondary)
                                .show(ui, &theme)
                                .clicked()
                            {
                                app.relink_dialog();
                            }
                        }
                    });
                });
            });

            section(ui, &theme, "library-audio", "Audio", true, |ui| {
                ui.horizontal(|ui| {
                    ui.add_space(16.0);
                    ui.vertical(|ui| {
                        ui.set_width((ui.available_width() - 16.0).max(120.0));
                        let (rect, _) = ui.allocate_exact_size(
                            Vec2::new(ui.available_width(), 54.0),
                            Sense::hover(),
                        );
                        ui.painter().rect_filled(
                            rect,
                            egui::CornerRadius::same(5),
                            theme.colors.well,
                        );
                        nerd_ui_kit::waveform(
                            ui.painter(),
                            rect.shrink2(Vec2::new(8.0, 10.0)),
                            &asset.waveform,
                            if asset.kind == MediaKind::Video {
                                theme.colors.video
                            } else {
                                theme.colors.accent
                            },
                            None,
                        );
                        ui.add_space(6.0);
                        metadata_row(ui, &theme, "Canali", "Stereo");
                        metadata_row(ui, &theme, "Campioni", "44,1 kHz");
                    });
                });
            });

            section(ui, &theme, "library-times", "Tempi", true, |ui| {
                ui.horizontal(|ui| {
                    ui.add_space(16.0);
                    ui.vertical(|ui| {
                        ui.set_width((ui.available_width() - 16.0).max(120.0));
                        metadata_row(ui, &theme, "Inizio", &strings::format_timecode(0));
                        metadata_row(
                            ui,
                            &theme,
                            "Fine",
                            &strings::format_timecode(asset.duration_ms),
                        );
                        metadata_row(ui, &theme, "Durata", &duration(asset.duration_ms));
                    });
                });
            });

            ui.add_space(12.0);
            ui.horizontal(|ui| {
                ui.add_space(16.0);
                if Button::new("Rimuovi dalla libreria")
                    .variant(ButtonVariant::Destructive)
                    .icon(Icon::Trash)
                    .show(ui, &theme)
                    .clicked()
                {
                    let title = asset.display_title().to_string();
                    app.library.remove(&asset.id);
                    app.selected_asset = app.library.assets.first().map(|a| a.id.clone());
                    app.save_library();
                    app.toast(format!("Rimosso: {title}"), ToastKind::Info);
                }
            });
            ui.add_space(16.0);
        });
}

fn kind_label(kind: MediaKind) -> &'static str {
    match kind {
        MediaKind::Audio => "Audio",
        MediaKind::Video => "Video",
    }
}

fn kind_icon(kind: MediaKind) -> Icon {
    match kind {
        MediaKind::Audio => Icon::Music,
        MediaKind::Video => Icon::Video,
    }
}

fn duration(ms: u64) -> String {
    strings::format_timecode(ms)
}

fn file_format(asset: &MediaAsset) -> String {
    asset
        .path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("—")
        .to_uppercase()
}

fn size_label(bytes: u64) -> String {
    if bytes > 1_000_000_000 {
        format!("{:.1} GB", bytes as f64 / 1_000_000_000.0)
    } else if bytes > 1_000_000 {
        format!("{:.1} MB", bytes as f64 / 1_000_000.0)
    } else {
        format!("{:.0} KB", bytes as f64 / 1_000.0)
    }
}

fn collect_files(folder: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    let Ok(entries) = std::fs::read_dir(folder) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_files(&path, out);
        } else if nerd_core::library::media_kind_for_path(&path).is_some() {
            out.push(path);
        }
    }
}
