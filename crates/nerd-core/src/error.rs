//! Error type shared by persistence, the library and project validation.

use std::path::PathBuf;

/// Errors produced by the platform-independent core.
#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("errore di I/O: {0}")]
    Io(#[from] std::io::Error),

    #[error("formato JSON non valido: {0}")]
    Json(#[from] serde_json::Error),

    #[error("il file non è un progetto Project Nerd: {0}")]
    NotAProject(PathBuf),

    #[error("la serata usa lo schema {found}, questa app supporta fino allo schema {supported}")]
    UnsupportedSchema { found: u32, supported: u32 },

    #[error("percorso non valido")]
    InvalidPath,

    #[error("operazione annullata")]
    Cancelled,

    #[error("{0}")]
    Other(String),
}

/// Convenience result used throughout the core.
pub type CoreResult<T> = Result<T, CoreError>;
