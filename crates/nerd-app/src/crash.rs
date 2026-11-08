//! Crash safety: keep the latest project in memory so the panic hook can write
//! a recovery file before the process dies.

use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use nerd_core::persistence::save_recovery;
use nerd_core::Project;

static SNAPSHOT: Mutex<Option<Project>> = Mutex::new(None);
static RECOVERY_DIR: OnceLock<PathBuf> = OnceLock::new();

pub fn install(app_dir: PathBuf) {
    let _ = RECOVERY_DIR.set(app_dir);
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        tracing::error!(panic = %info, "unhandled panic");
        if let (Some(dir), Ok(snapshot)) = (RECOVERY_DIR.get(), SNAPSHOT.lock()) {
            if let Some(project) = snapshot.as_ref() {
                if let Err(err) = save_recovery(project, dir) {
                    tracing::error!(error = %err, "could not write panic recovery");
                }
            }
        }
        eprintln!("{info}");
        previous(info);
    }));
}

/// Update the snapshot. Called on edits and show transitions; keeps the most
/// recent state without touching disk.
pub fn set_snapshot(project: &Project) {
    if let Ok(mut slot) = SNAPSHOT.lock() {
        *slot = Some(project.clone());
    }
}
