//! Italian UI strings.
//!
//! Kept in one module so a future translation can replace this catalog without
//! touching layout code. Page-specific strings live next to their page.

pub const GO: &str = "GO";
pub const PAUSE: &str = "Pausa";
pub const RESUME: &str = "Riprendi";
pub const STOP: &str = "Stop";
pub const SKIP: &str = "Salta";
pub const BACK: &str = "Indietro";
pub const PANIC: &str = "Panic";
pub const BLACKOUT: &str = "Blackout";
pub const MODE_MANUAL: &str = "Manuale";
pub const MODE_AUTO: &str = "Automatico";
pub const ON_AIR: &str = "In onda";
pub const NEXT: &str = "Prossimo";
pub const BACKGROUND: &str = "Sottofondo";
pub const SETTINGS: &str = "Impostazioni";
pub const PAGE_LIBRARY: &str = "Libreria";
pub const PAGE_TIMELINE: &str = "Scaletta";
pub const PAGE_TITLES: &str = "Titoli";
pub const PAGE_LIVE: &str = "Live";
pub const UNSAVED: &str = "Non salvato";
pub const SAVED: &str = "Salvato";
pub const READY: &str = "Pronto";
pub const MISSING: &str = "File mancante";
pub const OUTPUT: &str = "Uscita video";

pub fn format_timecode(ms: u64) -> String {
    let total_seconds = ms / 1000;
    let minutes = total_seconds / 60;
    let seconds = total_seconds % 60;
    let tenths = (ms % 1000) / 100;
    format!("{minutes:02}:{seconds:02}.{tenths}")
}

pub fn format_short_duration(ms: u64) -> String {
    let total_seconds = ms / 1000;
    let minutes = total_seconds / 60;
    let seconds = total_seconds % 60;
    if minutes >= 60 {
        format!("{}h {:02}m", minutes / 60, minutes % 60)
    } else {
        format!("{minutes}m {seconds:02}s")
    }
}

pub fn format_volume(volume: f32) -> String {
    if volume <= 0.0001 {
        "-∞ dB".to_string()
    } else {
        format!("{:+.1} dB", 20.0 * volume.log10())
    }
}
