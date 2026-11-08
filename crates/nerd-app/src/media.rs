//! Importing files into the media library.
//!
//! Everything expensive (decoding for waveform analysis, video probing and
//! thumbnail extraction) happens on a worker thread. The UI receives a fully
//! populated [`MediaAsset`].

use std::path::{Path, PathBuf};

use nerd_core::library::{media_kind_for_path, MediaAsset, MediaKind};
use nerd_core::waveform::DEFAULT_BUCKETS;

#[derive(Debug)]
pub struct ImportOutcome {
    pub path: PathBuf,
    pub asset: Option<MediaAsset>,
    pub error: Option<String>,
}

pub fn import_file(path: PathBuf, thumbnail_dir: PathBuf) -> ImportOutcome {
    let Some(kind) = media_kind_for_path(&path) else {
        return ImportOutcome {
            path,
            asset: None,
            error: Some("Formato non supportato".to_string()),
        };
    };
    if !path.is_file() {
        return ImportOutcome {
            path,
            asset: None,
            error: Some("File non trovato".to_string()),
        };
    }

    let analysis = nerd_audio::analyze_file(&path).ok();
    let mut duration_ms = analysis.as_ref().map(|a| a.duration_ms).unwrap_or(0);
    let mut width = 0;
    let mut height = 0;
    let mut has_thumbnail = false;
    let mut thumbnail_error = None;

    if kind == MediaKind::Video {
        match nerd_video::probe(&path) {
            Ok(metadata) => {
                if metadata.duration_ms > 0 {
                    duration_ms = metadata.duration_ms;
                }
                width = metadata.width;
                height = metadata.height;
            }
            Err(err) => {
                thumbnail_error = Some(err.to_string());
            }
        }
    }

    if kind == MediaKind::Audio && analysis.is_none() {
        return ImportOutcome {
            path,
            asset: None,
            error: Some("Impossibile decodificare l'audio".to_string()),
        };
    }

    let mut asset = MediaAsset::new(path.clone(), kind, duration_ms);
    asset.width = width;
    asset.height = height;
    if let Some(analysis) = &analysis {
        asset.waveform = if analysis.peaks.is_empty() {
            vec![0.0; DEFAULT_BUCKETS]
        } else {
            analysis.peaks.clone()
        };
        if asset.waveform.len() > DEFAULT_BUCKETS {
            asset.waveform.truncate(DEFAULT_BUCKETS);
        }
    }

    if kind == MediaKind::Video {
        let thumbnail_path = thumbnail_dir.join(format!("{}.png", asset.id));
        match nerd_video::thumbnail(&path, 1_000, 720) {
            Ok(thumbnail) => {
                match image::RgbaImage::from_raw(thumbnail.width, thumbnail.height, thumbnail.rgba)
                {
                    Some(image) => match image.save(&thumbnail_path) {
                        Ok(()) => {
                            asset.has_thumbnail = true;
                            has_thumbnail = true;
                        }
                        Err(err) => thumbnail_error = Some(err.to_string()),
                    },
                    None => {
                        thumbnail_error = Some("anteprima video non valida".to_string());
                    }
                }
            }
            Err(err) => thumbnail_error = Some(err.to_string()),
        }
    }

    let warning = thumbnail_error
        .filter(|_| !has_thumbnail && kind == MediaKind::Video)
        .map(|err| format!("Video importato senza anteprima: {err}"));

    tracing::info!(
        path = %path.display(),
        kind = ?kind,
        duration_ms,
        waveform = asset.waveform.len(),
        "media imported"
    );

    ImportOutcome {
        path,
        asset: Some(asset),
        error: warning,
    }
}

/// Path of the cached thumbnail for an asset.
pub fn thumbnail_path(thumbnail_dir: &Path, asset: &MediaAsset) -> PathBuf {
    thumbnail_dir.join(format!("{}.png", asset.id))
}
