//! The evening (project) data model.
//!
//! A project is a single JSON document with a schema version. Everything that
//! describes a concert evening lives here: the running order, the background
//! music configuration, the video output and the title template.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::branding::{CURRENT_SCHEMA_VERSION, PROJECT_EXTENSION};

/// Stable identifier for new songs.
pub fn new_id() -> String {
    Uuid::new_v4().to_string()
}

/// How the show advances between songs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PlaybackMode {
    /// Every song needs an explicit GO.
    #[default]
    Manual,
    /// After the background gap the next song starts by itself.
    Automatic,
}

/// Lifecycle of a song inside the current evening.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SongStatus {
    #[default]
    Pending,
    Live,
    Done,
    Skipped,
}

impl SongStatus {
    pub fn is_finished(self) -> bool {
        matches!(self, Self::Done | Self::Skipped)
    }
}

/// Vertical placement of the title card over the video.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TitlePosition {
    Upper,
    Center,
    #[default]
    Lower,
}

/// Entrance/exit animation of the title card.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TitleAnimation {
    None,
    Fade,
    #[default]
    SlideUp,
    Scale,
}

/// What the projector shows when no video is playing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum IdleScreen {
    Black,
    #[default]
    Courtesy,
}

/// Background playback memory between songs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ResumeMode {
    /// Continue from where the background stopped.
    #[default]
    Resume,
    /// Restart the playlist from the first track.
    Restart,
}

/// Free-form information shown in the Sanremo-style title card.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct TitleInfo {
    /// Overrides [`crate::model::Song::title`] when non-empty.
    pub title: String,
    /// Overrides [`crate::model::Song::artist`] when non-empty.
    pub artist: String,
    /// Authors / composers, one line.
    pub authors: String,
    /// Performers or class, one line.
    pub performers: String,
    /// Additional free lines, shown in order.
    pub extra_lines: Vec<String>,
}

impl TitleInfo {
    pub fn is_empty(&self) -> bool {
        self.title.is_empty()
            && self.artist.is_empty()
            && self.authors.is_empty()
            && self.performers.is_empty()
            && self.extra_lines.iter().all(|l| l.trim().is_empty())
    }

    /// Returns the lines that should actually be rendered, falling back to the
    /// song title and artist.
    pub fn resolved_lines(&self, fallback_title: &str, fallback_artist: &str) -> Vec<TitleLine> {
        let mut out = Vec::new();
        let title = if self.title.trim().is_empty() {
            fallback_title
        } else {
            self.title.trim()
        };
        if !title.is_empty() {
            out.push(TitleLine::Title(title.to_string()));
        }
        let artist = if self.artist.trim().is_empty() {
            fallback_artist
        } else {
            self.artist.trim()
        };
        if !artist.is_empty() {
            out.push(TitleLine::Artist(artist.to_string()));
        }
        if !self.authors.trim().is_empty() {
            out.push(TitleLine::Meta(self.authors.trim().to_string()));
        }
        if !self.performers.trim().is_empty() {
            out.push(TitleLine::Meta(self.performers.trim().to_string()));
        }
        for line in &self.extra_lines {
            if !line.trim().is_empty() {
                out.push(TitleLine::Meta(line.trim().to_string()));
            }
        }
        out
    }
}

/// A styled line of the title card.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TitleLine {
    Title(String),
    Artist(String),
    Meta(String),
}

/// Visual template for the title overlay. All sizes are relative to the output
/// height so preview and projector render identically at any resolution.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TitleTemplate {
    pub name: String,
    /// Title font size as a percentage of the output height.
    pub title_size_pct: f32,
    /// Secondary text size as a percentage of the output height.
    pub body_size_pct: f32,
    pub title_color: [u8; 4],
    pub text_color: [u8; 4],
    pub accent_color: [u8; 4],
    pub position: TitlePosition,
    pub panel: bool,
    pub panel_opacity: f32,
    pub animation: TitleAnimation,
    pub delay_ms: u64,
    /// `0` keeps the card visible for the whole song.
    pub duration_ms: u64,
}

