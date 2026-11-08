//! Media pool: browser on the left, viewer in the middle, inspector on the right.

use eframe::egui::{
    self, Align, Align2, Color32, CornerRadius, FontId, Layout, Pos2, Rect, RichText, ScrollArea,
    Sense, Stroke, TextureHandle, Ui, Vec2,
};
use nerd_core::library::{MediaAsset, MediaKind};

use crate::app::{LibrarySort, NerdApp};
use crate::widgets;

pub fn show(app: &mut NerdApp, ui: &mut Ui) {
    egui::Panel::left("library-browser")
        .resizable(true)
        .default_size(338.0)
        .min_size(270.0)
        .max_size(460.0)
        .frame(
            egui::Frame::new()
                .fill(app.palette.panel)
                .inner_margin(egui::Margin::symmetric(14, 12))
                .stroke(Stroke::new(1.0, app.palette.border)),
        )
        .show(ui, |ui| browser(app, ui));

    egui::Panel::right("library-inspector")
        .resizable(true)
        .default_size(310.0)
        .min_size(260.0)
        .max_size(420.0)
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
                .inner_margin(egui::Margin::same(20)),
        )
        .show(ui, |ui| viewer(app, ui));
}

fn browser(app: &mut NerdApp, ui: &mut Ui) {
    let palette = app.palette;
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(crate::strings::PAGE_LIBRARY)
                .size(22.0)
                .strong()
                .color(palette.text),
        );
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(
                RichText::new(format!("{} media", app.library.assets.len()))
                    .size(11.0)
                    .color(palette.text_muted),
            );
        });
    });
    ui.add_space(4.0);
    if crate::widgets::accent_button(ui, palette, "Importa file…")
        .on_hover_text("Audio: MP3, M4A, WAV, AIFF, FLAC · Video: MP4, MOV, M4V")
        .clicked()
    {
        app.import_dialog();
    }
    if app.imports_in_flight > 0 {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label(
                RichText::new(format!(
                    "Analisi di {} file in corso…",
                    app.imports_in_flight
                ))
                .size(11.0)
                .color(palette.text_muted),
            );
        });
    } else {
        ui.label(
            RichText::new("Trascina file dal Finder in qualsiasi pagina.")
                .size(10.5)
                .color(palette.text_muted),
        );
    }
    ui.add_space(10.0);
    ui.add(
        egui::TextEdit::singleline(&mut app.search)
            .hint_text("Cerca titolo o artista")
            .desired_width(f32::INFINITY),
    );
    ui.horizontal(|ui| {
        ui.label(RichText::new("Ordina").size(11.0).color(palette.text_muted));
        egui::ComboBox::from_id_salt("library-sort")
            .selected_text(match app.sort {
                LibrarySort::Title => "Titolo",
                LibrarySort::Duration => "Durata",
                LibrarySort::Recent => "Recenti",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut app.sort, LibrarySort::Title, "Titolo");
                ui.selectable_value(&mut app.sort, LibrarySort::Duration, "Durata");
                ui.selectable_value(&mut app.sort, LibrarySort::Recent, "Recenti");
            });
    });
    ui.add_space(8.0);

    let query = app.search.trim().to_lowercase();
    let mut assets: Vec<MediaAsset> = app
        .library
        .assets
        .iter()
        .filter(|asset| {
            query.is_empty()
                || asset.display_title().to_lowercase().contains(&query)
                || asset.artist.to_lowercase().contains(&query)
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
            widgets::empty_state(
                ui,
                palette,
                "La libreria è vuota",
                "Importa i file audio e video della serata per iniziare.",
            );
        } else {
            widgets::empty_state(
                ui,
                palette,
                "Nessun risultato",
                "Prova con un altro titolo o artista.",
            );
        }
        return;
    }

    ScrollArea::vertical()
        .id_salt("library-assets")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for asset in &assets {
                draw_asset_row(app, ui, asset);
            }
            ui.add_space(10.0);
        });
}

