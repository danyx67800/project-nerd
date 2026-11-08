//! Unified macOS chrome shared by every workspace page.
#![allow(deprecated)]
//!
//! Native traffic lights stay visible; the system title is hidden and this
//! 52 pt strip is drawn underneath them. Page navigation lives in a DaVinci-
//! style 56 pt bar at the bottom.

use eframe::egui::{self, Align, Layout, Pos2, Rangef, Rect, RichText, Sense, Ui, Vec2};
use nerd_ui_kit::{
    body, caption, icon, icon_button, keycap, menu_button, menu_item, primary_button, secondary,
    section_title, ButtonVariant, Icon, Theme,
};

use crate::app::NerdApp;
use crate::output;
use crate::state::Page;
use crate::strings;

pub fn titlebar(app: &mut NerdApp, ui: &mut Ui, frame: &mut eframe::Frame) {
    frame.set_traffic_lights_position(ui.ctx(), Rangef::new(0.0, 52.0), 14.0);
    let theme = app.palette;
    egui::Panel::top("workspace-titlebar")
        .exact_size(theme.metrics.titlebar)
        .frame(
            egui::Frame::new()
                .fill(theme.colors.window)
                .inner_margin(egui::Margin::symmetric(12, 0)),
        )
        .show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                // Native traffic lights occupy the first 76 pt.
                ui.add_space(76.0);
                if icon_button(
                    ui,
                    &theme,
                    Icon::Sidebar,
                    Some("Mostra o nascondi la barra laterale"),
                    app.chrome.library_sidebar,
                )
                .clicked()
                {
                    app.chrome.library_sidebar = !app.chrome.library_sidebar;
                }
                ui.add_space(4.0);
                project_identity(app, ui, &theme);
                ui.add_space(12.0);
                let available = ui.available_width();
                let controls_width = 390.0;
                let status_width = (available - controls_width).clamp(170.0, 330.0);
                show_status_capsule(app, ui, &theme, status_width);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if icon_button(
                        ui,
                        &theme,
                        Icon::Inspector,
                        Some("Mostra o nascondi l'inspector"),
                        app.chrome.inspector_open,
                    )
                    .clicked()
                    {
                        app.chrome.inspector_open = !app.chrome.inspector_open;
                    }
                    if icon_button(
                        ui,
                        &theme,
                        Icon::Blackout,
                        Some("Blackout uscita video (⌘B)"),
                        app.project.output.blackout,
                    )
                    .clicked()
                    {
                        app.toggle_blackout();
                    }
                    output_menu(app, ui, &theme);
                });
            });
        });
    hairline(ui, &theme, "titlebar");
}

fn project_identity(app: &mut NerdApp, ui: &mut Ui, theme: &Theme) {
    ui.allocate_ui_with_layout(Vec2::new(210.0, 44.0), Layout::top_down(Align::Min), |ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        let mut name = app.project.name.clone();
        let before = app.project.clone();
        let response = nerd_ui_kit::bare_text_field(
            ui,
            theme,
            &mut name,
            "Nome serata",
            210.0,
            egui::FontId::proportional(13.0),
        );
        if response.changed() {
            app.project.name = name;
            app.commit_edit(&before);
        }
        response.on_hover_text("Nome della serata — clicca per modificare");
        let count = app.project.songs.len();
        let minutes = app.project.total_duration_ms() / 60_000;
        let saved = if app.dirty {
            strings::UNSAVED
        } else {
            strings::SAVED
        };
        ui.label(
            RichText::new(format!("{saved} · {count} brani · {minutes} min"))
                .font(caption())
                .color(if app.dirty {
                    theme.colors.amber
                } else {
                    theme.colors.text_secondary
                }),
        );
    });
}

