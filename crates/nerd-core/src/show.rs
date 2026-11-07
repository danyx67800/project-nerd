//! Deterministic show state machine.
//!
//! The machine owns no clock and performs no I/O. The caller feeds it elapsed
//! milliseconds and forwards the returned [`Effect`]s to the audio engine. This
//! keeps every edge case (manual/automatic transitions, suspensions, fade
//! stops, skips, panics) unit-testable without hardware.

use crate::model::{PlaybackMode, Project, Song, SongStatus};

/// Visible phase of the evening.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// Nothing is playing. Background may still be running.
    Idle,
    /// A song is on air.
    Playing,
    /// A song is on air but paused.
    Paused,
    /// A fade-out is running; the machine is neither playing nor waiting.
    Stopping,
    /// Background music while waiting for the next GO / countdown.
    Waiting,
    /// The running order is over; background keeps playing until Stop.
    Finished,
    /// Panic fade in progress.
    Panic,
}

/// A single cue the machine can schedule.
#[derive(Debug, Clone)]
pub struct Cue {
    pub song_id: String,
    pub title: String,
    pub duration_ms: u64,
    pub fade_in_ms: u64,
    pub fade_out_ms: u64,
    /// Pause after this cue; `None` uses the global gap.
    pub gap_after_ms: Option<u64>,
    pub status: SongStatus,
    pub has_video: bool,
}

impl Cue {
    pub fn from_song(song: &Song) -> Self {
        Self {
            song_id: song.id.clone(),
            title: song.display_title(),
            duration_ms: song.effective_duration_ms(),
            fade_in_ms: song.fade_in_ms,
            fade_out_ms: song.fade_out_ms,
            gap_after_ms: song.gap_after_ms,
            status: song.status,
            has_video: song.has_video(),
        }
    }

    /// Test helper.
    pub fn new(title: &str, duration_ms: u64) -> Self {
        Self {
            song_id: title.to_string(),
            title: title.to_string(),
            duration_ms,
            fade_in_ms: 300,
            fade_out_ms: 700,
            gap_after_ms: None,
            status: SongStatus::Pending,
            has_video: false,
        }
    }
}

/// Timing settings mirrored from the project.
#[derive(Debug, Clone, Copy)]
pub struct ShowConfig {
    pub mode: PlaybackMode,
    pub default_gap_ms: u64,
    pub panic_fade_ms: u64,
    pub background_fade_in_ms: u64,
    pub background_fade_out_ms: u64,
}

impl Default for ShowConfig {
    fn default() -> Self {
        Self {
            mode: PlaybackMode::Manual,
            default_gap_ms: 8_000,
            panic_fade_ms: 180,
            background_fade_in_ms: 1_800,
            background_fade_out_ms: 2_200,
        }
    }
}

impl ShowConfig {
    pub fn from_project(project: &Project) -> Self {
        Self {
            mode: project.show.mode,
            default_gap_ms: project.show.default_gap_ms,
            panic_fade_ms: project.show.panic_fade_ms,
            background_fade_in_ms: project.background.fade_in_ms,
            background_fade_out_ms: project.background.fade_out_ms,
        }
    }
}

/// Side effects the audio engine (or the UI) must apply.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Effect {
    /// Start a cue. The engine crossfades from any song still on air.
    PlaySong {
        index: usize,
    },
    StopSong {
        fade_ms: u64,
    },
    StartBackground {
        fade_in_ms: u64,
    },
    StopBackground {
        fade_ms: u64,
    },
    PauseSong,
    ResumeSong,
    PauseBackground,
    ResumeBackground,
}

/// Answer to a GO press.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GoOutcome {
    Started(Vec<Effect>),
    /// A song is on air: the UI must ask for confirmation.
    ConfirmRequired,
    Ignored,
}

/// The show machine.
#[derive(Debug, Clone)]
pub struct Show {
    cues: Vec<Cue>,
    config: ShowConfig,
    phase: Phase,
    live: Option<usize>,
    last_live: Option<usize>,
    cursor: Option<usize>,
    elapsed_ms: u64,
    gap_remaining_ms: Option<u64>,
    auto_suspended: bool,
    bg_running: bool,
    bg_paused: bool,
    hold_for_go: bool,
    closing_ms: u64,
}

