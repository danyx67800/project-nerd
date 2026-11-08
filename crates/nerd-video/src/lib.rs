//! Video probing, thumbnails, playback frames and display enumeration.
//!
//! The macOS implementation uses AVFoundation/CoreGraphics through `objc2`.
//! All native code is isolated in the `macos` module; the public API is usable
//! from the UI on every platform.

use std::path::Path;

/// A physical or virtual display usable for the projector output.
#[derive(Debug, Clone, PartialEq)]
pub struct DisplayInfo {
    /// Index in the macOS display list (0 = main display in practice).
    pub index: usize,
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub is_main: bool,
    pub refresh_hz: f32,
}

/// Technical metadata of a video file.
#[derive(Debug, Clone, Default)]
pub struct VideoMetadata {
    pub duration_ms: u64,
    pub width: u32,
    pub height: u32,
    pub has_audio: bool,
}

/// RGBA thumbnail produced by AVFoundation.
#[derive(Debug, Clone)]
pub struct Thumbnail {
    pub width: u32,
    pub height: u32,
    /// `width * height * 4` bytes, RGBA8.
    pub rgba: Vec<u8>,
}

#[derive(Debug, thiserror::Error)]
pub enum VideoError {
    #[error("video non supportato su questa piattaforma")]
    UnsupportedPlatform,
    #[error("file video non leggibile: {0}")]
    Probe(String),
    #[error("generazione anteprima fallita: {0}")]
    Thumbnail(String),
    #[error("decodifica video: {0}")]
    Decode(String),
}

pub type VideoResult<T> = Result<T, VideoError>;

#[cfg(target_os = "macos")]
mod macos;

/// Enumerates active displays. Cheap enough to poll every couple of seconds.
pub fn displays() -> Vec<DisplayInfo> {
    #[cfg(target_os = "macos")]
    {
        macos::displays()
    }
    #[cfg(not(target_os = "macos"))]
    {
        Vec::new()
    }
}

/// Reads duration and dimensions without decoding frames.
pub fn probe(path: &Path) -> VideoResult<VideoMetadata> {
    #[cfg(target_os = "macos")]
    {
        macos::probe(path)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = path;
        Err(VideoError::UnsupportedPlatform)
    }
}

/// Extracts one frame at `at_ms`, scaled down to at most `max_width`.
pub fn thumbnail(path: &Path, at_ms: u64, max_width: u32) -> VideoResult<Thumbnail> {
    #[cfg(target_os = "macos")]
    {
        macos::thumbnail(path, at_ms, max_width)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (path, at_ms, max_width);
        Err(VideoError::UnsupportedPlatform)
    }
}

/// Whether the platform build can decode video at all.
pub fn supported() -> bool {
    cfg!(target_os = "macos")
}