impl Default for TitleTemplate {
    fn default() -> Self {
        Self {
            name: "Intestazione classica".to_string(),
            title_size_pct: 5.2,
            body_size_pct: 2.3,
            title_color: [255, 255, 255, 255],
            text_color: [232, 234, 240, 255],
            accent_color: [64, 214, 190, 255],
            position: TitlePosition::Lower,
            panel: true,
            panel_opacity: 0.55,
            animation: TitleAnimation::SlideUp,
            delay_ms: 250,
            duration_ms: 8500,
        }
    }
}

impl TitleTemplate {
    pub fn validate(&mut self) {
        self.title_size_pct = self.title_size_pct.clamp(1.5, 18.0);
        self.body_size_pct = self.body_size_pct.clamp(1.0, 10.0);
        self.panel_opacity = self.panel_opacity.clamp(0.0, 1.0);
        self.delay_ms = self.delay_ms.min(60_000);
        self.duration_ms = self.duration_ms.min(30 * 60_000);
    }
}

/// One song of the running order.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Song {
    pub id: String,
    pub title: String,
    pub artist: String,
    /// Decoded by the library on import; `None` while pending.
    pub audio: Option<PathBuf>,
    /// Optional music video. When present the video soundtrack is used for the
    /// audio bus as well, so audio and picture cannot drift.
    pub video: Option<PathBuf>,
    /// Full media duration in milliseconds.
    pub duration_ms: u64,
    /// In point inside the media file.
    pub start_ms: u64,
    /// Out point inside the media file. `None` means the end of the file.
    pub end_ms: Option<u64>,
    pub volume: f32,
    pub fade_in_ms: u64,
    pub fade_out_ms: u64,
    pub notes: String,
    pub status: SongStatus,
    /// Pause after this song; `None` uses the global setting.
    pub gap_after_ms: Option<u64>,
    pub info: TitleInfo,
}

impl Default for Song {
    fn default() -> Self {
        Self {
            id: new_id(),
            title: String::new(),
            artist: String::new(),
            audio: None,
            video: None,
            duration_ms: 0,
            start_ms: 0,
            end_ms: None,
            volume: 1.0,
            fade_in_ms: 350,
            fade_out_ms: 700,
            notes: String::new(),
            status: SongStatus::Pending,
            gap_after_ms: None,
            info: TitleInfo::default(),
        }
    }
}

impl Song {
    pub fn new(title: impl Into<String>, audio: Option<PathBuf>, duration_ms: u64) -> Self {
        Self {
            title: title.into(),
            audio,
            duration_ms,
            ..Self::default()
        }
    }

    pub fn effective_start_ms(&self) -> u64 {
        self.start_ms.min(self.duration_ms)
    }

    pub fn effective_end_ms(&self) -> u64 {
        self.end_ms
            .unwrap_or(self.duration_ms)
            .clamp(self.effective_start_ms(), self.duration_ms)
    }

    pub fn effective_duration_ms(&self) -> u64 {
        self.effective_end_ms()
            .saturating_sub(self.effective_start_ms())
    }

    /// A song is playable when it has an audio source. Video-only assets are
    /// imported as audio+video, so a missing audio file is always blocking.
    pub fn is_playable(&self) -> bool {
        self.audio.as_ref().is_some_and(|p| p.is_file())
    }

    pub fn has_video(&self) -> bool {
        self.video.as_ref().is_some_and(|p| p.is_file())
    }