impl Show {
    pub fn new(cues: Vec<Cue>, config: ShowConfig) -> Self {
        let mut cues = cues;
        for cue in &mut cues {
            if matches!(cue.status, SongStatus::Live) {
                cue.status = SongStatus::Pending;
            }
        }
        let cursor = cues
            .iter()
            .position(|c| matches!(c.status, SongStatus::Pending));
        Self {
            cues,
            config,
            phase: Phase::Idle,
            live: None,
            last_live: None,
            cursor,
            elapsed_ms: 0,
            gap_remaining_ms: None,
            auto_suspended: false,
            bg_running: false,
            bg_paused: false,
            hold_for_go: false,
            closing_ms: 0,
        }
    }

    pub fn from_project(project: &Project) -> Self {
        Self::new(
            project.songs.iter().map(Cue::from_song).collect(),
            ShowConfig::from_project(project),
        )
    }

    pub fn empty() -> Self {
        Self::new(Vec::new(), ShowConfig::default())
    }

    // ---------------------------------------------------------------------
    // Read-only views
    // ---------------------------------------------------------------------

    pub fn phase(&self) -> Phase {
        self.phase
    }

    pub fn is_empty(&self) -> bool {
        self.cues.is_empty()
    }

    pub fn cues(&self) -> &[Cue] {
        &self.cues
    }

    pub fn cue(&self, index: usize) -> Option<&Cue> {
        self.cues.get(index)
    }

    pub fn live_index(&self) -> Option<usize> {
        self.live
    }

    pub fn last_live_index(&self) -> Option<usize> {
        self.last_live
    }

    pub fn cursor_index(&self) -> Option<usize> {
        self.cursor
    }

    pub fn elapsed_ms(&self) -> u64 {
        self.elapsed_ms
    }

    pub fn live_duration_ms(&self) -> u64 {
        self.live
            .and_then(|i| self.cues.get(i))
            .map(|c| c.duration_ms)
            .unwrap_or(0)
    }

    pub fn remaining_ms(&self) -> u64 {
        self.live_duration_ms().saturating_sub(self.elapsed_ms)
    }

    pub fn progress_fraction(&self) -> f32 {
        let total = self.live_duration_ms();
        if total == 0 {
            0.0
        } else {
            (self.elapsed_ms as f32 / total as f32).clamp(0.0, 1.0)
        }
    }

    pub fn gap_remaining_ms(&self) -> Option<u64> {
        self.gap_remaining_ms
    }

    pub fn is_counting_down(&self) -> bool {
        self.phase == Phase::Waiting
            && self.config.mode == PlaybackMode::Automatic
            && !self.auto_suspended
            && !self.hold_for_go
            && !self.bg_paused
            && self.gap_remaining_ms.is_some_and(|g| g > 0)
    }

    pub fn auto_suspended(&self) -> bool {
        self.auto_suspended
    }

    pub fn background_running(&self) -> bool {
        self.bg_running
    }

    pub fn background_paused(&self) -> bool {
        self.bg_paused
    }

    pub fn hold_for_go(&self) -> bool {
        self.hold_for_go
    }

    pub fn is_paused(&self) -> bool {
        self.phase == Phase::Paused
    }

    pub fn mode(&self) -> PlaybackMode {
        self.config.mode
    }

    pub fn config(&self) -> ShowConfig {
        self.config
    }

    pub fn statuses(&self) -> Vec<(String, SongStatus)> {
        self.cues
            .iter()
            .map(|c| (c.song_id.clone(), c.status))
            .collect()
    }

    pub fn status_of(&self, index: usize) -> Option<SongStatus> {
        self.cues.get(index).map(|c| c.status)
    }

    /// Copies the live statuses back into the project. Statuses are show state,
    /// not undoable edits.
    pub fn sync_statuses(&self, project: &mut Project) {
        project.apply_statuses(&self.statuses());
    }

    // ---------------------------------------------------------------------
    // Commands
    // ---------------------------------------------------------------------

    /// GO. In manual mode starts the selected song; in automatic it interrupts
    /// a countdown. During a song it asks for confirmation.
    pub fn go(&mut self) -> GoOutcome {
        match self.phase {
            Phase::Playing | Phase::Paused => GoOutcome::ConfirmRequired,
            Phase::Stopping | Phase::Panic => GoOutcome::Ignored,
            Phase::Finished => {
                if self.cues.is_empty() {
                    return GoOutcome::Ignored;
                }
                // Encore: the same running order starts over.
                for cue in &mut self.cues {
                    cue.status = SongStatus::Pending;
                }
                self.cursor = Some(0);
                GoOutcome::Started(self.start_cue(0))
            }
            Phase::Idle | Phase::Waiting => match self.cursor {
                Some(index) => GoOutcome::Started(self.start_cue(index)),
                None => GoOutcome::Ignored,
            },
        }
    }