fn draw_asset_row(app: &mut NerdApp, ui: &mut Ui, asset: &MediaAsset) {
    let palette = app.palette;
    let selected = app.selected_asset.as_deref() == Some(asset.id.as_str());
    let texture = app.thumbnail_for(ui.ctx(), asset);
    let width = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 58.0), Sense::click());
    let hovered = response.hovered();
    ui.painter().rect(
        rect,
        CornerRadius::same(9),
        if selected {
            palette.raised
        } else if hovered {
            palette.raised.gamma_multiply(0.55)
        } else {
            Color32::TRANSPARENT
        },
        if selected {
            Stroke::new(1.0, palette.accent.gamma_multiply(0.55))
        } else {
            Stroke::NONE
        },
        egui::StrokeKind::Inside,
    );

    let thumb_rect = Rect::from_min_size(
        Pos2::new(rect.left() + 8.0, rect.center().y - 18.0),
        Vec2::new(64.0, 36.0),
    );
    ui.painter()
        .rect_filled(thumb_rect, CornerRadius::same(5), palette.background);
    if let Some(texture) = &texture {
        paint_texture_fit(ui, texture, thumb_rect);
    } else if asset.kind == MediaKind::Video {
        ui.painter().text(
            thumb_rect.center(),
            Align2::CENTER_CENTER,
            "▶",
            FontId::proportional(17.0),
            palette.video,
        );
    } else {
        crate::widgets::waveform(
            ui.painter(),
            thumb_rect.shrink2(Vec2::new(4.0, 8.0)),
            &asset.waveform,
            palette.accent,
            None,
        );
    }

    let text_x = thumb_rect.right() + 10.0;
    let missing = !asset.exists();
    let title_color = if missing {
        palette.danger
    } else {
        palette.text
    };
    ui.painter().text(
        Pos2::new(text_x, rect.top() + 10.0),
        Align2::LEFT_TOP,
        truncate(asset.display_title(), 30),
        FontId::proportional(12.5),
        title_color,
    );
    ui.painter().text(
        Pos2::new(text_x, rect.top() + 29.0),
        Align2::LEFT_TOP,
        if asset.artist.is_empty() {
            format!(
                "{} · {}",
                if asset.kind == MediaKind::Video {
                    "Video"
                } else {
                    "Audio"
                },
                crate::strings::format_timecode(asset.duration_ms)
            )
        } else {
            asset.artist.clone()
        },
        FontId::proportional(10.5),
        palette.text_muted,
    );
    ui.painter().text(
        Pos2::new(rect.right() - 9.0, rect.top() + 10.0),
        Align2::RIGHT_TOP,
        crate::strings::format_timecode(asset.duration_ms),
        FontId::monospace(10.0),
        palette.text_muted,
    );

    if response.clicked() {
        app.selected_asset = Some(asset.id.clone());
    }
    if response.double_clicked() {
        app.selected_asset = Some(asset.id.clone());
        app.add_asset_to_show(&asset.id);
    }
    let id = asset.id.clone();
    let title = asset.display_title().to_string();
    let path = asset.path.clone();
    response.context_menu(|ui| {
        if ui.button("Aggiungi alla scaletta").clicked() {
            app.selected_asset = Some(id.clone());
            app.add_asset_to_show(&id);
        }
        if ui.button("Rivela nel Finder").clicked() {
            app.reveal_path(&path);
        }
        ui.separator();
        if ui.button("Rimuovi dalla libreria").clicked() {
            app.library.remove(&id);
            if app.selected_asset.as_deref() == Some(id.as_str()) {
                app.selected_asset = app.library.assets.first().map(|a| a.id.clone());
            }
            app.save_library();
            app.toast(
                format!("Rimosso dalla libreria: {title}"),
                crate::state::ToastKind::Info,
            );
        }
    });
}