    pub fn display_title(&self) -> String {
        if !self.title.trim().is_empty() {
            return self.title.trim().to_string();
        }
        if let Some(path) = &self.audio {
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                return stem.to_string();
            }
        }
        "Senza titolo".to_string()
    }

    pub fn validate(&mut self) {
        if self.id.trim().is_empty() {
            self.id = new_id();
        }
        self.volume = self.volume.clamp(0.0, 2.0);
        self.fade_in_ms = self.fade_in_ms.min(60_000);
        self.fade_out_ms = self.fade_out_ms.min(60_000);
        if self.start_ms > self.duration_ms {
            self.start_ms = self.duration_ms;
        }
        if let Some(end) = self.end_ms {
            self.end_ms = Some(end.clamp(self.start_ms, self.duration_ms.max(self.start_ms)));
        }
        if self.title.trim().is_empty() {
            self.title = self.display_title();
        }
    }
}

/// Background music configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct BackgroundSettings {
    pub tracks: Vec<PathBuf>,
    pub volume: f32,
    pub looping: bool,
    pub shuffle: bool,
    pub fade_in_ms: u64,
    pub fade_out_ms: u64,
    pub resume_mode: ResumeMode,
}

impl Default for BackgroundSettings {
    fn default() -> Self {
        Self {
            tracks: Vec::new(),
            volume: 0.32,
            looping: true,
            shuffle: false,
            fade_in_ms: 1800,
            fade_out_ms: 2200,
            resume_mode: ResumeMode::Resume,
        }
    }
}

impl BackgroundSettings {
    pub fn validate(&mut self) {
        self.volume = self.volume.clamp(0.0, 2.0);
        self.fade_in_ms = self.fade_in_ms.min(30_000);
        self.fade_out_ms = self.fade_out_ms.min(30_000);
    }
}

/// Projector / video output configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct OutputSettings {
    /// Index in the macOS display list. `0` is normally the built-in display.
    pub display_index: usize,
    pub video_enabled: bool,
    pub show_titles: bool,
    pub idle_screen: IdleScreen,
    pub courtesy_message: String,
    pub logo_path: Option<PathBuf>,
    pub blackout: bool,
}

impl Default for OutputSettings {
    fn default() -> Self {
        Self {
            display_index: 0,
            video_enabled: true,
            show_titles: true,
            idle_screen: IdleScreen::Courtesy,
            courtesy_message: "Prossimo brano".to_string(),
            logo_path: None,
            blackout: false,
        }
    }
}

/// Show-wide timing settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ShowSettings {
    pub mode: PlaybackMode,
    /// Pause between songs in automatic mode.
    pub default_gap_ms: u64,
    /// Fast fade used by Panic.
    pub panic_fade_ms: u64,
}

impl Default for ShowSettings {
    fn default() -> Self {
        Self {
            mode: PlaybackMode::Manual,
            default_gap_ms: 8000,
            panic_fade_ms: 180,
        }
    }
}

/// A complete evening.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Project {
    pub schema_version: u32,
    pub name: String,
    pub venue: String,
    /// Free-form date string, e.g. `2026-11-07`.
    pub date: String,
    pub notes: String,
    pub show: ShowSettings,
    pub background: BackgroundSettings,
    pub output: OutputSettings,
    pub title_template: TitleTemplate,
    pub songs: Vec<Song>,
}

impl Default for Project {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_SCHEMA_VERSION,
            name: "Nuova serata".to_string(),
            venue: String::new(),
            date: String::new(),
            notes: String::new(),
            show: ShowSettings::default(),
            background: BackgroundSettings::default(),
            output: OutputSettings::default(),
            title_template: TitleTemplate::default(),
            songs: Vec::new(),
        }
    }
}