    /// Confirms the "start the next song?" dialog. This is an immediate,
    /// crossfaded skip rather than a gap.
    pub fn confirm_go(&mut self) -> Vec<Effect> {
        if matches!(self.phase, Phase::Playing | Phase::Paused) {
            self.finish_current(SongStatus::Skipped, true, true)
        } else if let GoOutcome::Started(effects) = self.go() {
            effects
        } else {
            Vec::new()
        }
    }

    /// Skip: ends the current song; behaviour matches a natural end (manual
    /// waits, automatic shows the gap), except that skipping a countdown starts
    /// the next song immediately.
    pub fn skip(&mut self) -> Vec<Effect> {
        match self.phase {
            Phase::Playing | Phase::Paused => self.finish_current(SongStatus::Skipped, true, false),
            Phase::Waiting => match self.cursor {
                Some(index) => self.start_cue(index),
                None => Vec::new(),
            },
            _ => Vec::new(),
        }
    }

    /// Back: stops the current song and selects the previous one, waiting for
    /// an explicit GO even in automatic mode.
    pub fn previous(&mut self) -> Vec<Effect> {
        if self.cues.is_empty() {
            return Vec::new();
        }
        let base = self.live.or(self.cursor).unwrap_or(0);
        let target = base.saturating_sub(1);
        let mut effects = Vec::new();
        let was_active = self.live.is_some()
            || matches!(
                self.phase,
                Phase::Playing | Phase::Paused | Phase::Waiting | Phase::Finished
            );

        if let Some(index) = self.live {
            self.cues[index].status = SongStatus::Skipped;
            self.last_live = Some(index);
            effects.push(Effect::StopSong { fade_ms: 250 });
        }

        self.cues[target].status = SongStatus::Pending;
        self.cursor = Some(target);
        self.live = None;
        self.elapsed_ms = 0;
        self.gap_remaining_ms = None;

        if was_active {
            effects.extend(self.ensure_background());
            self.phase = Phase::Waiting;
            self.hold_for_go = true;
        } else {
            self.phase = Phase::Idle;
            self.hold_for_go = false;
        }
        effects
    }

    /// Stop with fade. Marks an interrupted song as skipped, stops background
    /// and ends the current run; GO resumes from the next pending song.
    pub fn stop(&mut self, fade_ms: u64) -> Vec<Effect> {
        if matches!(self.phase, Phase::Stopping | Phase::Panic) {
            return Vec::new();
        }
        let had_song = self.live.is_some();
        let had_bg = self.bg_running;
        if !had_song && !had_bg {
            self.phase = Phase::Idle;
            self.gap_remaining_ms = None;
            self.hold_for_go = false;
            return Vec::new();
        }
        if let Some(index) = self.live {
            self.cues[index].status = SongStatus::Skipped;
            self.last_live = Some(index);
        }
        self.live = None;
        self.elapsed_ms = 0;
        self.bg_running = false;
        self.bg_paused = false;
        self.gap_remaining_ms = None;
        self.hold_for_go = false;

        let mut effects = Vec::new();
        if had_song {
            effects.push(Effect::StopSong { fade_ms });
        }
        if had_bg {
            effects.push(Effect::StopBackground { fade_ms });
        }
        if fade_ms == 0 {
            self.phase = Phase::Idle;
        } else {
            self.phase = Phase::Stopping;
            self.closing_ms = fade_ms;
        }
        effects
    }

    /// Panic: fastest safe stop of every bus. The current song is marked as
    /// skipped and GO can restart from the next pending song.
    pub fn panic(&mut self) -> Vec<Effect> {
        if matches!(self.phase, Phase::Stopping | Phase::Panic) {
            return Vec::new();
        }
        let fade = self.config.panic_fade_ms;
        let had_song = self.live.is_some();
        let had_bg = self.bg_running;
        if !had_song && !had_bg {
            self.phase = Phase::Idle;
            return Vec::new();
        }
        if let Some(index) = self.live {
            self.cues[index].status = SongStatus::Skipped;
            self.last_live = Some(index);
        }
        self.live = None;
        self.bg_running = false;
        self.bg_paused = false;
        self.gap_remaining_ms = None;
        self.hold_for_go = false;
        self.elapsed_ms = 0;

        let mut effects = Vec::new();
        if had_song {
            effects.push(Effect::StopSong { fade_ms: fade });
        }
        if had_bg {
            effects.push(Effect::StopBackground { fade_ms: fade });
        }
        if fade == 0 {
            self.phase = Phase::Idle;
        } else {
            self.phase = Phase::Panic;
            self.closing_ms = fade;
        }
        effects
    }

