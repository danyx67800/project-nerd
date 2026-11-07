//! Index of imported media, independent from any single evening.
//!
//! Importing an asset copies no audio: the library stores the original path,
//! probed metadata and cached analysis data. Evenings reference the same files,
//! so a song can be reused across concerts without duplicating media.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::model::new_id;

/// The kind of media an asset points at.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaKind {
    Audio,
    Video,
}

/// An imported file.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct MediaAsset {
    pub id: String,
    pub path: PathBuf,
    pub kind: MediaKind,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration_ms: u64,
    /// Video dimensions, when known.
    pub width: u32,
    pub height: u32,
    pub file_size: u64,
    /// Cached downsampled waveform peaks (0.0 - 1.0), computed by the audio crate.
    pub waveform: Vec<f32>,
    /// Cached thumbnail as RGBA8, stored separately on disk by the app.
    pub has_thumbnail: bool,
    pub added_at_unix: u64,
}

impl Default for MediaAsset {
    fn default() -> Self {
        Self {
            id: new_id(),
            path: PathBuf::new(),
            kind: MediaKind::Audio,
            title: String::new(),
            artist: String::new(),
            album: String::new(),
            duration_ms: 0,
            width: 0,
            height: 0,
            file_size: 0,
            waveform: Vec::new(),
            has_thumbnail: false,
            added_at_unix: 0,
        }
    }
}

impl MediaAsset {
    pub fn new(path: PathBuf, kind: MediaKind, duration_ms: u64) -> Self {
        let title = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Senza titolo")
            .to_string();
        let file_size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        Self {
            path,
            kind,
            title,
            duration_ms,
            file_size,
            ..Self::default()
        }
    }

    pub fn display_title(&self) -> &str {
        if self.title.trim().is_empty() {
            self.path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Senza titolo")
        } else {
            &self.title
        }
    }

    pub fn exists(&self) -> bool {
        self.path.is_file()
    }
}

/// Media library persisted at `library.json` in Application Support.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Library {
    pub assets: Vec<MediaAsset>,
}

impl Library {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds an asset, replacing metadata for an identical canonical path.
    pub fn upsert(&mut self, asset: MediaAsset) -> bool {
        let canonical = asset
            .path
            .canonicalize()
            .unwrap_or_else(|_| asset.path.clone());
        if let Some(existing) = self
            .assets
            .iter_mut()
            .find(|a| a.path.canonicalize().unwrap_or_else(|_| a.path.clone()) == canonical)
        {
            let id = existing.id.clone();
            let waveform = if asset.waveform.is_empty() {
                existing.waveform.clone()
            } else {
                asset.waveform
            };
            let has_thumbnail = existing.has_thumbnail || asset.has_thumbnail;
            *existing = MediaAsset {
                id,
                waveform,
                has_thumbnail,
                ..asset
            };
            return false;
        }
        self.assets.push(asset);
        true
    }

    pub fn find(&self, id: &str) -> Option<&MediaAsset> {
        self.assets.iter().find(|a| a.id == id)
    }

    pub fn find_by_path(&self, path: &Path) -> Option<&MediaAsset> {
        let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        self.assets
            .iter()
            .find(|a| a.path.canonicalize().unwrap_or_else(|_| a.path.clone()) == canonical)
    }

    pub fn remove(&mut self, id: &str) -> Option<MediaAsset> {
        let index = self.assets.iter().position(|a| a.id == id)?;
        Some(self.assets.remove(index))
    }

    pub fn missing(&self) -> impl Iterator<Item = &MediaAsset> {
        self.assets.iter().filter(|a| !a.path.is_file())
    }

    pub fn sort_by_title(&mut self) {
        self.assets.sort_by(|a, b| {
            a.display_title()
                .to_lowercase()
                .cmp(&b.display_title().to_lowercase())
        });
    }
}

/// Extensions accepted by the import dialog and drag & drop.
pub const AUDIO_EXTENSIONS: &[&str] = &["mp3", "m4a", "wav", "aiff", "aif", "flac"];
pub const VIDEO_EXTENSIONS: &[&str] = &["mp4", "mov", "m4v"];

pub fn media_kind_for_path(path: &Path) -> Option<MediaKind> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    if VIDEO_EXTENSIONS.contains(&ext.as_str()) {
        Some(MediaKind::Video)
    } else if AUDIO_EXTENSIONS.contains(&ext.as_str()) {
        Some(MediaKind::Audio)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_supported_extensions() {
        assert_eq!(
            media_kind_for_path(Path::new("a.MP4")),
            Some(MediaKind::Video)
        );
        assert_eq!(
            media_kind_for_path(Path::new("a.flac")),
            Some(MediaKind::Audio)
        );
        assert_eq!(media_kind_for_path(Path::new("a.txt")), None);
    }

    #[test]
    fn upsert_deduplicates_paths() {
        let mut library = Library::new();
        let first = MediaAsset::new(
            PathBuf::from("/tmp/does-not-exist.mp3"),
            MediaKind::Audio,
            1,
        );
        assert!(library.upsert(first));
        let second = MediaAsset::new(
            PathBuf::from("/tmp/does-not-exist.mp3"),
            MediaKind::Audio,
            2,
        );
        assert!(!library.upsert(second));
        assert_eq!(library.assets.len(), 1);
        assert_eq!(library.assets[0].duration_ms, 2);
    }
}
