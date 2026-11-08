//! File decoding and resampling.
//!
//! All public decode functions return **stereo interleaved f32** at the
//! requested sample rate, because the realtime mixer is stereo by design.
//! Decoding happens off the audio thread; the callback only reads finished
//! buffers.

use std::fs::File;
use std::path::{Path, PathBuf};

use audioadapter::Adapter;
use audioadapter_buffers::owned::InterleavedOwned;
use rubato::{Fft, FixedSync, Resampler};
use symphonia::core::codecs::audio::AudioDecoderOptions;
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, TrackType};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use tracing::{debug, warn};

use crate::{AudioError, AudioResult};

/// Fully decoded audio, always stereo interleaved at its own sample rate.
#[derive(Debug, Clone)]
pub struct DecodedAudio {
    pub sample_rate: u32,
    pub frames: usize,
    /// `frames * 2` samples, left/right.
    pub samples: Vec<f32>,
}

impl DecodedAudio {
    pub fn new(sample_rate: u32, samples: Vec<f32>) -> Self {
        let frames = samples.len() / 2;
        Self {
            sample_rate,
            frames,
            samples,
        }
    }

    pub fn duration_ms(&self) -> u64 {
        if self.sample_rate == 0 {
            0
        } else {
            (self.frames as u64 * 1000) / self.sample_rate as u64
        }
    }

    pub fn is_empty(&self) -> bool {
        self.frames == 0
    }
}

/// Result of a library analysis pass.
#[derive(Debug, Clone, Default)]
pub struct Analysis {
    pub duration_ms: u64,
    pub sample_rate: u32,
    pub channels: u16,
    pub peaks: Vec<f32>,
    pub rms: f32,
}

struct RawAudio {
    sample_rate: u32,
    channels: usize,
    samples: Vec<f32>,
}

impl RawAudio {
    fn duration_ms(&self) -> u64 {
        if self.sample_rate == 0 || self.channels == 0 {
            0
        } else {
            let frames = self.samples.len() / self.channels;
            (frames as u64 * 1000) / self.sample_rate as u64
        }
    }
}

/// Decodes a file, downmixes to stereo and resamples to `target_rate`.
///
/// `trim` is `(start_ms, end_ms)`, both optional in the sense that end `None`
/// means the end of the file.
pub fn decode_stereo(
    path: &Path,
    trim: (u64, Option<u64>),
    target_rate: u32,
) -> AudioResult<DecodedAudio> {
    if target_rate == 0 {
        return Err(AudioError::Decode(
            "frequenza di campionamento non valida".into(),
        ));
    }
    let raw = decode_raw(path)?;
    debug!(
        path = %path.display(),
        rate = raw.sample_rate,
        channels = raw.channels,
        ms = raw.duration_ms(),
        "decoded source"
    );
    let stereo = downmix_to_stereo(&raw);
    let resampled = if raw.sample_rate == target_rate {
        stereo
    } else {
        resample_stereo(&stereo, raw.sample_rate, target_rate)?
    };
    let frames = resampled.len() / 2;
    let start_frame = ms_to_frames(trim.0, target_rate).min(frames);
    let end_frame = trim
        .1
        .map(|ms| ms_to_frames(ms, target_rate))
        .unwrap_or(frames)
        .clamp(start_frame, frames);
    if start_frame == end_frame {
        return Err(AudioError::Decode(
            "l'intervallo di riproduzione è vuoto".into(),
        ));
    }
    let slice = &resampled[start_frame * 2..end_frame * 2];
    Ok(DecodedAudio::new(target_rate, slice.to_vec()))
}

/// Blocking helper used by the library importer.
pub fn analyze_file(path: &Path) -> AudioResult<Analysis> {
    let raw = decode_raw(path)?;
    let mono = downmix_to_mono(&raw);
    let peaks = nerd_core::waveform::peaks(&mono, 1, nerd_core::waveform::DEFAULT_BUCKETS);
    Ok(Analysis {
        duration_ms: raw.duration_ms(),
        sample_rate: raw.sample_rate,
        channels: raw.channels.min(u16::MAX as usize) as u16,
        peaks,
        rms: nerd_core::waveform::rms(&mono),
    })
}