    /// Pause the song, or the background when no song is on air.
    pub fn pause(&mut self) -> Vec<Effect> {
        match self.phase {
            Phase::Playing => {
                self.phase = Phase::Paused;
                vec![Effect::PauseSong]
            }
            Phase::Paused => Vec::new(),
            Phase::Stopping | Phase::Panic => Vec::new(),
            _ if self.bg_running && !self.bg_paused => {
                self.bg_paused = true;
                vec![Effect::PauseBackground]
            }
            _ => Vec::new(),
        }
    }

    pub fn resume(&mut self) -> Vec<Effect> {
        if self.phase == Phase::Paused {
            self.phase = Phase::Playing;
            return vec![Effect::ResumeSong];
        }
        if self.bg_paused {
            self.bg_paused = false;
            return vec![Effect::ResumeBackground];
        }
        Vec::new()
    }

    pub fn toggle_pause(&mut self) -> Vec<Effect> {
        if self.phase == Phase::Paused || self.bg_paused {
            self.resume()
        } else {
            self.pause()
        }
    }

    pub fn set_mode(&mut self, mode: PlaybackMode) {
        self.config.mode = mode;
        if mode == PlaybackMode::Automatic
            && self.phase == Phase::Waiting
            && self.gap_remaining_ms.is_none()
        {
            self.gap_remaining_ms = Some(self.config.default_gap_ms);
        }
    }

    pub fn set_auto_suspended(&mut self, suspended: bool) {
        self.auto_suspended = suspended;
    }

    pub fn toggle_auto_suspend(&mut self) {
        self.auto_suspended = !self.auto_suspended;
    }

    /// Manually starts background music (pre-show, or after Stop).
    pub fn start_background(&mut self) -> Vec<Effect> {
        if matches!(self.phase, Phase::Playing | Phase::Stopping | Phase::Panic) {
            return Vec::new();
        }
        if self.bg_running {
            if self.bg_paused {
                self.bg_paused = false;
                return vec![Effect::ResumeBackground];
            }
            return Vec::new();
        }
        self.bg_running = true;
        self.bg_paused = false;
        vec![Effect::StartBackground {
            fade_in_ms: self.config.background_fade_in_ms,
        }]
    }

    pub fn stop_background(&mut self) -> Vec<Effect> {
        if !self.bg_running {
            return Vec::new();
        }
        self.bg_running = false;
        self.bg_paused = false;
        vec![Effect::StopBackground {
            fade_ms: self.config.background_fade_out_ms,
        }]
    }

    /// Resets the entire run. Useful for "Azzera stati" and for a fresh load.
    pub fn reset(&mut self) -> Vec<Effect> {
        let mut effects = Vec::new();
        if self.live.is_some() {
            effects.push(Effect::StopSong { fade_ms: 250 });
        }
        if self.bg_running {
            effects.push(Effect::StopBackground {
                fade_ms: self.config.background_fade_out_ms,
            });
        }
        for cue in &mut self.cues {
            cue.status = SongStatus::Pending;
        }
        self.phase = Phase::Idle;
        self.live = None;
        self.last_live = None;
        self.elapsed_ms = 0;
        self.gap_remaining_ms = None;
        self.auto_suspended = false;
        self.bg_running = false;
        self.bg_paused = false;
        self.hold_for_go = false;
        self.closing_ms = 0;
        self.cursor = if self.cues.is_empty() { None } else { Some(0) };
        effects
    }

    /// Rebuilds the cue list and timing after an edit. Playback stops; the
    /// caller is expected to announce the change to the operator.
    pub fn reconfigure(&mut self, cues: Vec<Cue>, config: ShowConfig) {
        self.cues = cues;
        self.config = config;
        self.phase = Phase::Idle;
        self.live = None;
        self.last_live = None;
        self.elapsed_ms = 0;
        self.gap_remaining_ms = None;
        self.bg_running = false;
        self.bg_paused = false;
        self.hold_for_go = false;
        self.closing_ms = 0;
        self.cursor = self
            .cues
            .iter()
            .position(|c| matches!(c.status, SongStatus::Pending));
    }