fn show_status_capsule(app: &mut NerdApp, ui: &mut Ui, theme: &Theme, width: f32) {
    let phase = app.show.phase();
    let color = phase_color(theme, phase);
    let label = output::phase_label(phase);
    let detail = app
        .live_song()
        .map(|song| song.display_title())
        .or_else(|| {
            app.next_song()
                .map(|song| format!("Prossimo: {}", song.display_title()))
        })
        .unwrap_or_else(|| "Scaletta vuota".to_string());
    let time = app
        .live_song()
        .map(|song| {
            let remaining = song
                .effective_duration_ms()
                .saturating_sub(app.show.elapsed_ms());
            format!(" · {}", nerd_ui_kit::format_clock(remaining))
        })
        .unwrap_or_default();
    let width = width.max(160.0);
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 30.0), Sense::click());
    let fill = if response.hovered() {
        theme.colors.card
    } else {
        theme.colors.panel
    };
    ui.painter()
        .rect_filled(rect, egui::CornerRadius::same(15), fill);
    ui.painter().rect_stroke(
        rect,
        egui::CornerRadius::same(15),
        egui::Stroke::new(1.0, theme.colors.separator),
        egui::StrokeKind::Inside,
    );
    let dot_color = if app.show.background_running() && phase == nerd_core::Phase::Idle {
        theme.colors.mint
    } else {
        color
    };
    ui.painter().circle_filled(
        Pos2::new(rect.left() + 13.0, rect.center().y),
        4.0,
        dot_color,
    );
    ui.painter().text(
        Pos2::new(rect.left() + 23.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        body(),
        theme.colors.text,
    );
    let label_width = ui
        .painter()
        .layout_no_wrap(label.to_string(), body(), theme.colors.text)
        .size()
        .x;
    let detail_x = rect.left() + 23.0 + label_width + 7.0;
    let max_width = rect.right() - detail_x - 10.0;
    if max_width > 12.0 {
        nerd_ui_kit::paint_truncated(
            ui.painter(),
            Pos2::new(detail_x, rect.center().y - 7.0),
            &format!("{detail}{time}"),
            caption(),
            theme.colors.text_secondary,
            max_width,
        );
    }
    if response.clicked() {
        app.page = Page::Live;
    }
    response.on_hover_text("Apri la pagina Live");
}

fn phase_color(theme: &Theme, phase: nerd_core::Phase) -> egui::Color32 {
    match phase {
        nerd_core::Phase::Playing => theme.colors.green,
        nerd_core::Phase::Paused => theme.colors.amber,
        nerd_core::Phase::Waiting => theme.colors.amber,
        nerd_core::Phase::Stopping => theme.colors.amber,
        nerd_core::Phase::Panic => theme.colors.red,
        nerd_core::Phase::Finished => theme.colors.green,
        nerd_core::Phase::Idle => theme.colors.text_tertiary,
    }
}

fn output_menu(app: &mut NerdApp, ui: &mut Ui, theme: &Theme) {
    let display = app.display_name();
    let label = if app.output_open {
        display
    } else {
        format!("Uscita · {display}")
    };
    let width = 168.0;
    menu_button(
        ui,
        theme,
        "output",
        &label,
        Some(Icon::Monitor),
        if app.output_open {
            ButtonVariant::Tinted
        } else {
            ButtonVariant::Plain
        },
        width,
        |ui| {
            let displays: Vec<(usize, String)> = app
                .displays
                .iter()
                .map(|display| (display.index, display.name.clone()))
                .collect();
            for (index, name) in displays {
                let selected = index == app.project.output.display_index;
                let response = menu_item(
                    ui,
                    theme,
                    &name,
                    if selected { Some(Icon::Check) } else { None },
                );
                if response.clicked() {
                    app.project.output.display_index = index;
                    app.mark_dirty();
                    if app.output_open {
                        app.close_output_viewport();
                    }
                }
            }
            ui.add_space(4.0);
            nerd_ui_kit::divider(ui, theme);
            ui.add_space(4.0);
            if menu_item(ui, theme, "Aggiorna schermi", Some(Icon::Refresh)).clicked() {
                app.refresh_displays();
            }
            if menu_item(
                ui,
                theme,
                if app.output_open {
                    "Chiudi uscita video"
                } else {
                    "Apri uscita video"
                },
                Some(Icon::Monitor),
            )
            .clicked()
            {
                app.toggle_output(ui.ctx());
            }
        },
    )
    .on_hover_text("Scegli lo schermo di uscita");
}

