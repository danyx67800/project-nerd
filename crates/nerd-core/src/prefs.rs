//! Application preferences, persisted outside of any evening file.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Release channel used by the updater.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum UpdateChannel {
    #[default]
    Stable,
    Alpha,
}

impl UpdateChannel {
    pub fn accepts_prerelease(self) -> bool {
        matches!(self, Self::Alpha)
    }
}

/// Visual appearance of the workspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ThemeMode {
    #[default]
    Dark,
    Light,
    System,
}

/// Preferences that are not part of a saved evening.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    /// Output device name as returned by CoreAudio. `None` means system default.
    pub audio_device: Option<String>,
    pub theme: ThemeMode,
    pub update_channel: UpdateChannel,
    pub check_updates_on_startup: bool,
    /// GitHub repository in `owner/repo` form. Empty disables the updater.
    /// Defaults to the public release repository.
    pub update_repo: String,
    pub recent_projects: Vec<PathBuf>,
    /// Master fader applied on top of the show mix (0.0 - 1.0).
    pub master_volume: f32,
    pub confirm_quit_during_show: bool,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            audio_device: None,
            theme: ThemeMode::Dark,
            update_channel: UpdateChannel::Stable,
            check_updates_on_startup: true,
            update_repo: "danyx67800/project-nerd".to_string(),
            recent_projects: Vec::new(),
            master_volume: 1.0,
            confirm_quit_during_show: true,
        }
    }
}

impl Preferences {
    pub fn remember_project(&mut self, path: PathBuf) {
        self.recent_projects.retain(|p| p != &path);
        self.recent_projects.insert(0, path);
        self.recent_projects.truncate(8);
    }

    pub fn clamp(&mut self) {
        self.master_volume = self.master_volume.clamp(0.0, 1.0);
        if self.recent_projects.len() > 8 {
            self.recent_projects.truncate(8);
        }
    }
}