    /// Advances time. `dt_ms` may be large after a stall; the machine clamps
    /// internally by processing phases with saturating arithmetic.
    pub fn tick(&mut self, dt_ms: u64) -> Vec<Effect> {
        if dt_ms == 0 {
            return Vec::new();
        }
        match self.phase {
            Phase::Playing => {
                let Some(index) = self.live else {
                    return Vec::new();
                };
                let duration = self.cues[index].duration_ms;
                self.elapsed_ms = self.elapsed_ms.saturating_add(dt_ms);
                if self.elapsed_ms >= duration {
                    self.elapsed_ms = duration;
                    self.finish_current(SongStatus::Done, false, false)
                } else {
                    Vec::new()
                }
            }
            Phase::Paused => Vec::new(),
            Phase::Waiting => {
                if !self.can_count_down() {
                    return Vec::new();
                }
                match self.gap_remaining_ms {
                    Some(0) => match self.cursor {
                        Some(index) => self.start_cue(index),
                        None => {
                            self.phase = Phase::Finished;
                            Vec::new()
                        }
                    },
                    Some(remaining) => {
                        let remaining = remaining.saturating_sub(dt_ms);
                        self.gap_remaining_ms = Some(remaining);
                        if remaining == 0 {
                            match self.cursor {
                                Some(index) => self.start_cue(index),
                                None => {
                                    self.phase = Phase::Finished;
                                    Vec::new()
                                }
                            }
                        } else {
                            Vec::new()
                        }
                    }
                    None => Vec::new(),
                }
            }
            Phase::Stopping | Phase::Panic => {
                self.closing_ms = self.closing_ms.saturating_sub(dt_ms);
                if self.closing_ms == 0 {
                    self.phase = Phase::Idle;
                }
                Vec::new()
            }
            Phase::Idle | Phase::Finished => Vec::new(),
        }
    }

    // ---------------------------------------------------------------------
    // Internals
    // ---------------------------------------------------------------------

    fn can_count_down(&self) -> bool {
        self.phase == Phase::Waiting
            && self.config.mode == PlaybackMode::Automatic
            && !self.auto_suspended
            && !self.hold_for_go
            && !self.bg_paused
    }

    fn next_pending_after(&self, index: usize) -> Option<usize> {
        self.cues
            .iter()
            .enumerate()
            .skip(index + 1)
            .find(|(_, cue)| matches!(cue.status, SongStatus::Pending))
            .map(|(i, _)| i)
    }

    fn gap_after(&self, index: Option<usize>) -> u64 {
        index
            .and_then(|i| self.cues.get(i))
            .and_then(|cue| cue.gap_after_ms)
            .unwrap_or(self.config.default_gap_ms)
    }

    fn ensure_background(&mut self) -> Vec<Effect> {
        if !self.bg_running {
            self.bg_running = true;
            self.bg_paused = false;
            vec![Effect::StartBackground {
                fade_in_ms: self.config.background_fade_in_ms,
            }]
        } else if self.bg_paused {
            self.bg_paused = false;
            vec![Effect::ResumeBackground]
        } else {
            Vec::new()
        }
    }

    fn start_cue(&mut self, index: usize) -> Vec<Effect> {
        if index >= self.cues.len() {
            return Vec::new();
        }
        let mut effects = Vec::new();
        if self.bg_running {
            effects.push(Effect::StopBackground {
                fade_ms: self.config.background_fade_out_ms,
            });
            self.bg_running = false;
        }
        self.bg_paused = false;
        self.cues[index].status = SongStatus::Live;
        self.live = Some(index);
        self.last_live = Some(index);
        self.cursor = self.next_pending_after(index);
        self.elapsed_ms = 0;
        self.gap_remaining_ms = None;
        self.hold_for_go = false;
        self.phase = Phase::Playing;
        effects.push(Effect::PlaySong { index });
        effects
    }