pub fn pagebar(app: &mut NerdApp, ui: &mut Ui) {
    let theme = app.palette;
    egui::Panel::bottom("workspace-pages")
        .exact_size(theme.metrics.pagebar)
        .frame(
            egui::Frame::new()
                .fill(theme.colors.panel)
                .inner_margin(egui::Margin::symmetric(14, 0)),
        )
        .show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                for page in Page::ALL {
                    page_item(app, ui, &theme, page);
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if icon_button(
                        ui,
                        &theme,
                        Icon::Keyboard,
                        Some("Scorciatoie da tastiera (?)"),
                        app.chrome.help,
                    )
                    .clicked()
                    {
                        app.chrome.help = !app.chrome.help;
                    }
                });
            });
        });
    hairline(ui, &theme, "pagebar");
    if app.chrome.help {
        shortcuts_popover(app, ui, &theme);
    }
}

fn page_item(app: &mut NerdApp, ui: &mut Ui, theme: &Theme, page: Page) {
    let active = app.page == page;
    let icon_kind = match page {
        Page::Library => Icon::Library,
        Page::Timeline => Icon::Timeline,
        Page::Titles => Icon::Titles,
        Page::Live => Icon::Live,
        Page::Settings => Icon::Settings,
        Page::Gallery => Icon::Grid,
    };
    let width = 104.0;
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 48.0), Sense::click());
    if response.hovered() || active {
        ui.painter().rect_filled(
            rect,
            egui::CornerRadius::same(8),
            if active {
                theme.colors.accent_soft
            } else {
                theme.colors.hover
            },
        );
    }
    if active {
        ui.painter().rect_filled(
            Rect::from_min_size(
                Pos2::new(rect.left() + 18.0, rect.top() + 1.0),
                Vec2::new(rect.width() - 36.0, 2.0),
            ),
            egui::CornerRadius::same(1),
            theme.colors.accent,
        );
    }
    let color = if active {
        theme.colors.accent
    } else {
        theme.colors.text_secondary
    };
    icon(
        ui.painter(),
        Rect::from_center_size(
            Pos2::new(rect.center().x, rect.top() + 17.0),
            Vec2::splat(18.0),
        ),
        icon_kind,
        color,
        1.5,
    );
    ui.painter().text(
        Pos2::new(rect.center().x, rect.top() + 31.0),
        egui::Align2::CENTER_TOP,
        page.title(),
        nerd_ui_kit::tokens::micro(),
        color,
    );
    if response.clicked() {
        app.page = page;
        app.chrome.help = false;
    }
    response.on_hover_text(format!("{} (⌘{})", page.title(), page.index() + 1));
}

fn shortcuts_popover(app: &mut NerdApp, ui: &mut Ui, theme: &Theme) {
    egui::Area::new(egui::Id::new("shortcuts-popover"))
        .order(egui::Order::Foreground)
        .anchor(egui::Align2::RIGHT_BOTTOM, [-18.0, -64.0])
        .show(ui.ctx(), |ui| {
            egui::Frame::new()
                .fill(theme.colors.popover)
                .stroke(egui::Stroke::new(1.0, theme.colors.separator_strong))
                .corner_radius(egui::CornerRadius::same(11))
                .inner_margin(egui::Margin::same(16))
                .shadow(egui::epaint::Shadow {
                    offset: [0, 12],
                    blur: 32,
                    spread: 0,
                    color: theme.colors.shadow,
                })
                .show(ui, |ui| {
                    ui.set_width(330.0);
                    ui.label(
                        RichText::new("Scorciatoie")
                            .font(section_title())
                            .color(theme.colors.text),
                    );
                    ui.add_space(2.0);
                    ui.label(
                        RichText::new("Durante lo spettacolo")
                            .font(secondary())
                            .color(theme.colors.text_secondary),
                    );
                    ui.add_space(8.0);
                    for (key, action) in [
                        ("Spazio", "GO / avvia il brano"),
                        ("P", "Pausa o riprendi"),
                        ("S", "Stop con fade"),
                        ("N", "Salta al prossimo"),
                        ("B", "Brano precedente"),
                        ("⇧⌘P", "Panic"),
                        ("⌘B", "Blackout uscita video"),
                    ] {
                        shortcut_row(ui, theme, key, action);
                    }
                    ui.add_space(10.0);
                    ui.label(
                        RichText::new("Sempre")
                            .font(secondary())
                            .color(theme.colors.text_secondary),
                    );
                    ui.add_space(6.0);
                    for (key, action) in [
                        ("⌘S", "Salva"),
                        ("⌘I", "Importa media"),
                        ("⌘Z", "Annulla"),
                        ("⌘1…5", "Cambia pagina"),
                    ] {
                        shortcut_row(ui, theme, key, action);
                    }
                    ui.add_space(10.0);
                    if primary_button(ui, theme, "Chiudi").clicked() {
                        app.chrome.help = false;
                    }
                });
        });
}

