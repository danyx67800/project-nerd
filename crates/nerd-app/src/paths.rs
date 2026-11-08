//! Filesystem locations.
//!
//! Application Support is the only writable location used implicitly. User
//! evenings live wherever the operator saves them.

use std::path::PathBuf;

use nerd_core::APP_NAME;

#[derive(Debug, Clone)]
pub struct AppPaths {
    pub support: PathBuf,
    pub logs: PathBuf,
    pub cache: PathBuf,
    pub thumbnails: PathBuf,
    pub prefs_file: PathBuf,
    pub library_file: PathBuf,
    pub autosave_file: PathBuf,
}

impl AppPaths {
    pub fn resolve() -> Self {
        let home = std::env::var_os("HOME").map(PathBuf::from);
        let support = match home {
            Some(home) => home
                .join("Library")
                .join("Application Support")
                .join(APP_NAME),
            None => std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join(".project-nerd-data"),
        };
        let cache = support.join("Cache");
        Self {
            logs: support.join("Logs"),
            thumbnails: cache.join("thumbnails"),
            prefs_file: support.join("preferences.json"),
            library_file: support.join("library.json"),
            autosave_file: support.join("Sessione-autosave.nerdshow"),
            support,
            cache,
        }
    }

    pub fn ensure(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.support)?;
        std::fs::create_dir_all(&self.logs)?;
        std::fs::create_dir_all(&self.thumbnails)?;
        Ok(())
    }
}