    /// Common path for natural end, skip and confirmed GO.
    fn finish_current(
        &mut self,
        status: SongStatus,
        interrupted: bool,
        force_next: bool,
    ) -> Vec<Effect> {
        let ended = self.live;
        if let Some(index) = ended {
            self.cues[index].status = status;
            self.last_live = Some(index);
        }
        let gap = self.gap_after(ended);
        self.live = None;
        self.elapsed_ms = 0;
        self.phase = Phase::Idle;

        let next = self.cursor;
        let auto_ready = self.config.mode == PlaybackMode::Automatic
            && !self.auto_suspended
            && !self.hold_for_go;

        let mut effects = Vec::new();
        match next {
            None => {
                if interrupted {
                    effects.push(Effect::StopSong { fade_ms: 250 });
                }
                effects.extend(self.ensure_background());
                self.phase = Phase::Finished;
            }
            Some(index) if force_next || (auto_ready && gap == 0) => {
                // The engine crossfades from the interrupted song, so no
                // StopSong is emitted here.
                effects.extend(self.start_cue(index));
            }
            Some(_) => {
                if interrupted {
                    effects.push(Effect::StopSong { fade_ms: 250 });
                }
                effects.extend(self.ensure_background());
                self.phase = Phase::Waiting;
                self.gap_remaining_ms = Some(gap);
            }
        }
        self.hold_for_go = false;
        effects
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cues(count: usize, duration_ms: u64) -> Vec<Cue> {
        (0..count)
            .map(|i| Cue::new(&format!("Brano {i}"), duration_ms))
            .collect()
    }

    fn manual() -> ShowConfig {
        ShowConfig {
            mode: PlaybackMode::Manual,
            default_gap_ms: 5_000,
            ..ShowConfig::default()
        }
    }

    fn automatic(gap_ms: u64) -> ShowConfig {
        ShowConfig {
            mode: PlaybackMode::Automatic,
            default_gap_ms: gap_ms,
            ..ShowConfig::default()
        }
    }

    fn started(show: &mut Show) -> Vec<Effect> {
        match show.go() {
            GoOutcome::Started(effects) => effects,
            other => panic!("expected started, got {other:?}"),
        }
    }

    fn has(effects: &[Effect], wanted: Effect) -> bool {
        effects.contains(&wanted)
    }

    #[test]
    fn manual_waits_for_go_after_each_song() {
        let mut show = Show::new(cues(2, 10_000), manual());
        assert!(has(&started(&mut show), Effect::PlaySong { index: 0 }));
        assert_eq!(
            show.tick(10_000),
            vec![Effect::StartBackground { fade_in_ms: 1_800 }]
        );
        assert_eq!(show.phase(), Phase::Waiting);
        assert_eq!(show.cursor_index(), Some(1));
        assert_eq!(show.status_of(0), Some(SongStatus::Done));
        assert!(!show.is_counting_down());

        show.tick(60_000);
        assert_eq!(
            show.phase(),
            Phase::Waiting,
            "manual must never auto-advance"
        );

        assert!(has(&started(&mut show), Effect::PlaySong { index: 1 }));
        assert_eq!(show.phase(), Phase::Playing);
        let effects = show.tick(10_000);
        assert!(has(&effects, Effect::StartBackground { fade_in_ms: 1_800 }));
        assert_eq!(show.phase(), Phase::Finished);
    }

    #[test]
    fn go_during_playback_requires_confirmation_then_crossfades() {
        let mut show = Show::new(cues(2, 10_000), manual());
        started(&mut show);
        assert_eq!(show.go(), GoOutcome::ConfirmRequired);
        let effects = show.confirm_go();
        assert!(has(&effects, Effect::PlaySong { index: 1 }));
        assert!(!has(&effects, Effect::StopSong { fade_ms: 250 }));
        assert_eq!(show.status_of(0), Some(SongStatus::Skipped));
        assert_eq!(show.phase(), Phase::Playing);
        assert_eq!(show.live_index(), Some(1));
    }

    #[test]
    fn automatic_counts_down_and_starts_next_song() {
        let mut show = Show::new(cues(2, 10_000), automatic(5_000));
        started(&mut show);
        let effects = show.tick(10_000);
        assert!(has(&effects, Effect::StartBackground { fade_in_ms: 1_800 }));
        assert_eq!(show.phase(), Phase::Waiting);
        assert!(show.is_counting_down());
        assert_eq!(show.gap_remaining_ms(), Some(5_000));

        show.tick(4_999);
        assert_eq!(show.phase(), Phase::Waiting);
        assert_eq!(show.gap_remaining_ms(), Some(1));

        let effects = show.tick(1);
        assert!(has(&effects, Effect::StopBackground { fade_ms: 2_200 }));
        assert!(has(&effects, Effect::PlaySong { index: 1 }));
        assert_eq!(show.phase(), Phase::Playing);
    }

    #[test]
    fn suspending_automatic_freezes_countdown_and_resumes_it() {
        let mut show = Show::new(cues(2, 10_000), automatic(5_000));
        started(&mut show);
        show.tick(10_000);
        show.toggle_auto_suspend();
        show.tick(60_000);
        assert_eq!(show.phase(), Phase::Waiting);
        assert_eq!(show.gap_remaining_ms(), Some(5_000));

        show.set_auto_suspended(false);
        show.tick(5_000);
        assert_eq!(show.live_index(), Some(1));
    }

    #[test]
    fn per_transition_gap_overrides_the_global_one() {
        let mut cue_list = cues(2, 1_000);
        cue_list[0].gap_after_ms = Some(1_500);
        let mut show = Show::new(cue_list, automatic(9_000));
        started(&mut show);
        show.tick(1_000);
        assert_eq!(show.gap_remaining_ms(), Some(1_500));
    }

    #[test]
    fn zero_gap_automatic_advances_without_background() {
        let mut show = Show::new(cues(2, 1_000), automatic(0));
        started(&mut show);
        let effects = show.tick(1_000);
        assert!(has(&effects, Effect::PlaySong { index: 1 }));
        assert!(!has(
            &effects,
            Effect::StartBackground { fade_in_ms: 1_800 }
        ));
        assert_eq!(show.status_of(0), Some(SongStatus::Done));
    }

    #[test]
    fn skip_in_manual_stops_song_and_waits_with_background() {
        let mut show = Show::new(cues(2, 10_000), manual());
        started(&mut show);
        let effects = show.skip();
        assert!(has(&effects, Effect::StopSong { fade_ms: 250 }));
        assert!(has(&effects, Effect::StartBackground { fade_in_ms: 1_800 }));
        assert_eq!(show.phase(), Phase::Waiting);
        assert_eq!(show.status_of(0), Some(SongStatus::Skipped));
        assert_eq!(show.live_index(), None);
    }

    #[test]
    fn skip_during_countdown_starts_next_song_immediately() {
        let mut show = Show::new(cues(2, 10_000), automatic(5_000));
        started(&mut show);
        show.tick(10_000);
        let effects = show.skip();
        assert!(has(&effects, Effect::PlaySong { index: 1 }));
        assert_eq!(show.phase(), Phase::Playing);
    }

    #[test]
    fn stop_during_fade_finishes_in_idle() {
        let mut show = Show::new(cues(2, 10_000), manual());
        started(&mut show);
        let effects = show.stop(800);
        assert!(has(&effects, Effect::StopSong { fade_ms: 800 }));
        assert_eq!(show.phase(), Phase::Stopping);
        show.tick(400);
        assert_eq!(show.phase(), Phase::Stopping);
        show.tick(400);
        assert_eq!(show.phase(), Phase::Idle);
        assert_eq!(show.live_index(), None);
        assert_eq!(show.cursor_index(), Some(1));
    }

    #[test]
    fn panic_fades_everything_and_returns_to_idle() {
        let mut show = Show::new(cues(3, 10_000), manual());
        started(&mut show);
        let effects = show.panic();
        assert!(has(&effects, Effect::StopSong { fade_ms: 180 }));
        assert_eq!(show.phase(), Phase::Panic);
        show.tick(180);
        assert_eq!(show.phase(), Phase::Idle);
        assert_eq!(show.status_of(0), Some(SongStatus::Skipped));
        assert_eq!(show.cursor_index(), Some(1));
    }

    #[test]
    fn pause_freezes_elapsed_time() {
        let mut show = Show::new(cues(1, 10_000), manual());
        started(&mut show);
        show.tick(2_000);
        show.pause();
        assert_eq!(show.phase(), Phase::Paused);
        show.tick(9_000);
        assert_eq!(show.elapsed_ms(), 2_000);
        show.resume();
        assert_eq!(show.phase(), Phase::Playing);
        show.tick(8_000);
        assert_eq!(show.phase(), Phase::Finished);
    }

    #[test]
    fn pause_and_resume_background_while_waiting() {
        let mut show = Show::new(cues(2, 1_000), manual());
        started(&mut show);
        show.tick(1_000);
        assert_eq!(show.pause(), vec![Effect::PauseBackground]);
        assert_eq!(show.resume(), vec![Effect::ResumeBackground]);
    }

    #[test]
    fn previous_goes_back_and_holds_for_go() {
        let mut show = Show::new(cues(3, 10_000), automatic(1_000));
        started(&mut show);
        show.tick(10_000);
        show.tick(1_000); // now playing index 1
        assert_eq!(show.live_index(), Some(1));

        let effects = show.previous();
        assert!(has(&effects, Effect::StopSong { fade_ms: 250 }));
        assert!(has(&effects, Effect::StartBackground { fade_in_ms: 1_800 }));
        assert_eq!(show.cursor_index(), Some(0));
        assert_eq!(show.phase(), Phase::Waiting);
        assert!(show.hold_for_go());
        show.tick(60_000);
        assert_eq!(show.phase(), Phase::Waiting, "previous must wait for GO");

        assert!(has(&started(&mut show), Effect::PlaySong { index: 0 }));
    }

    #[test]
    fn end_of_running_order_keeps_background_until_stop() {
        let mut show = Show::new(cues(1, 1_000), manual());
        started(&mut show);
        show.tick(1_000);
        assert_eq!(show.phase(), Phase::Finished);
        assert!(show.background_running());
        show.tick(600_000);
        assert_eq!(show.phase(), Phase::Finished);
        let effects = show.stop(500);
        assert!(has(&effects, Effect::StopBackground { fade_ms: 500 }));
        show.tick(500);
        assert_eq!(show.phase(), Phase::Idle);
    }

    #[test]
    fn go_on_finished_restarts_the_evening() {
        let mut show = Show::new(cues(2, 1_000), manual());
        started(&mut show);
        show.tick(1_000);
        started(&mut show);
        show.tick(1_000);
        assert_eq!(show.phase(), Phase::Finished);
        assert!(has(
            &show.go_after_finished_effects(),
            Effect::PlaySong { index: 0 }
        ));
        assert_eq!(show.status_of(0), Some(SongStatus::Live));
        assert_eq!(show.cursor_index(), Some(1));
    }

    impl Show {
        #[cfg(test)]
        fn go_after_finished_effects(&mut self) -> Vec<Effect> {
            match self.go() {
                GoOutcome::Started(effects) => effects,
                other => panic!("expected started, got {other:?}"),
            }
        }
    }

    #[test]
    fn switching_to_automatic_during_wait_starts_the_countdown() {
        let mut show = Show::new(cues(2, 10_000), manual());
        started(&mut show);
        show.tick(10_000);
        assert_eq!(show.gap_remaining_ms(), Some(5_000));
        show.set_mode(PlaybackMode::Automatic);
        assert!(show.is_counting_down());
        show.tick(5_000);
        assert_eq!(show.live_index(), Some(1));
    }

    #[test]
    fn switching_to_manual_never_auto_advances() {
        let mut show = Show::new(cues(2, 10_000), automatic(1_000));
        started(&mut show);
        show.tick(10_000);
        show.set_mode(PlaybackMode::Manual);
        show.tick(60_000);
        assert_eq!(show.phase(), Phase::Waiting);
    }

    #[test]
    fn empty_show_is_safe() {
        let mut show = Show::empty();
        assert_eq!(show.go(), GoOutcome::Ignored);
        assert!(show.tick(10_000).is_empty());
        assert!(show.stop(200).is_empty());
        assert!(show.skip().is_empty());
        assert!(show.previous().is_empty());
        assert!(show.panic().is_empty());
        assert_eq!(show.phase(), Phase::Idle);
    }

    #[test]
    fn manual_background_can_start_before_the_first_song() {
        let mut show = Show::new(cues(1, 1_000), manual());
        let effects = show.start_background();
        assert!(has(&effects, Effect::StartBackground { fade_in_ms: 1_800 }));
        assert_eq!(show.phase(), Phase::Idle);
        assert!(show.background_running());
        let effects = started(&mut show);
        assert!(has(&effects, Effect::StopBackground { fade_ms: 2_200 }));
        assert_eq!(show.phase(), Phase::Playing);
    }

    #[test]
    fn statuses_can_be_synced_into_a_project() {
        let mut project = Project::new("Serata");
        for i in 0..2 {
            project.add_song(Song::new(format!("Brano {i}"), None, 1_000));
        }
        let mut show = Show::from_project(&project);
        started(&mut show);
        show.tick(1_000);
        show.sync_statuses(&mut project);
        assert_eq!(project.songs[0].status, SongStatus::Done);
        assert_eq!(project.songs[1].status, SongStatus::Pending);
    }

    #[test]
    fn soak_simulates_two_hours_of_automatic_playback() {
        // 12 synthetic tracks, 45 s each, 6 s gaps: ~11 minutes per pass.
        // Simulating 2 hours covers several full passes plus the encore restart.
        let mut cue_list: Vec<Cue> = (0..12)
            .map(|i| Cue::new(&format!("Brano {i}"), 45_000))
            .collect();
        cue_list[3].gap_after_ms = Some(2_000);
        let mut show = Show::new(cue_list, automatic(6_000));
        let mut elapsed = 0u64;
        let mut played = 0usize;
        let mut guard = 0;
        while elapsed < 2 * 60 * 60 * 1000 {
            if matches!(show.phase(), Phase::Idle | Phase::Finished) {
                if show.go() == GoOutcome::Ignored {
                    break;
                }
                played += 1;
            }
            let effects = show.tick(20);
            for effect in effects {
                if matches!(effect, Effect::PlaySong { .. }) {
                    played += 1;
                }
            }
            elapsed += 20;
            guard += 1;
            assert!(guard < 500_000, "soak loop did not converge");
        }
        assert!(played >= 12, "expected a full running order, got {played}");
        assert!(show.elapsed_ms() <= show.live_duration_ms());
        assert!(show.gap_remaining_ms().is_none_or(|g| g < 60_000));
    }
}