impl Project {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Self::default()
        }
    }

    pub fn add_song(&mut self, song: Song) -> usize {
        self.songs.push(song);
        self.songs.len() - 1
    }

    pub fn insert_song(&mut self, index: usize, song: Song) {
        let index = index.min(self.songs.len());
        self.songs.insert(index, song);
    }

    pub fn remove_song(&mut self, index: usize) -> Option<Song> {
        if index < self.songs.len() {
            Some(self.songs.remove(index))
        } else {
            None
        }
    }

    /// Moves a song, clamping both indices. Returns whether anything changed.
    pub fn move_song(&mut self, from: usize, to: usize) -> bool {
        if self.songs.is_empty() || from >= self.songs.len() || from == to {
            return false;
        }
        let to = to.min(self.songs.len() - 1);
        let song = self.songs.remove(from);
        self.songs.insert(to, song);
        true
    }

    pub fn find_song(&self, id: &str) -> Option<&Song> {
        self.songs.iter().find(|s| s.id == id)
    }

    pub fn find_song_mut(&mut self, id: &str) -> Option<&mut Song> {
        self.songs.iter_mut().find(|s| s.id == id)
    }

    pub fn total_duration_ms(&self) -> u64 {
        self.songs.iter().map(Song::effective_duration_ms).sum()
    }

    pub fn reset_statuses(&mut self) {
        for song in &mut self.songs {
            song.status = SongStatus::Pending;
        }
    }

    /// Copies the show machine statuses back into the project (matched by id).
    pub fn apply_statuses(&mut self, statuses: &[(String, SongStatus)]) {
        for (id, status) in statuses {
            if let Some(song) = self.find_song_mut(id) {
                song.status = *status;
            }
        }
    }

    pub fn validate(&mut self) {
        self.schema_version = CURRENT_SCHEMA_VERSION;
        if self.name.trim().is_empty() {
            self.name = "Nuova serata".to_string();
        }
        self.background.validate();
        self.title_template.validate();
        self.show.default_gap_ms = self.show.default_gap_ms.min(10 * 60_000);
        self.show.panic_fade_ms = self.show.panic_fade_ms.clamp(0, 5_000);
        for song in &mut self.songs {
            song.validate();
        }
    }

    /// Suggested file name for "Save As".
    pub fn suggested_file_name(&self) -> String {
        let mut base = self.name.trim().to_string();
        if base.is_empty() {
            base = "Serata".to_string();
        }
        let safe: String = base
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '-' || c == '_' || c == ' ' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        format!("{}.{}", safe.trim(), PROJECT_EXTENSION)
    }
}

/// Ensures a path ends with the project extension.
pub fn with_project_extension(path: &Path) -> PathBuf {
    if path
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case(PROJECT_EXTENSION))
    {
        path.to_path_buf()
    } else {
        path.with_extension(PROJECT_EXTENSION)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trim_is_clamped_and_reversed_points_are_fixed() {
        let mut song = Song::new("Test", None, 10_000);
        song.start_ms = 2_000;
        song.end_ms = Some(9_000);
        assert_eq!(song.effective_duration_ms(), 7_000);

        song.end_ms = Some(1_000);
        assert_eq!(song.effective_duration_ms(), 0);
        assert_eq!(song.effective_end_ms(), 2_000);
    }

    #[test]
    fn moving_songs_clamps_destination() {
        let mut project = Project::new("Serata");
        for i in 0..3 {
            project.add_song(Song::new(format!("Brano {i}"), None, 1_000));
        }
        assert!(project.move_song(0, 99));
        assert_eq!(project.songs[2].title, "Brano 0");
        assert!(!project.move_song(5, 1));
    }

    #[test]
    fn title_lines_fall_back_to_song_fields() {
        let info = TitleInfo::default();
        let lines = info.resolved_lines("Titolo", "Artista");
        assert_eq!(
            lines,
            vec![
                TitleLine::Title("Titolo".into()),
                TitleLine::Artist("Artista".into())
            ]
        );
    }

    #[test]
    fn applying_statuses_matches_by_id() {
        let mut project = Project::new("Serata");
        let song = Song::new("Uno", None, 1_000);
        let id = song.id.clone();
        project.add_song(song);
        project.apply_statuses(&[(id, SongStatus::Done)]);
        assert_eq!(project.songs[0].status, SongStatus::Done);
    }
}
