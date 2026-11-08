//! Hidden component gallery (`--gallery`).
//!
//! It renders the kit twice, light and dark, so appearance regressions are
//! visible without changing the application preference.

use egui::{RichText, Ui};

use crate::components::*;
use crate::icon::Icon;
use crate::tokens::{Appearance, SystemAppearance, Theme};

pub fn show(ui: &mut Ui, system: SystemAppearance) {
    let dark = Theme::mode(Appearance::Dark, system);
    let light = Theme::mode(Appearance::Light, system);
    ui.horizontal_top(|ui| {
        let width = (ui.available_width() - 12.0) / 2.0;
        ui.allocate_ui_with_layout(
            egui::vec2(width, 0.0),
            egui::Layout::top_down(egui::Align::Min),
            |ui| column(ui, &dark),
        );
        ui.add_space(12.0);
        ui.allocate_ui_with_layout(
            egui::vec2(width, 0.0),
            egui::Layout::top_down(egui::Align::Min),
            |ui| column(ui, &light),
        );
    });
}

fn column(ui: &mut Ui, theme: &Theme) {
    let style = crate::theme::style_for(theme);
    ui.scope(|ui| {
        ui.set_style(style);
        egui::Frame::new()
            .fill(theme.colors.window)
            .inner_margin(egui::Margin::same(16))
            .corner_radius(egui::CornerRadius::same(12))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(
                    RichText::new(if theme.dark {
                        "Aspetto scuro"
                    } else {
                        "Aspetto chiaro"
                    })
                    .size(18.0)
                    .color(theme.colors.text),
                );
                ui.label(
                    RichText::new("Tutti gli stati, senza widget predefiniti")
                        .size(11.0)
                        .color(theme.colors.text_secondary),
                );
                ui.add_space(14.0);
                buttons(ui, theme);
                ui.add_space(14.0);
                fields(ui, theme);
                ui.add_space(14.0);
                surfaces(ui, theme);
            });
    });
}

fn buttons(ui: &mut Ui, theme: &Theme) {
    section_label(ui, theme, "Bottoni");
    ui.horizontal_wrapped(|ui| {
        primary_button(ui, theme, "Aggiungi alla scaletta");
        Button::new("Secondario").show(ui, theme);
        tinted_button(ui, theme, "Tinted");
        Button::new("Plain")
            .variant(ButtonVariant::Plain)
            .show(ui, theme);
        Button::new("Panic")
            .variant(ButtonVariant::Destructive)
            .show(ui, theme);
        Button::new("Disabilitato").enabled(false).show(ui, theme);
        Button::new("GO")
            .variant(ButtonVariant::Primary)
            .icon(Icon::Go)
            .keycap("Spazio")
            .show(ui, theme);
    });
    ui.add_space(8.0);
    ui.horizontal_wrapped(|ui| {
        icon_button(ui, theme, Icon::Sidebar, Some("Sidebar"), false);
        icon_button(ui, theme, Icon::Inspector, Some("Inspector"), true);
        icon_button(ui, theme, Icon::More, Some("Altro"), false);
        keycap(ui, theme, "⇧⌘P");
        status_pill(ui, theme, "In onda", theme.colors.green, Some(Icon::Play));
        status_pill(ui, theme, "Sottofondo", theme.colors.mint, None);
        status_pill(ui, theme, "Pausa", theme.colors.amber, Some(Icon::Pause));
        status_pill(ui, theme, "Panic", theme.colors.red, Some(Icon::Panic));
    });
}

fn fields(ui: &mut Ui, theme: &Theme) {
    section_label(ui, theme, "Campi");
    let mut search = String::new();
    search_field(ui, theme, &mut search, "Cerca titolo o artista");
    ui.add_space(6.0);
    let mut text = "Saggio di fine anno".to_string();
    labeled_field(ui, theme, "Nome serata", &mut text, "Nome serata");
    ui.add_space(6.0);
    let mut number = 8.0;
    number_field(
        ui,
        theme,
        "Pausa tra i brani",
        &mut number,
        0.0..=30.0,
        " s",
        0.5,
    );
    ui.add_space(6.0);
    let mut value = 0.68;
    ui.horizontal(|ui| {
        field_label(ui, theme, "Volume");
        slider(ui, theme, &mut value, 0.0..=1.0, "", 150.0);
    });
    ui.add_space(6.0);
    let mut selected = 0;
    segmented(
        ui,
        theme,
        "gallery-seg",
        &["Tutti", "Audio", "Video"],
        &mut selected,
        210.0,
    );
    let mut enabled = true;
    toggle(
        ui,
        theme,
        &mut enabled,
        "Controlla gli aggiornamenti all'avvio",
    );
}

fn surfaces(ui: &mut Ui, theme: &Theme) {
    section_label(ui, theme, "Superfici e righe");
    card(ui, theme, |ui| {
        ui.label(
            RichText::new("Card inspector")
                .size(13.0)
                .color(theme.colors.text),
        );
        ui.add_space(2.0);
        ui.label(
            RichText::new("Metadati, file e tempi")
                .size(11.0)
                .color(theme.colors.text_secondary),
        );
    });
    ui.add_space(8.0);
    let peaks = [0.2, 0.6, 0.9, 0.4, 0.7, 0.3, 0.8, 0.5, 0.2, 0.7, 0.9, 0.4];
    ListRow {
        title: "Ouverture",
        subtitle: "Orchestra della scuola · 03:42",
        duration: "03:42",
        icon: Icon::Music,
        accent: theme.colors.green,
        selected: true,
        badge: Some("In scaletta"),
        waveform: Some(&peaks),
        thumbnail: None,
        trailing_icon: Some(Icon::Video),
        missing: false,
    }
    .show(ui, theme);
    ListRow::new("Nocturne", "Piano solo · 04:10", Icon::Video).show(ui, theme);
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(160.0, 40.0), egui::Sense::hover());
        waveform(ui.painter(), rect, &peaks, theme.colors.accent, Some(0.42));
        progress_bar(ui, theme, 0.64, theme.colors.accent);
    });
    ui.add_space(8.0);
    vu_meter(ui, theme, "Brano", 0.72, 0.68, 0.86, 0.78);
    ui.add_space(6.0);
    drop_zone(ui, theme, "Trascina qui i file della serata", false);
    ui.add_space(6.0);
    empty_state(
        ui,
        theme,
        Icon::Music,
        "Nessun media",
        "Importa i file per iniziare.",
        |ui| {
            primary_button(ui, theme, "Importa…");
        },
    );
}
