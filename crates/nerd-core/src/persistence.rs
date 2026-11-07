//! Saving and loading evenings, preferences, the media library and recovery
//! snapshots. Writes are atomic: a crash can leave a temporary file, never a
//! half-written project.

use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::de::DeserializeOwned;
use serde::Serialize;
use tracing::{info, warn};

use crate::branding::{CURRENT_SCHEMA_VERSION, PROJECT_EXTENSION};
use crate::error::{CoreError, CoreResult};
use crate::library::Library;
use crate::model::{IdleScreen, Project, ResumeMode, SongStatus};
use crate::prefs::Preferences;

pub const LIBRARY_FILE: &str = "library.json";
pub const PREFS_FILE: &str = "preferences.json";
pub const RECOVERY_FILE: &str = "recovery.nerdshow";
pub const WINDOW_STATE_FILE: &str = "window.json";

/// Writes bytes through `<name>.tmp` and renames over the destination.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> CoreResult<()> {
    let parent = path.parent().ok_or(CoreError::InvalidPath)?;
    fs::create_dir_all(parent)?;
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("document");
    let tmp = parent.join(format!(".{}.{}.tmp", file_name, std::process::id()));
    let result = (|| -> CoreResult<()> {
        let mut file = fs::File::create(&tmp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        fs::rename(&tmp, path)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result
}

fn save_json<T: Serialize>(value: &T, path: &Path) -> CoreResult<()> {
    let bytes = serde_json::to_vec_pretty(value)?;
    write_atomic(path, &bytes)
}

fn load_json<T: DeserializeOwned>(path: &Path) -> CoreResult<T> {
    let bytes = fs::read(path)?;
    Ok(serde_json::from_slice(&bytes)?)
}

/// Saves a project, validating it first so on-disk data is always sane.
pub fn save_project(project: &Project, path: &Path) -> CoreResult<()> {
    let mut clean = project.clone();
    clean.validate();
    save_json(&clean, path)?;
    info!(path = %path.display(), "project saved");
    Ok(())
}

/// Loads a project and rejects documents from a newer schema.
pub fn load_project(path: &Path) -> CoreResult<Project> {
    let mut project: Project = load_json(path).map_err(|err| match err {
        CoreError::Json(_) => CoreError::NotAProject(path.to_path_buf()),
        other => other,
    })?;
    if project.schema_version > CURRENT_SCHEMA_VERSION {
        return Err(CoreError::UnsupportedSchema {
            found: project.schema_version,
            supported: CURRENT_SCHEMA_VERSION,
        });
    }
    project.validate();
    info!(path = %path.display(), songs = project.songs.len(), "project loaded");
    Ok(project)
}

/// A missing media reference, with enough context to explain it to the operator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissingMedia {
    pub role: MediaRole,
    pub song_id: Option<String>,
    pub song_title: String,
    pub path: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaRole {
    SongAudio,
    SongVideo,
    Background,
}

impl MediaRole {
    pub fn label_it(self) -> &'static str {
        match self {
            Self::SongAudio => "Audio del brano",
            Self::SongVideo => "Video del brano",
            Self::Background => "Sottofondo",
        }
    }
}

/// Lists every referenced file that is not currently on disk.
pub fn missing_media(project: &Project) -> Vec<MissingMedia> {
    let mut out = Vec::new();
    for song in &project.songs {
        if let Some(path) = &song.audio {
            if !path.is_file() {
                out.push(MissingMedia {
                    role: MediaRole::SongAudio,
                    song_id: Some(song.id.clone()),
                    song_title: song.display_title(),
                    path: path.clone(),
                });
            }
        }
        if let Some(path) = &song.video {
            if !path.is_file() {
                out.push(MissingMedia {
                    role: MediaRole::SongVideo,
                    song_id: Some(song.id.clone()),
                    song_title: song.display_title(),
                    path: path.clone(),
                });
            }
        }
    }
    for path in &project.background.tracks {
        if !path.is_file() {
            out.push(MissingMedia {
                role: MediaRole::Background,
                song_id: None,
                song_title: "Sottofondo".to_string(),
                path: path.clone(),
            });
        }
    }
    out
}

/// Result of a relink pass.
#[derive(Debug, Clone, Default)]
pub struct RelinkReport {
    pub relinked: usize,
    pub unresolved: Vec<PathBuf>,
}

/// Searches `root` for files with the same name as missing references and
/// rewrites the project paths in place. Matching is case-insensitive.
pub fn relink_media(project: &mut Project, root: &Path) -> RelinkReport {
    let mut index: HashMap<String, PathBuf> = HashMap::new();
    collect_files(root, 0, 8, &mut index);

    let mut report = RelinkReport::default();
    let resolve = |path: &Option<PathBuf>, relinked: &mut usize, unresolved: &mut Vec<PathBuf>| {
        let Some(path) = path else { return None };
        if path.is_file() {
            return Some(path.clone());
        }
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|n| n.to_lowercase());
        if let Some(found) = name.and_then(|n| index.get(&n).cloned()) {
            *relinked += 1;
            Some(found)
        } else {
            unresolved.push(path.clone());
            None
        }
    };

    for song in &mut project.songs {
        if let Some(found) = resolve(&song.audio, &mut report.relinked, &mut report.unresolved) {
            song.audio = Some(found);
        }
        if let Some(found) = resolve(&song.video, &mut report.relinked, &mut report.unresolved) {
            song.video = Some(found);
        }
    }
    let mut new_tracks = Vec::with_capacity(project.background.tracks.len());
    for track in &project.background.tracks {
        match resolve(
            &Some(track.clone()),
            &mut report.relinked,
            &mut report.unresolved,
        ) {
            Some(found) => new_tracks.push(found),
            None => new_tracks.push(track.clone()),
        }
    }
    project.background.tracks = new_tracks;
    report.unresolved.sort();
    report.unresolved.dedup();
    report
}

