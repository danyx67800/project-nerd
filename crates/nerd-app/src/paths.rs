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
        // Explicit isolation hook for demos, snapshots and integration tests.
        if let Some(support) = std::env::var_os("PROJECT_NERD_SUPPORT") {
            return Self::under(PathBuf::from(support));
        }
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
        Self::under(support)
    }

    /// Isolated data directory for `--demo` and visual checks. It never reads
    /// or writes the operator's real library, preferences or autosave.
    pub fn demo() -> Self {
        Self::under(std::env::temp_dir().join("project-nerd-demo"))
    }

    fn under(support: PathBuf) -> Self {
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