fn shortcut_row(ui: &mut Ui, theme: &Theme, key: &str, action: &str) {
    ui.horizontal(|ui| {
        keycap(ui, theme, key);
        ui.label(
            RichText::new(action)
                .font(caption())
                .color(theme.colors.text_secondary),
        );
    });
}

pub fn recovery_banner(app: &mut NerdApp, ui: &mut Ui) {
    if !app.recovery_prompt || app.recovery.is_none() {
        return;
    }
    let theme = app.palette;
    banner(
        ui,
        &theme,
        "Sessione non salvata rilevata",
        "L'app è stata chiusa in modo imprevisto durante l'ultima serata.",
        theme.colors.amber,
        |ui| {
            if primary_button(ui, &theme, "Ripristina sessione").clicked() {
                app.restore_recovery();
            }
            if nerd_ui_kit::Button::new("Ignora")
                .variant(ButtonVariant::Plain)
                .show(ui, &theme)
                .clicked()
            {
                app.discard_recovery();
            }
        },
    );
}

pub fn missing_banner(app: &mut NerdApp, ui: &mut Ui) {
    if app.missing.is_empty() {
        return;
    }
    let theme = app.palette;
    let names: Vec<String> = app
        .missing
        .iter()
        .take(2)
        .map(|m| m.song_title.clone())
        .collect();
    banner(
        ui,
        &theme,
        &format!("{} file mancanti", app.missing.len()),
        &names.join(", "),
        theme.colors.red,
        |ui| {
            if nerd_ui_kit::Button::new("Ricollega…")
                .variant(ButtonVariant::Secondary)
                .show(ui, &theme)
                .clicked()
            {
                app.relink_dialog();
            }
        },
    );
}

fn banner(
    ui: &mut Ui,
    theme: &Theme,
    title: &str,
    body_text: &str,
    color: egui::Color32,
    actions: impl FnOnce(&mut Ui),
) {
    egui::Panel::top(format!("banner-{title}"))
        .frame(
            egui::Frame::new()
                .fill(color.gamma_multiply(0.10))
                .inner_margin(egui::Margin::symmetric(18, 7)),
        )
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                icon(
                    ui.painter(),
                    Rect::from_center_size(
                        ui.cursor().min + Vec2::new(8.0, 0.0),
                        Vec2::splat(14.0),
                    ),
                    Icon::Warning,
                    color,
                    1.4,
                );
                ui.add_space(18.0);
                ui.label(RichText::new(title).strong().size(12.0).color(color));
                ui.label(
                    RichText::new(body_text)
                        .size(11.0)
                        .color(theme.colors.text_secondary),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| actions(ui));
            });
        });
}

fn hairline(ui: &mut Ui, theme: &Theme, id: &str) {
    let _ = id;
    let rect = ui.max_rect();
    ui.painter().line_segment(
        [
            Pos2::new(rect.left(), rect.bottom()),
            Pos2::new(rect.right(), rect.bottom()),
        ],
        egui::Stroke::new(1.0, theme.colors.separator),
    );
}
