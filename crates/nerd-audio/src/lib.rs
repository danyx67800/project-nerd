//! Project Nerd realtime audio.
//!
//! Design goals:
//!
//! * no allocation and no locks inside the CoreAudio callback,
//! * sample accurate fades and crossfades,
//! * separate song and background buses,
//! * pre-decoded next track for a GO latency below 100 ms,
//! * graceful device disconnection.
//!
//! Decoding is done by [`decode`] on worker threads; playback lives in
//! [`engine`]. The library importer also uses [`analyze_file`].

pub mod decode;
pub mod engine;

pub use decode::{
    analyze_file, decode_playlist, decode_stereo, is_usable, probe_summary, Analysis, DecodedAudio,
};
pub use engine::{AudioDeviceInfo, AudioEngine, AudioEvent, EngineStatus, Levels};

/// Errors surfaced to the UI in Italian.
#[derive(Debug, thiserror::Error)]
pub enum AudioError {
    #[error("errore audio: {0}")]
    Io(String),

    #[error("formato non supportato: {0}")]
    Unsupported(String),

    #[error("decodifica fallita: {0}")]
    Decode(String),

    #[error("dispositivo audio: {0}")]
    Device(String),

    #[error("brano non ancora preparato")]
    NotPrepared,

    #[error("coda comandi audio piena")]
    QueueFull,
}

pub type AudioResult<T> = Result<T, AudioError>;