fn collect_files(dir: &Path, depth: usize, max_depth: usize, index: &mut HashMap<String, PathBuf>) {
    if depth > max_depth {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<_> = entries.flatten().collect();
    entries.sort_by_key(|e| e.path());
    for entry in entries {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') || name == "Library" || name == "System" {
            continue;
        }
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_dir() {
            collect_files(&path, depth + 1, max_depth, index);
        } else if file_type.is_file() {
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                let ext = ext.to_ascii_lowercase();
                if crate::library::AUDIO_EXTENSIONS.contains(&ext.as_str())
                    || crate::library::VIDEO_EXTENSIONS.contains(&ext.as_str())
                {
                    index
                        .entry(name.to_lowercase())
                        .or_insert_with(|| path.clone());
                }
            }
        }
    }
}

pub fn save_library(library: &Library, path: &Path) -> CoreResult<()> {
    save_json(library, path)
}

pub fn load_library(path: &Path) -> CoreResult<Library> {
    load_json(path)
}

pub fn save_preferences(prefs: &Preferences, path: &Path) -> CoreResult<()> {
    save_json(prefs, path)
}

pub fn load_preferences(path: &Path) -> CoreResult<Preferences> {
    let mut prefs: Preferences = load_json(path)?;
    prefs.clamp();
    Ok(prefs)
}

/// Writes a crash-recovery snapshot next to the preferences.
pub fn save_recovery(project: &Project, app_dir: &Path) -> CoreResult<PathBuf> {
    let path = app_dir.join(RECOVERY_FILE);
    save_project(project, &path)?;
    Ok(path)
}

/// Returns a recovery snapshot if one exists and parses.
pub fn load_recovery(app_dir: &Path) -> Option<Project> {
    let path = app_dir.join(RECOVERY_FILE);
    if !path.is_file() {
        return None;
    }
    match load_project(&path) {
        Ok(project) => Some(project),
        Err(err) => {
            warn!(error = %err, "ignoring unreadable recovery snapshot");
            None
        }
    }
}

pub fn clear_recovery(app_dir: &Path) {
    let path = app_dir.join(RECOVERY_FILE);
    if path.exists() {
        if let Err(err) = fs::remove_file(&path) {
            warn!(error = %err, "could not remove recovery snapshot");
        }
    }
}

/// Normalizes legacy/partial documents created by external tools.
pub fn normalize_loaded_project(project: &mut Project) {
    project.validate();
    if project.output.idle_screen == IdleScreen::Courtesy
        && project.output.courtesy_message.trim().is_empty()
    {
        project.output.courtesy_message = "Prossimo brano".to_string();
    }
    if project.background.resume_mode == ResumeMode::Resume {
        // no-op, documents the supported values
    }
    if project
        .songs
        .iter()
        .any(|s| matches!(s.status, SongStatus::Live))
    {
        project.reset_statuses();
    }
}

/// Adds the project extension when the user typed a bare name.
pub fn ensure_project_path(path: PathBuf) -> PathBuf {
    crate::model::with_project_extension(&path)
}

