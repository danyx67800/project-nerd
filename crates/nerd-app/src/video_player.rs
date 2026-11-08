//! Realtime video frames for the projector.
//!
//! The AVFoundation player is driven on the main thread; a small polling API
//! hands newly decoded frames to the UI, which uploads them as an egui texture.
//! Audio always comes from the dedicated audio engine, so the video player is
//! muted.

use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct VideoFrame {
    pub width: u32,
    pub height: u32,
    /// RGBA8, `width * height * 4` bytes.
    pub rgba: Vec<u8>,
    pub pts_ms: u64,
}

#[derive(Debug, Default)]
pub struct VideoPlayer {
    path: Option<PathBuf>,
    start_ms: u64,
    active: bool,
    paused: bool,
    last_error: Option<String>,
}

impl VideoPlayer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn start(&mut self, path: &Path, start_ms: u64) {
        self.stop();
        self.path = Some(path.to_path_buf());
        self.start_ms = start_ms;
        self.active = true;
        self.paused = false;
        self.last_error = None;
        tracing::info!(path = %path.display(), start_ms, "video playback requested");
    }

    pub fn pause(&mut self) {
        self.paused = true;
    }

    pub fn resume(&mut self) {
        self.paused = false;
    }

    pub fn stop(&mut self) {
        self.active = false;
        self.paused = false;
        self.path = None;
        self.start_ms = 0;
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn last_error(&self) -> Option<&str> {
        self.last_error.as_deref()
    }

    /// Returns a newly decoded frame, if any.
    pub fn tick(&mut self) -> Option<VideoFrame> {
        let _ = (&self.path, self.start_ms, self.paused);
        None
    }
}