fn viewer(app: &mut NerdApp, ui: &mut Ui) {
    let palette = app.palette;
    let asset = app
        .selected_asset
        .as_ref()
        .and_then(|id| app.library.find(id))
        .cloned();
    let Some(asset) = asset else {
        widgets::empty_state(
            ui,
            palette,
            "Media pool",
            "Seleziona un file dalla libreria per vederlo e aggiungerlo alla scaletta.",
        );
        return;
    };
    let texture = app.thumbnail_for(ui.ctx(), &asset);
    let missing = !asset.exists();

    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(
                RichText::new(asset.display_title())
                    .size(26.0)
                    .strong()
                    .color(palette.text),
            );
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(if asset.artist.is_empty() {
                        "Artista non indicato".to_string()
                    } else {
                        asset.artist.clone()
                    })
                    .size(13.0)
                    .color(palette.text_muted),
                );
                widgets::pill(
                    ui,
                    if asset.kind == MediaKind::Video {
                        "VIDEO"
                    } else {
                        "AUDIO"
                    },
                    if asset.kind == MediaKind::Video {
                        palette.video
                    } else {
                        palette.accent
                    },
                );
                if missing {
                    widgets::pill(ui, "FILE MANCANTE", palette.danger);
                }
            });
        });
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if crate::widgets::accent_button(ui, palette, "Aggiungi alla scaletta").clicked() {
                app.add_asset_to_show(&asset.id);
            }
            if widgets::ghost_button(ui, palette, "Rivela nel Finder").clicked() {
                app.reveal_path(&asset.path);
            }
        });
    });
    ui.add_space(14.0);

    let available = ui.available_size();
    let height = (available.y * 0.55).clamp(220.0, 440.0);
    let width = (height * 16.0 / 9.0).min(available.x);
    let image_rect = Rect::from_center_size(
        Pos2::new(
            ui.available_rect_before_wrap().center().x,
            ui.cursor().top() + height / 2.0,
        ),
        Vec2::new(width, height),
    );
    let (_, response) = ui.allocate_exact_size(Vec2::new(available.x, height), Sense::hover());
    let painter = ui.painter().with_clip_rect(image_rect);
    painter.rect_filled(image_rect, CornerRadius::same(14), Color32::BLACK);
    if let Some(texture) = &texture {
        paint_texture_fit_painter(&painter, texture, image_rect);
    } else {
        let inner = image_rect.shrink(28.0);
        painter.rect_filled(inner, CornerRadius::same(8), palette.panel_alt);
        widgets::waveform(
            &painter,
            Rect::from_center_size(
                inner.center(),
                Vec2::new(inner.width(), inner.height() * 0.4),
            ),
            &asset.waveform,
            if asset.kind == MediaKind::Video {
                palette.video
            } else {
                palette.accent
            },
            None,
        );
        painter.text(
            Pos2::new(inner.center().x, inner.top() + 18.0),
            Align2::CENTER_TOP,
            if asset.kind == MediaKind::Video {
                "Anteprima video non disponibile"
            } else {
                "Forma d'onda"
            },
            FontId::proportional(14.0),
            palette.text_muted,
        );
    }

    ui.add_space(12.0);
    egui::Frame::new()
        .fill(palette.panel)
        .stroke(Stroke::new(1.0, palette.border))
        .corner_radius(CornerRadius::same(12))
        .inner_margin(egui::Margin::same(16))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                widgets::section_label(ui, palette, "Forma d'onda");
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(
                        RichText::new(format!(
                            "{} · {}",
                            crate::strings::format_timecode(asset.duration_ms),
                            size_label(asset.file_size)
                        ))
                        .size(10.5)
                        .color(palette.text_muted),
                    );
                });
            });
            let (rect, _) =
                ui.allocate_exact_size(Vec2::new(ui.available_width(), 86.0), Sense::hover());
            widgets::waveform(
                ui.painter(),
                rect,
                &asset.waveform,
                if asset.kind == MediaKind::Video {
                    palette.video
                } else {
                    palette.accent
                },
                None,
            );
        });

    let _ = response;
    ui.add_space(14.0);
    ui.horizontal(|ui| {
        let detail = format!(
            "{} · {} · {}",
            match asset.kind {
                MediaKind::Audio => "Audio",
                MediaKind::Video => "Video",
            },
            asset
                .path
                .extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or("—")
                .to_uppercase(),
            crate::strings::format_timecode(asset.duration_ms)
        );
        ui.label(RichText::new(detail).size(11.0).color(palette.text_muted));
        if asset.width > 0 && asset.height > 0 {
            ui.label(
                RichText::new(format!("· {}×{}", asset.width, asset.height))
                    .size(11.0)
                    .color(palette.text_muted),
            );
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if ui
                .button("Ricarica anteprima")
                .on_hover_text("Rilegge il file e rigenera i metadati")
                .clicked()
            {
                app.import_paths(vec![asset.path.clone()]);
            }
        });
    });
}