/// File name used for autosave when the document has never been saved.
pub fn untitled_project_path(app_dir: &Path) -> PathBuf {
    app_dir.join(format!("Senza titolo.{PROJECT_EXTENSION}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Song, SongStatus};
    use std::fs;

    fn project_with_media(audio: PathBuf) -> Project {
        let mut project = Project::new("Saggio di fine anno");
        project.venue = "Auditorium".to_string();
        let mut song = Song::new("Canzone", Some(audio), 123_456);
        song.artist = "Classe 3A".to_string();
        song.info.performers = "Orchestra".to_string();
        project.add_song(song);
        project
    }

    #[test]
    fn roundtrip_preserves_everything_that_matters() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("serata.nerdshow");
        let project = project_with_media(PathBuf::from("/tmp/audio.mp3"));
        save_project(&project, &path).unwrap();
        let loaded = load_project(&path).unwrap();
        assert_eq!(loaded.name, project.name);
        assert_eq!(loaded.songs.len(), 1);
        assert_eq!(loaded.songs[0].artist, "Classe 3A");
        assert_eq!(loaded.songs[0].duration_ms, 123_456);
        assert_eq!(loaded.title_template.name, project.title_template.name);
        assert!(!dir.path().join(".serata.nerdshow.tmp").exists());
    }

    #[test]
    fn newer_schema_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("future.nerdshow");
        let mut project = Project::new("Futuro");
        project.schema_version = CURRENT_SCHEMA_VERSION + 1;
        fs::write(&path, serde_json::to_vec(&project).unwrap()).unwrap();
        assert!(matches!(
            load_project(&path),
            Err(CoreError::UnsupportedSchema { .. })
        ));
    }

    #[test]
    fn missing_media_reports_audio_video_and_background() {
        let mut project = Project::new("Serata");
        let mut song = Song::new("Assente", Some(PathBuf::from("/nope/audio.wav")), 1_000);
        song.video = Some(PathBuf::from("/nope/video.mp4"));
        project.add_song(song);
        project
            .background
            .tracks
            .push(PathBuf::from("/nope/bg.flac"));
        let missing = missing_media(&project);
        assert_eq!(missing.len(), 3);
        assert!(missing
            .iter()
            .any(|m| m.role == MediaRole::SongAudio && m.song_id.is_some()));
        assert!(missing.iter().any(|m| m.role == MediaRole::Background));
    }

    #[test]
    fn relink_finds_moved_files_by_name() {
        let dir = tempfile::tempdir().unwrap();
        let new_root = dir.path().join("nuova_cartella");
        fs::create_dir_all(&new_root).unwrap();
        let audio = new_root.join("voce.wav");
        let video = new_root.join("clip.mp4");
        let bg = new_root.join("atmosfera.mp3");
        fs::write(&audio, b"x").unwrap();
        fs::write(&video, b"x").unwrap();
        fs::write(&bg, b"x").unwrap();

        let mut project = project_with_media(PathBuf::from("/vecchia/voce.wav"));
        project.songs[0].video = Some(PathBuf::from("/vecchia/clip.mp4"));
        project.background.tracks = vec![PathBuf::from("/vecchia/atmosfera.mp3")];

        let report = relink_media(&mut project, &new_root);
        assert_eq!(report.relinked, 3);
        assert!(report.unresolved.is_empty());
        assert_eq!(project.songs[0].audio.as_deref(), Some(audio.as_path()));
        assert_eq!(project.songs[0].video.as_deref(), Some(video.as_path()));
        assert_eq!(project.background.tracks[0], bg);
    }

    #[test]
    fn relink_leaves_unresolved_references_untouched() {
        let dir = tempfile::tempdir().unwrap();
        let mut project = project_with_media(PathBuf::from("/missing/voce.wav"));
        let report = relink_media(&mut project, dir.path());
        assert_eq!(report.relinked, 0);
        assert_eq!(report.unresolved.len(), 1);
        assert_eq!(
            project.songs[0].audio.as_deref(),
            Some(Path::new("/missing/voce.wav"))
        );
    }

    #[test]
    fn recovery_can_be_saved_loaded_and_cleared() {
        let dir = tempfile::tempdir().unwrap();
        let project = Project::new("Recovery");
        save_recovery(&project, dir.path()).unwrap();
        assert!(load_recovery(dir.path()).is_some());
        clear_recovery(dir.path());
        assert!(load_recovery(dir.path()).is_none());
    }

    #[test]
    fn statuses_are_preserved_in_project_files() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("show.nerdshow");
        let mut project = project_with_media(PathBuf::from("/tmp/a.wav"));
        project.songs[0].status = SongStatus::Done;
        save_project(&project, &path).unwrap();
        let loaded = load_project(&path).unwrap();
        assert_eq!(loaded.songs[0].status, SongStatus::Done);
    }
}