/// Reads the first audio stream of `path` without resampling.
fn decode_raw(path: &Path) -> AudioResult<RawAudio> {
    let file = File::open(path)
        .map_err(|err| AudioError::Io(format!("impossibile aprire {}: {err}", path.display())))?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }
    let format_opts: FormatOptions = Default::default();
    let meta_opts: MetadataOptions = Default::default();
    let mut format = symphonia::default::get_probe()
        .probe(&hint, mss, format_opts, meta_opts)
        .map_err(|err| AudioError::Unsupported(format!("{err}")))?;

    let track = format
        .default_track(TrackType::Audio)
        .ok_or_else(|| AudioError::Unsupported("nessuna traccia audio".into()))?;
    let track_id = track.id;
    let audio_params = track
        .codec_params
        .as_ref()
        .and_then(|params| params.audio())
        .ok_or_else(|| AudioError::Unsupported("codec audio sconosciuto".into()))?
        .clone();

    let mut decoder = symphonia::default::get_codecs()
        .make_audio_decoder(&audio_params, &AudioDecoderOptions::default())
        .map_err(|err| AudioError::Unsupported(format!("{err}")))?;

    let mut samples: Vec<f32> = Vec::new();
    let mut sample_rate = audio_params.sample_rate.unwrap_or(0);
    let mut channels = audio_params
        .channels
        .as_ref()
        .map(|c| c.count())
        .unwrap_or(0);

    loop {
        let packet = match format.next_packet() {
            Ok(Some(packet)) => packet,
            Ok(None) => break,
            Err(SymphoniaError::IoError(err))
                if err.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                break
            }
            Err(err) => return Err(AudioError::Decode(format!("{err}"))),
        };
        if packet.track_id != track_id {
            continue;
        }
        match decoder.decode(&packet) {
            Ok(buffer) => {
                let spec = buffer.spec();
                if sample_rate == 0 {
                    sample_rate = spec.rate();
                }
                if channels == 0 {
                    channels = spec.channels().count();
                }
                if channels == 0 {
                    return Err(AudioError::Decode("canali audio non determinabili".into()));
                }
                let count = buffer.samples_interleaved();
                if count > 0 {
                    let start = samples.len();
                    samples.resize(start + count, 0.0);
                    buffer.copy_to_slice_interleaved::<f32, _>(&mut samples[start..]);
                }
            }
            Err(SymphoniaError::DecodeError(err)) => {
                warn!(error = %err, "skipping corrupt audio packet");
                continue;
            }
            Err(SymphoniaError::IoError(err))
                if err.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                break
            }
            Err(SymphoniaError::ResetRequired) => {
                return Err(AudioError::Decode(
                    "il flusso audio richiede un reset non supportato".into(),
                ))
            }
            Err(err) => return Err(AudioError::Decode(format!("{err}"))),
        }
    }

    if sample_rate == 0 || channels == 0 || samples.is_empty() {
        return Err(AudioError::Decode("file audio vuoto".into()));
    }
    Ok(RawAudio {
        sample_rate,
        channels,
        samples,
    })
}

fn downmix_to_stereo(raw: &RawAudio) -> Vec<f32> {
    match raw.channels {
        1 => raw.samples.iter().flat_map(|s| [*s, *s]).collect(),
        2 => raw.samples.clone(),
        channels => {
            let frames = raw.samples.len() / channels;
            let mut out = Vec::with_capacity(frames * 2);
            for frame in 0..frames {
                let base = frame * channels;
                let left = raw.samples[base];
                let right = if channels > 1 {
                    raw.samples[base + 1]
                } else {
                    left
                };
                out.push(left);
                out.push(right);
            }
            out
        }
    }
}

fn downmix_to_mono(raw: &RawAudio) -> Vec<f32> {
    let channels = raw.channels.max(1);
    let frames = raw.samples.len() / channels;
    let mut out = Vec::with_capacity(frames);
    for frame in 0..frames {
        let base = frame * channels;
        let mut sum = 0.0;
        for ch in 0..channels {
            sum += raw.samples[base + ch];
        }
        out.push(sum / channels as f32);
    }
    out
}

fn resample_stereo(samples: &[f32], from: u32, to: u32) -> AudioResult<Vec<f32>> {
    let frames = samples.len() / 2;
    if frames == 0 {
        return Ok(Vec::new());
    }
    let input = InterleavedOwned::new_from(samples.to_vec(), 2, frames)
        .map_err(|err| AudioError::Decode(format!("buffer di resampling: {err:?}")))?;
    let mut resampler = Fft::<f32>::new(from as usize, to as usize, 1024, 2, FixedSync::Both)
        .map_err(|err| AudioError::Decode(format!("resampler: {err}")))?;
    let output = resampler
        .process_all(&input, frames, None)
        .map_err(|err| AudioError::Decode(format!("resampler: {err}")))?;
    let frames = output.frames();
    let mut out = Vec::with_capacity(frames * 2);
    for frame in 0..frames {
        let left = output.read_sample(0, frame).unwrap_or(0.0);
        let right = output.read_sample(1, frame).unwrap_or(0.0);
        out.push(left);
        out.push(right);
    }
    Ok(out)
}

fn ms_to_frames(ms: u64, rate: u32) -> usize {
    ((ms as u128 * rate as u128) / 1000) as usize
}

