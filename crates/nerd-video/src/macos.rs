//! macOS video integration (AVFoundation via objc2, CoreGraphics displays).
//!
//! This module is filled in during the video milestone. Keeping the stub here
//! lets the application shell build and run while the rest of the show is
//! implemented.

use std::path::Path;

use crate::{DisplayInfo, Thumbnail, VideoError, VideoMetadata, VideoResult};

pub(crate) fn displays() -> Vec<DisplayInfo> {
    Vec::new()
}

pub(crate) fn probe(path: &Path) -> VideoResult<VideoMetadata> {
    let _ = path;
    Err(VideoError::Probe(
        "decodifica video non ancora inizializzata".into(),
    ))
}

pub(crate) fn thumbnail(path: &Path, at_ms: u64, max_width: u32) -> VideoResult<Thumbnail> {
    let _ = (path, at_ms, max_width);
    Err(VideoError::Thumbnail(
        "decodifica video non ancora inizializzata".into(),
    ))
}
