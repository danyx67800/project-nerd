//! Synthetic evening used by `--demo` and by the visual checks.
//!
//! Everything is written under the cache directory; no user library or evening
//! is touched. The generated WAV files keep the demo self-contained.

use std::f32::consts::TAU;
use std::path::Path;

use nerd_core::library::{Library, MediaAsset, MediaKind};
use nerd_core::model::{Project, Song};
use nerd_core::Show;

use crate::app::NerdApp;

pub fn populate(app: &mut NerdApp) {
    let dir = app.paths.cache.join("demo");
    if std::fs::create_dir_all(&dir).is_err() {
        return;
    }

    let songs = [
        ("Ouverture", "Orchestra della scuola", 32.0, 196.0),
        ("Nocturne", "Piano solo", 28.0, 261.63),
        ("Allegro", "Ensemble d'archi", 24.0, 329.63),
        ("Intermezzo", "Duo di chitarre", 22.0, 293.66),
        ("Aria", "Soprano e pianoforte", 30.0, 392.0),
        ("Ritmo", "Percussioni", 26.0, 174.61),
        ("Notturno", "Quartetto d'archi", 34.0, 220.0),
        ("Variazioni", "Pianoforte a quattro mani", 29.0, 246.94),
        ("Coro", "Coro della scuola", 31.0, 146.83),
        ("Finale", "Tutti gli interpreti", 38.0, 220.0),
        ("Bis", "Orchestra della scuola", 21.0, 196.0),
        ("Saluto", "Ensemble d'archi", 18.0, 261.63),
    ];

    let mut project = Project::new("Saggio di fine anno");
    project.venue = "Auditorium della scuola".to_string();
    project.date = "2026-11-08".to_string();
    project.notes = "Serata di saggio. Luci calde, proiezione 16:9.".to_string();
    project.background.volume = 0.28;
    project.background.looping = true;

    let mut library = Library::new();
    for (index, (title, artist, seconds, frequency)) in songs.iter().enumerate() {
        let path = dir.join(format!("{:02}-{}.wav", index + 1, slug(title)));
        let duration_ms = (*seconds * 1000.0) as u64;
        if !path.is_file() && write_wave(&path, *seconds, *frequency).is_err() {
            continue;
        }
        let mut asset = MediaAsset::new(path.clone(), MediaKind::Audio, duration_ms);
        asset.title = (*title).to_string();
        asset.artist = (*artist).to_string();
        asset.album = "Project Nerd · Demo".to_string();
        asset.waveform = envelope_waveform(index, 96);
        asset.added_at_unix = 1_760_000_000 + index as u64;
        library.assets.push(asset);

        let mut song = Song::new(*title, Some(path), duration_ms);
        song.artist = (*artist).to_string();
        song.info.authors = "Tradizionale".to_string();
        song.info.performers = if index % 3 == 0 {
            "Classe 3ª".to_string()
        } else {
            "Laboratorio musicale".to_string()
        };
        song.fade_in_ms = 420;
        song.fade_out_ms = 900;
        project.add_song(song);
    }

    for (index, name) in ["Ouverture", "Intermezzo", "Notturno"].iter().enumerate() {
        let path = dir.join(format!("bg-{index}-{}.wav", slug(name)));
        if !path.is_file() {
            let _ = write_wave(&path, 20.0, 110.0 + index as f32 * 20.0);
        }
        project.background.tracks.push(path);
    }
    project.output.show_titles = true;
    project.title_template.name = "Intestazione serata".to_string();

    app.project = project;
    app.library = library;
    app.show = Show::from_project(&app.project);
    app.selected_song = app.project.songs.first().map(|song| song.id.clone());
    app.selected_asset = app.library.assets.first().map(|asset| asset.id.clone());
    app.dirty = false;
    app.refresh_missing();
    app.ensure_background_loaded();
    app.ensure_preparations();
}

fn slug(title: &str) -> String {
    title
        .to_ascii_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect()
}

fn envelope_waveform(seed: usize, buckets: usize) -> Vec<f32> {
    (0..buckets)
        .map(|i| {
            let t = i as f32 / buckets as f32;
            let pulse = (t * TAU * (2.0 + seed as f32 * 0.17)).sin().abs();
            (0.18 + pulse * 0.68 * (0.35 + 0.65 * (t * std::f32::consts::PI).sin())).clamp(0.0, 1.0)
        })
        .collect()
}

fn write_wave(path: &Path, seconds: f32, frequency: f32) -> std::io::Result<()> {
    let sample_rate = 22_050u32;
    let frames = (seconds * sample_rate as f32) as u32;
    let data_len = frames * 2;
    let mut bytes = Vec::with_capacity(44 + data_len as usize);
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data_len).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&sample_rate.to_le_bytes());
    bytes.extend_from_slice(&(sample_rate * 2).to_le_bytes());
    bytes.extend_from_slice(&2u16.to_le_bytes());
    bytes.extend_from_slice(&16u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_len.to_le_bytes());
    for frame in 0..frames {
        let t = frame as f32 / sample_rate as f32;
        let fade = (t / 1.4).min(1.0).min(((seconds - t) / 1.8).max(0.0));
        let vibrato = 1.0 + 0.006 * (t * TAU * 4.6).sin();
        let value = (t * frequency * vibrato * TAU).sin() * 0.20
            + (t * frequency * 2.0 * TAU).sin() * 0.06
            + (t * frequency * 0.5 * TAU).sin() * 0.04;
        let sample = (value * fade * i16::MAX as f32 * 0.7) as i16;
        bytes.extend_from_slice(&sample.to_le_bytes());
    }
    std::fs::write(path, bytes)
}