/// Creates a mono/stereo playlist buffer by concatenating decoded tracks.
/// A short silence separates tracks so a hard edit does not click.
pub fn decode_playlist(
    paths: &[PathBuf],
    shuffle: bool,
    target_rate: u32,
) -> AudioResult<DecodedAudio> {
    let mut order: Vec<usize> = (0..paths.len()).collect();
    if shuffle && order.len() > 1 {
        let mut seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x9E37_79B9_7F4A_7C15);
        for i in (1..order.len()).rev() {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            let j = (seed as usize) % (i + 1);
            order.swap(i, j);
        }
    }
    let mut out: Vec<f32> = Vec::new();
    for (position, index) in order.into_iter().enumerate() {
        let path = &paths[index];
        match decode_stereo(path, (0, None), target_rate) {
            Ok(track) => {
                if position > 0 {
                    out.extend(std::iter::repeat_n(0.0, (target_rate as usize / 4) * 2));
                }
                out.extend_from_slice(&track.samples);
            }
            Err(err) => {
                warn!(path = %path.display(), error = %err, "background track skipped");
            }
        }
    }
    if out.is_empty() {
        return Err(AudioError::Decode(
            "nessun brano di sottofondo riproducibile".into(),
        ));
    }
    Ok(DecodedAudio::new(target_rate, out))
}

/// Returns whether a decoded buffer looks usable before it reaches the mixer.
pub fn is_usable(buffer: &DecodedAudio) -> bool {
    buffer.frames > 0 && buffer.samples.len() >= 2 && buffer.samples.iter().any(|s| s.is_finite())
}

/// Kept for symmetry and future metadata work; extracts the audio track's
/// technical description without decoding.
pub fn probe_summary(path: &Path) -> AudioResult<Analysis> {
    analyze_file(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// Writes a small PCM16 WAV without pulling a codec crate into dev-deps.
    fn write_test_wav(path: &Path, rate: u32, seconds: f32, channels: u16) {
        let frames = (rate as f32 * seconds) as u32;
        let data_len = frames * channels as u32 * 2;
        let mut file = File::create(path).unwrap();
        file.write_all(b"RIFF").unwrap();
        file.write_all(&(36 + data_len).to_le_bytes()).unwrap();
        file.write_all(b"WAVEfmt ").unwrap();
        file.write_all(&16u32.to_le_bytes()).unwrap();
        file.write_all(&1u16.to_le_bytes()).unwrap();
        file.write_all(&channels.to_le_bytes()).unwrap();
        file.write_all(&rate.to_le_bytes()).unwrap();
        file.write_all(&(rate * channels as u32 * 2).to_le_bytes())
            .unwrap();
        file.write_all(&(channels * 2).to_le_bytes()).unwrap();
        file.write_all(&16u16.to_le_bytes()).unwrap();
        file.write_all(b"data").unwrap();
        file.write_all(&data_len.to_le_bytes()).unwrap();
        for frame in 0..frames {
            let t = frame as f32 / rate as f32;
            let sample = (2.0 * std::f32::consts::PI * 440.0 * t).sin() * 0.5;
            let value = (sample * i16::MAX as f32) as i16;
            for _ in 0..channels {
                file.write_all(&value.to_le_bytes()).unwrap();
            }
        }
    }

    #[test]
    fn decodes_and_resamples_a_generated_wav() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tone.wav");
        write_test_wav(&path, 44_100, 1.0, 2);
        let decoded = decode_stereo(&path, (0, None), 48_000).unwrap();
        assert_eq!(decoded.sample_rate, 48_000);
        assert!((decoded.duration_ms() as i64 - 1000).abs() <= 10);
        assert!(decoded.frames > 47_000);
        assert!(is_usable(&decoded));
    }

    #[test]
    fn trims_to_the_requested_interval() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tone.wav");
        write_test_wav(&path, 48_000, 2.0, 1);
        let decoded = decode_stereo(&path, (500, Some(1_500)), 48_000).unwrap();
        assert!((decoded.duration_ms() as i64 - 1000).abs() <= 2);
        assert_eq!(decoded.samples.len() % 2, 0);
    }

    #[test]
    fn analysis_reports_duration_and_peaks() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tone.wav");
        write_test_wav(&path, 44_100, 1.5, 2);
        let analysis = analyze_file(&path).unwrap();
        assert!((analysis.duration_ms as i64 - 1500).abs() <= 10);
        assert_eq!(analysis.channels, 2);
        assert!(!analysis.peaks.is_empty());
        assert!(analysis.peaks.iter().any(|p| *p > 0.1));
    }

    #[test]
    fn empty_trim_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tone.wav");
        write_test_wav(&path, 48_000, 1.0, 2);
        assert!(decode_stereo(&path, (500, Some(500)), 48_000).is_err());
    }

    #[test]
    fn playlist_concatenates_tracks() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.wav");
        let b = dir.path().join("b.wav");
        write_test_wav(&a, 48_000, 0.5, 1);
        write_test_wav(&b, 48_000, 0.5, 1);
        let playlist = decode_playlist(&[a, b], false, 48_000).unwrap();
        assert!(playlist.duration_ms() >= 1200);
    }
}