fn inspector(app: &mut NerdApp, ui: &mut Ui) {
    let palette = app.palette;
    let Some(asset) = app
        .selected_asset
        .as_ref()
        .and_then(|id| app.library.find(id))
        .cloned()
    else {
        ui.label(
            RichText::new("Nessun media selezionato")
                .size(12.0)
                .color(palette.text_muted),
        );
        return;
    };
    ScrollArea::vertical()
        .id_salt("library-inspector-scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.label(
                RichText::new("Inspector")
                    .size(11.0)
                    .strong()
                    .color(palette.text_muted),
            );
            ui.add_space(10.0);
            ui.label(
                RichText::new(asset.display_title())
                    .size(17.0)
                    .strong()
                    .color(palette.text),
            );
            ui.label(
                RichText::new(if asset.kind == MediaKind::Video {
                    "Video musicale"
                } else {
                    "Traccia audio"
                })
                .size(11.0)
                .color(palette.text_muted),
            );
            ui.add_space(14.0);

            widgets::section_label(ui, palette, "Metadati");
            let mut title = asset.title.clone();
            if ui
                .add(egui::TextEdit::singleline(&mut title).hint_text("Titolo"))
                .changed()
            {
                if let Some(item) = app.library.assets.iter_mut().find(|a| a.id == asset.id) {
                    item.title = title;
                    app.save_library();
                }
            }
            ui.add_space(4.0);
            let mut artist = asset.artist.clone();
            if ui
                .add(egui::TextEdit::singleline(&mut artist).hint_text("Artista"))
                .changed()
            {
                if let Some(item) = app.library.assets.iter_mut().find(|a| a.id == asset.id) {
                    item.artist = artist;
                    app.save_library();
                }
            }
            ui.add_space(4.0);
            let mut album = asset.album.clone();
            if ui
                .add(egui::TextEdit::singleline(&mut album).hint_text("Album / progetto"))
                .changed()
            {
                if let Some(item) = app.library.assets.iter_mut().find(|a| a.id == asset.id) {
                    item.album = album;
                    app.save_library();
                }
            }

            ui.add_space(14.0);
            widgets::section_label(ui, palette, "File");
            metadata_row(ui, palette, "Percorso", &asset.path.to_string_lossy());
            metadata_row(
                ui,
                palette,
                "Durata",
                &crate::strings::format_timecode(asset.duration_ms),
            );
            metadata_row(ui, palette, "Dimensione", &size_label(asset.file_size));
            if asset.width > 0 {
                metadata_row(
                    ui,
                    palette,
                    "Risoluzione",
                    &format!("{} × {}", asset.width, asset.height),
                );
            }
            metadata_row(
                ui,
                palette,
                "Formato",
                &asset
                    .path
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .unwrap_or("—")
                    .to_uppercase(),
            );
            if !asset.exists() {
                ui.add_space(6.0);
                ui.label(
                    RichText::new("⚠ Il file non esiste più in questo percorso.")
                        .size(11.0)
                        .color(palette.danger),
                );
                if ui.button("Ricollega file…").clicked() {
                    app.relink_dialog();
                }
            }
            ui.add_space(8.0);
            if ui.button("Rivela nel Finder").clicked() {
                app.reveal_path(&asset.path);
            }

            ui.add_space(16.0);
            widgets::section_label(ui, palette, "Nella serata");
            let count = app
                .project
                .songs
                .iter()
                .filter(|song| {
                    song.audio.as_ref() == Some(&asset.path)
                        || song.video.as_ref() == Some(&asset.path)
                })
                .count();
            ui.label(
                RichText::new(if count == 0 {
                    "Non è ancora in scaletta.".to_string()
                } else {
                    format!("Usato in {count} brani della scaletta.")
                })
                .size(11.0)
                .color(palette.text_muted),
            );
            ui.add_space(8.0);
            if crate::widgets::accent_button(ui, palette, "Aggiungi alla scaletta").clicked() {
                app.add_asset_to_show(&asset.id);
            }
            ui.add_space(18.0);
            if ui
                .add(
                    egui::Button::new(
                        RichText::new("Rimuovi dalla libreria")
                            .size(11.0)
                            .color(palette.danger),
                    )
                    .frame(false),
                )
                .clicked()
            {
                let title = asset.display_title().to_string();
                app.library.remove(&asset.id);
                app.selected_asset = app.library.assets.first().map(|a| a.id.clone());
                app.save_library();
                app.toast(
                    format!("Rimosso dalla libreria: {title}"),
                    crate::state::ToastKind::Info,
                );
            }
        });
}

fn metadata_row(ui: &mut Ui, palette: crate::theme::Palette, label: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).size(10.5).color(palette.text_muted));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.add(
                egui::Label::new(
                    RichText::new(truncate(value, 34))
                        .size(10.5)
                        .color(palette.text),
                )
                .truncate(),
            )
            .on_hover_text(value);
        });
    });
}

fn paint_texture_fit(ui: &Ui, texture: &TextureHandle, rect: Rect) {
    paint_texture_fit_painter(ui.painter(), texture, rect);
}

fn paint_texture_fit_painter(painter: &egui::Painter, texture: &TextureHandle, rect: Rect) {
    let size = texture.size_vec2();
    let aspect = if size.y > 0.0 {
        size.x / size.y
    } else {
        16.0 / 9.0
    };
    let container_aspect = rect.width() / rect.height().max(1.0);
    let dest = if container_aspect > aspect {
        let height = rect.height();
        Rect::from_center_size(rect.center(), Vec2::new(height * aspect, height))
    } else {
        let width = rect.width();
        Rect::from_center_size(rect.center(), Vec2::new(width, width / aspect))
    };
    painter.image(
        texture.id(),
        dest,
        Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
        Color32::WHITE,
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

fn size_label(bytes: u64) -> String {
    if bytes > 1_000_000_000 {
        format!("{:.1} GB", bytes as f64 / 1_000_000_000.0)
    } else if bytes > 1_000_000 {
        format!("{:.1} MB", bytes as f64 / 1_000_000.0)
    } else {
        format!("{:.0} KB", bytes as f64 / 1_000.0)
    }
}
