//! Project Nerd core.
//!
//! This crate is deliberately free of macOS, audio and UI dependencies so the
//! show logic can be tested anywhere. It owns:
//!
//! * the evening/project data model ([`model`]),
//! * saving, loading, recovery and relinking ([`persistence`]),
//! * the deterministic show state machine ([`show`]),
//! * the media library index ([`library`]),
//! * small pure helpers such as [`waveform`].

pub mod branding;
pub mod error;
pub mod library;
pub mod model;
pub mod persistence;
pub mod prefs;
pub mod show;
pub mod waveform;

pub use branding::{APP_ID, APP_NAME, APP_VERSION, CURRENT_SCHEMA_VERSION, PROJECT_EXTENSION};
pub use error::{CoreError, CoreResult};
pub use model::{BackgroundSettings, Project, Song, SongStatus, TitleInfo, TitleTemplate};
pub use show::{Effect, GoOutcome, Phase, Show, ShowConfig};
