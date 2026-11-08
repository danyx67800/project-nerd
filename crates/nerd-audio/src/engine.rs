//! CoreAudio output, lock-free command queue and sample-accurate mixer.
//!
//! The realtime callback never allocates and never locks. Decoded buffers are
//! moved in as `Arc<DecodedAudio>`; replaced buffers are handed back through a
//! garbage queue so the deallocation happens on the control thread.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SampleFormat, SizedSample, StreamConfig};
use crossbeam_queue::ArrayQueue;
use tracing::{info, warn};

use crate::decode::{decode_stereo, DecodedAudio};
use crate::{AudioError, AudioResult};

const COMMAND_CAPACITY: usize = 128;
const GARBAGE_CAPACITY: usize = 32;
const CACHE_LIMIT: usize = 3;

/// A selectable output device.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioDeviceInfo {
    pub name: String,
    pub is_default: bool,
}

/// Snapshot of the current output state.
#[derive(Debug, Clone)]
pub struct EngineStatus {
    pub device_name: Option<String>,
    pub sample_rate: u32,
    pub channels: u16,
    pub available: bool,
    pub warning: Option<String>,
    pub error: Option<String>,
}

/// Metering and progress values published by the audio callback.
#[derive(Debug, Clone, Copy, Default)]
pub struct Levels {
    pub song_peak: f32,
    pub background_peak: f32,
    pub song_pos_ms: u64,
    pub song_duration_ms: u64,
    pub background_pos_ms: u64,
    pub background_duration_ms: u64,
    pub background_playing: bool,
}

/// Asynchronous engine notifications, polled from the UI.
#[derive(Debug, Clone)]
pub enum AudioEvent {
    Prepared { key: String },
    PrepareFailed { key: String, message: String },
    OutputLost,
    OutputRestored,
}

#[derive(Debug, Clone, Copy)]
struct VoiceParams {
    volume: f32,
    fade_in_ms: u64,
    fade_out_ms: u64,
    start_frame: usize,
}

#[derive(Debug)]
enum Command {
    PlaySong {
        audio: Arc<DecodedAudio>,
        params: VoiceParams,
    },
    StopSong {
        fade_ms: u64,
    },
    PauseSong,
    ResumeSong,
    SetSongVolume(f32),
    PlayBackground {
        audio: Arc<DecodedAudio>,
        volume: f32,
        fade_in_ms: u64,
        looping: bool,
        start_frame: usize,
    },
    StopBackground {
        fade_ms: u64,
    },
    PauseBackground,
    ResumeBackground,
    SetBackgroundVolume(f32),
    SetMasterVolume(f32),
}

struct Shared {
    song_peak: AtomicU32,
    background_peak: AtomicU32,
    song_pos_ms: AtomicU64,
    song_duration_ms: AtomicU64,
    background_pos_ms: AtomicU64,
    background_duration_ms: AtomicU64,
    background_playing: AtomicBool,
    disconnected: AtomicBool,
    error: Mutex<Option<String>>,
}

impl Shared {
    fn new() -> Self {
        Self {
            song_peak: AtomicU32::new(0),
            background_peak: AtomicU32::new(0),
            song_pos_ms: AtomicU64::new(0),
            song_duration_ms: AtomicU64::new(0),
            background_pos_ms: AtomicU64::new(0),
            background_duration_ms: AtomicU64::new(0),
            background_playing: AtomicBool::new(false),
            disconnected: AtomicBool::new(false),
            error: Mutex::new(None),
        }
    }
}

struct Voice {
    audio: Arc<DecodedAudio>,
    rate: u32,
    pos: usize,
    gain: f32,
    base_volume: f32,
    ramp_target: f32,
    ramp_step: f32,
    ramp_remaining: usize,
    end_fade_at: Option<usize>,
    end_fade_started: bool,
    looping: bool,
    paused: bool,
    pausing: bool,
    stopping: bool,
    finished: bool,
}

impl Voice {
    fn new(
        audio: Arc<DecodedAudio>,
        rate: u32,
        volume: f32,
        fade_in_ms: u64,
        fade_out_ms: u64,
        start_frame: usize,
        looping: bool,
    ) -> Self {
        let frames = audio.frames;
        let pos = if frames == 0 {
            0
        } else if looping {
            start_frame % frames
        } else {
            start_frame.min(frames)
        };
        let end_fade_frames = ms_to_frames(fade_out_ms, rate);
        let end_fade_at = if !looping && frames > end_fade_frames && end_fade_frames > 0 {
            Some(frames - end_fade_frames)
        } else {
            None
        };
        let mut voice = Self {
            audio,
            rate,
            pos,
            gain: 0.0,
            base_volume: volume.clamp(0.0, 2.0),
            ramp_target: volume.clamp(0.0, 2.0),
            ramp_step: 0.0,
            ramp_remaining: 0,
            end_fade_at,
            end_fade_started: false,
            looping,
            paused: false,
            pausing: false,
            stopping: false,
            finished: frames == 0,
        };
        let fade_frames = ms_to_frames(fade_in_ms, rate);
        if fade_frames > 0 {
            voice.ramp_to_frames(voice.base_volume, fade_frames);
        } else {
            voice.gain = voice.base_volume;
        }
        voice
    }

    fn ramp_to_frames(&mut self, target: f32, frames: usize) {
        self.ramp_target = target;
        if frames == 0 {
            self.gain = target;
            self.ramp_remaining = 0;
            self.ramp_step = 0.0;
            return;
        }
        self.ramp_remaining = frames;
        self.ramp_step = (target - self.gain) / frames as f32;
    }

    fn ramp_to_ms(&mut self, target: f32, ms: u64) {
        let frames = ms_to_frames(ms, self.rate);
        self.ramp_to_frames(target, frames);
    }

    fn advance_gain(&mut self) {
        if self.ramp_remaining == 0 {
            return;
        }
        self.ramp_remaining -= 1;
        if self.ramp_remaining == 0 {
            self.gain = self.ramp_target;
        } else {
            self.gain += self.ramp_step;
        }
    }

    fn sample(&mut self) -> (f32, f32) {
        if self.paused || self.finished || self.audio.frames == 0 {
            return (0.0, 0.0);
        }
        if let Some(at) = self.end_fade_at {
            if self.pos >= at && !self.end_fade_started {
                self.end_fade_started = true;
                let remaining = self.audio.frames - self.pos;
                self.ramp_to_frames(0.0, remaining);
            }
        }
        let index = self.pos.min(self.audio.frames - 1) * 2;
        let left = self.audio.samples.get(index).copied().unwrap_or(0.0);
        let right = self.audio.samples.get(index + 1).copied().unwrap_or(0.0);
        let out = (left * self.gain, right * self.gain);

        self.pos += 1;
        self.advance_gain();

        if self.pausing && self.gain <= 0.0001 {
            self.gain = 0.0;
            self.pausing = false;
            self.paused = true;
        }
        if self.stopping && self.gain <= 0.0001 {
            self.finished = true;
        }
        if self.pos >= self.audio.frames {
            if self.looping {
                self.pos = 0;
                self.end_fade_started = false;
            } else {
                self.finished = true;
            }
        }
        out
    }

    fn pause(&mut self, ms: u64) {
        if self.paused || self.finished {
            return;
        }
        self.pausing = true;
        self.ramp_to_ms(0.0, ms);
    }

    fn resume(&mut self, ms: u64) {
        self.paused = false;
        self.pausing = false;
        self.ramp_to_ms(self.base_volume, ms);
    }

    fn fade_out(&mut self, ms: u64) {
        if self.finished {
            return;
        }
        if ms == 0 {
            self.gain = 0.0;
            self.finished = true;
            return;
        }
        self.stopping = true;
        self.ramp_to_ms(0.0, ms);
    }

    fn set_base_volume(&mut self, volume: f32) {
        self.base_volume = volume.clamp(0.0, 2.0);
        if !self.paused && !self.pausing && !self.stopping {
            self.ramp_to_ms(self.base_volume, 30);
        }
    }
}

struct Mixer {
    rate: u32,
    out_channels: usize,
    commands: Arc<ArrayQueue<Command>>,
    garbage: Arc<ArrayQueue<Arc<DecodedAudio>>>,
    shared: Arc<Shared>,
    songs: [Option<Voice>; 2],
    active: usize,
    background: Option<Voice>,
    master: f32,
}

impl Mixer {
    fn new(
        rate: u32,
        out_channels: u16,
        commands: Arc<ArrayQueue<Command>>,
        garbage: Arc<ArrayQueue<Arc<DecodedAudio>>>,
        shared: Arc<Shared>,
    ) -> Self {
        Self {
            rate,
            out_channels: out_channels.max(1) as usize,
            commands,
            garbage,
            shared,
            songs: [None, None],
            active: 0,
            background: None,
            master: 1.0,
        }
    }

    fn retire(&mut self, voice: Option<Voice>) {
        if let Some(voice) = voice {
            let _ = self.garbage.push(voice.audio);
        }
    }

    fn process_commands(&mut self) {
        while let Some(command) = self.commands.pop() {
            match command {
                Command::PlaySong { audio, params } => {
                    let incoming = if self.songs[0].is_none() {
                        0
                    } else if self.songs[1].is_none() {
                        1
                    } else {
                        1 - self.active
                    };
                    let old = self.songs[incoming].take();
                    self.retire(old);
                    for (index, voice) in self.songs.iter_mut().enumerate() {
                        if index == incoming {
                            continue;
                        }
                        if let Some(voice) = voice {
                            if !voice.stopping {
                                voice.fade_out(180);
                            }
                        }
                    }
                    self.songs[incoming] = Some(Voice::new(
                        audio,
                        self.rate,
                        params.volume,
                        params.fade_in_ms,
                        params.fade_out_ms,
                        params.start_frame,
                        false,
                    ));
                    self.active = incoming;
                }
                Command::StopSong { fade_ms } => {
                    for voice in self.songs.iter_mut().flatten() {
                        voice.fade_out(fade_ms);
                    }
                }
                Command::PauseSong => {
                    for voice in self.songs.iter_mut().flatten() {
                        voice.pause(5);
                    }
                }
                Command::ResumeSong => {
                    for voice in self.songs.iter_mut().flatten() {
                        voice.resume(12);
                    }
                }
                Command::SetSongVolume(volume) => {
                    for voice in self.songs.iter_mut().flatten() {
                        voice.set_base_volume(volume);
                    }
                }
                Command::PlayBackground {
                    audio,
                    volume,
                    fade_in_ms,
                    looping,
                    start_frame,
                } => {
                    let old = self.background.take();
                    self.retire(old);
                    self.background = Some(Voice::new(
                        audio,
                        self.rate,
                        volume,
                        fade_in_ms,
                        0,
                        start_frame,
                        looping,
                    ));
                }
                Command::StopBackground { fade_ms } => {
                    if let Some(voice) = &mut self.background {
                        voice.fade_out(fade_ms);
                    }
                }
                Command::PauseBackground => {
                    if let Some(voice) = &mut self.background {
                        voice.pause(5);
                    }
                }
                Command::ResumeBackground => {
                    if let Some(voice) = &mut self.background {
                        voice.resume(12);
                    }
                }
                Command::SetBackgroundVolume(volume) => {
                    if let Some(voice) = &mut self.background {
                        voice.set_base_volume(volume);
                    }
                }
                Command::SetMasterVolume(volume) => {
                    self.master = volume.clamp(0.0, 1.0);
                }
            }
        }
    }

    fn next_sample(&mut self) -> (f32, f32, f32, f32) {
        let mut song_left = 0.0;
        let mut song_right = 0.0;
        for voice in self.songs.iter_mut().flatten() {
            let (left, right) = voice.sample();
            song_left += left;
            song_right += right;
        }
        let mut background_left = 0.0;
        let mut background_right = 0.0;
        if let Some(voice) = &mut self.background {
            let (left, right) = voice.sample();
            background_left = left;
            background_right = right;
        }

        for index in 0..self.songs.len() {
            let finished = self.songs[index]
                .as_ref()
                .is_some_and(|voice| voice.finished);
            if finished {
                let voice = self.songs[index].take();
                self.retire(voice);
            }
        }
        let background_finished = self.background.as_ref().is_some_and(|voice| voice.finished);
        if background_finished {
            let voice = self.background.take();
            self.retire(voice);
        }
        (song_left, song_right, background_left, background_right)
    }

    fn render<T>(&mut self, data: &mut [T])
    where
        T: SizedSample + FromSample<f32>,
    {
        self.process_commands();
        let channels = self.out_channels.max(1);
        let frames = data.len() / channels;
        let mut song_peak = 0.0f32;
        let mut background_peak = 0.0f32;
        for frame in 0..frames {
            let (song_left, song_right, bg_left, bg_right) = self.next_sample();
            song_peak = song_peak.max(song_left.abs()).max(song_right.abs());
            background_peak = background_peak.max(bg_left.abs()).max(bg_right.abs());
            let left = ((song_left + bg_left) * self.master).clamp(-1.0, 1.0);
            let right = ((song_right + bg_right) * self.master).clamp(-1.0, 1.0);
            for channel in 0..channels {
                let value = if channels == 1 {
                    (left + right) * 0.5
                } else if channel % 2 == 0 {
                    left
                } else {
                    right
                };
                data[frame * channels + channel] = T::from_sample(value);
            }
        }
        publish_peak(&self.shared.song_peak, song_peak);
        publish_peak(&self.shared.background_peak, background_peak);

        let rate = self.rate.max(1) as u64;
        if let Some(voice) = &self.songs[self.active] {
            self.shared
                .song_pos_ms
                .store(voice.pos as u64 * 1000 / rate, Ordering::Relaxed);
            self.shared
                .song_duration_ms
                .store(voice.audio.frames as u64 * 1000 / rate, Ordering::Relaxed);
        } else {
            self.shared.song_pos_ms.store(0, Ordering::Relaxed);
            self.shared.song_duration_ms.store(0, Ordering::Relaxed);
        }
        if let Some(voice) = &self.background {
            self.shared
                .background_pos_ms
                .store(voice.pos as u64 * 1000 / rate, Ordering::Relaxed);
            self.shared
                .background_duration_ms
                .store(voice.audio.frames as u64 * 1000 / rate, Ordering::Relaxed);
            self.shared
                .background_playing
                .store(!voice.paused, Ordering::Relaxed);
        } else {
            self.shared.background_pos_ms.store(0, Ordering::Relaxed);
            self.shared
                .background_duration_ms
                .store(0, Ordering::Relaxed);
            self.shared
                .background_playing
                .store(false, Ordering::Relaxed);
        }
    }
}

fn publish_peak(atomic: &AtomicU32, value: f32) {
    let scaled = (value.clamp(0.0, 2.0) * 100_000.0) as u32;
    atomic.store(scaled, Ordering::Relaxed);
}

fn ms_to_frames(ms: u64, rate: u32) -> usize {
    if ms == 0 || rate == 0 {
        return 0;
    }
    ((ms as u128 * rate as u128) / 1000).min(usize::MAX as u128) as usize
}

struct OpenedOutput {
    stream: cpal::Stream,
    name: String,
    sample_rate: u32,
    channels: u16,
    warning: Option<String>,
}

fn open_output(
    preferred: Option<&str>,
    commands: Arc<ArrayQueue<Command>>,
    garbage: Arc<ArrayQueue<Arc<DecodedAudio>>>,
    shared: Arc<Shared>,
) -> AudioResult<OpenedOutput> {
    let host = cpal::default_host();
    let default = host
        .default_output_device()
        .ok_or_else(|| AudioError::Device("nessun dispositivo audio di uscita".into()))?;
    let mut warning = None;

    let device = if let Some(requested) = preferred.filter(|name| !name.is_empty()) {
        let mut found = None;
        if let Ok(devices) = host.output_devices() {
            for candidate in devices {
                if candidate.to_string() == requested {
                    found = Some(candidate);
                    break;
                }
            }
        }
        if found.is_none() {
            warning = Some(format!(
                "Uscita audio \"{requested}\" non disponibile: uso il dispositivo di sistema."
            ));
        }
        found.unwrap_or(default)
    } else {
        default
    };

    let name = device.to_string();
    let supported = device
        .default_output_config()
        .map_err(|err| AudioError::Device(format!("configurazione audio: {err}")))?;
    let sample_format = supported.sample_format();
    let config: StreamConfig = supported.config();
    let sample_rate = config.sample_rate;
    let channels = config.channels;
    if channels == 0 {
        return Err(AudioError::Device("dispositivo senza canali".into()));
    }

    let mixer = Mixer::new(sample_rate, channels, commands, garbage, shared.clone());
    let stream = match sample_format {
        SampleFormat::F32 => build_stream::<f32>(&device, &config, mixer, shared),
        SampleFormat::F64 => build_stream::<f64>(&device, &config, mixer, shared),
        SampleFormat::I16 => build_stream::<i16>(&device, &config, mixer, shared),
        SampleFormat::I32 => build_stream::<i32>(&device, &config, mixer, shared),
        SampleFormat::U16 => build_stream::<u16>(&device, &config, mixer, shared),
        other => Err(AudioError::Device(format!(
            "formato di uscita non supportato: {other:?}"
        ))),
    }?;
    stream
        .play()
        .map_err(|err| AudioError::Device(format!("avvio uscita audio: {err}")))?;

    Ok(OpenedOutput {
        stream,
        name,
        sample_rate,
        channels,
        warning,
    })
}

fn build_stream<T>(
    device: &cpal::Device,
    config: &StreamConfig,
    mut mixer: Mixer,
    shared: Arc<Shared>,
) -> AudioResult<cpal::Stream>
where
    T: SizedSample + FromSample<f32> + 'static,
{
    let error_shared = shared;
    device
        .build_output_stream::<T, _, _>(
            *config,
            move |data: &mut [T], _info| mixer.render(data),
            move |err| {
                warn!(error = %err, "audio output stream error");
                if let Ok(mut slot) = error_shared.error.lock() {
                    *slot = Some(format!("Uscita audio interrotta: {err}"));
                }
                error_shared.disconnected.store(true, Ordering::SeqCst);
            },
            None,
        )
        .map_err(|err| AudioError::Device(format!("apertura uscita audio: {err}")))
}

struct Cache {
    entries: HashMap<String, (Arc<DecodedAudio>, u64)>,
    clock: u64,
    limit: usize,
}

impl Cache {
    fn new(limit: usize) -> Self {
        Self {
            entries: HashMap::new(),
            clock: 0,
            limit,
        }
    }

    fn get(&mut self, key: &str) -> Option<Arc<DecodedAudio>> {
        self.clock += 1;
        let clock = self.clock;
        let entry = self.entries.get_mut(key)?;
        entry.1 = clock;
        Some(entry.0.clone())
    }

    fn contains(&self, key: &str) -> bool {
        self.entries.contains_key(key)
    }

    fn insert(&mut self, key: String, audio: Arc<DecodedAudio>) {
        self.clock += 1;
        self.entries.insert(key, (audio, self.clock));
        while self.entries.len() > self.limit {
            let Some(oldest) = self
                .entries
                .iter()
                .min_by_key(|(_, (_, touched))| *touched)
                .map(|(key, _)| key.clone())
            else {
                break;
            };
            self.entries.remove(&oldest);
        }
    }

    fn clear(&mut self) {
        self.entries.clear();
    }

    fn len(&self) -> usize {
        self.entries.len()
    }
}

struct PreparedMessage {
    key: String,
    result: Result<Arc<DecodedAudio>, String>,
}

struct BackgroundProgram {
    audio: Arc<DecodedAudio>,
    looping: bool,
    volume: f32,
}

/// Realtime audio engine. One instance per application.
pub struct AudioEngine {
    shared: Arc<Shared>,
    commands: Arc<ArrayQueue<Command>>,
    garbage: Arc<ArrayQueue<Arc<DecodedAudio>>>,
    stream: Option<cpal::Stream>,
    device_name: Option<String>,
    preferred_device: Option<String>,
    sample_rate: u32,
    channels: u16,
    warning: Option<String>,
    error: Option<String>,
    cache: Cache,
    preparing: HashSet<String>,
    decode_tx: Sender<PreparedMessage>,
    decode_rx: Receiver<PreparedMessage>,
    background: Option<BackgroundProgram>,
    bg_cursor_frames: usize,
    bg_started: bool,
    last_song: Option<Arc<DecodedAudio>>,
    last_song_params: Option<VoiceParams>,
    last_play_active: bool,
    paused: bool,
    master: f32,
    last_connect_attempt: Instant,
}

impl AudioEngine {
    /// Opens the preferred device, or the system default. The engine is still
    /// usable (silent) if no output is available; `status()` explains why.
    pub fn new(preferred_device: Option<String>) -> Self {
        let commands = Arc::new(ArrayQueue::new(COMMAND_CAPACITY));
        let garbage = Arc::new(ArrayQueue::new(GARBAGE_CAPACITY));
        let shared = Arc::new(Shared::new());
        let (decode_tx, decode_rx) = mpsc::channel();
        let mut engine = Self {
            shared,
            commands,
            garbage,
            stream: None,
            device_name: None,
            preferred_device,
            sample_rate: 48_000,
            channels: 2,
            warning: None,
            error: None,
            cache: Cache::new(CACHE_LIMIT),
            preparing: HashSet::new(),
            decode_tx,
            decode_rx,
            background: None,
            bg_cursor_frames: 0,
            bg_started: false,
            last_song: None,
            last_song_params: None,
            last_play_active: false,
            paused: false,
            master: 1.0,
            last_connect_attempt: Instant::now(),
        };
        engine.try_open();
        engine
    }

    fn try_open(&mut self) {
        match open_output(
            self.preferred_device.as_deref(),
            self.commands.clone(),
            self.garbage.clone(),
            self.shared.clone(),
        ) {
            Ok(opened) => {
                info!(
                    device = %opened.name,
                    rate = opened.sample_rate,
                    channels = opened.channels,
                    "audio output opened"
                );
                self.sample_rate = opened.sample_rate;
                self.channels = opened.channels;
                self.device_name = Some(opened.name);
                self.warning = opened.warning;
                self.error = None;
                self.stream = Some(opened.stream);
                if let Ok(mut slot) = self.shared.error.lock() {
                    *slot = None;
                }
                self.shared.disconnected.store(false, Ordering::SeqCst);
                let _ = self.commands.push(Command::SetMasterVolume(self.master));
            }
            Err(err) => {
                self.stream = None;
                self.error = Some(err.to_string());
                warn!(error = %err, "audio output unavailable");
            }
        }
    }

    /// Listing for the settings page. Does not open a stream.
    pub fn output_devices() -> Vec<AudioDeviceInfo> {
        let host = cpal::default_host();
        let default_name = host
            .default_output_device()
            .map(|device| device.to_string());
        let mut out = Vec::new();
        if let Ok(devices) = host.output_devices() {
            for device in devices {
                let name = device.to_string();
                out.push(AudioDeviceInfo {
                    is_default: default_name.as_deref() == Some(name.as_str()),
                    name,
                });
            }
        }
        out
    }

    pub fn status(&self) -> EngineStatus {
        let stream_error = self.shared.error.lock().ok().and_then(|slot| slot.clone());
        EngineStatus {
            device_name: self.device_name.clone(),
            sample_rate: self.sample_rate,
            channels: self.channels,
            available: self.stream.is_some() && !self.shared.disconnected.load(Ordering::SeqCst),
            warning: self.warning.clone(),
            error: stream_error.or_else(|| self.error.clone()),
        }
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn levels(&self) -> Levels {
        Levels {
            song_peak: self.shared.song_peak.load(Ordering::Relaxed) as f32 / 100_000.0,
            background_peak: self.shared.background_peak.load(Ordering::Relaxed) as f32 / 100_000.0,
            song_pos_ms: self.shared.song_pos_ms.load(Ordering::Relaxed),
            song_duration_ms: self.shared.song_duration_ms.load(Ordering::Relaxed),
            background_pos_ms: self.shared.background_pos_ms.load(Ordering::Relaxed),
            background_duration_ms: self.shared.background_duration_ms.load(Ordering::Relaxed),
            background_playing: self.shared.background_playing.load(Ordering::Relaxed),
        }
    }

    pub fn selected_device(&self) -> Option<&str> {
        self.preferred_device.as_deref()
    }

    pub fn set_preferred_device(&mut self, preferred: Option<String>) {
        if self.preferred_device == preferred {
            return;
        }
        self.preferred_device = preferred;
        self.stream = None;
        self.try_open();
    }

    pub fn prepared(&self, key: &str) -> bool {
        self.cache.contains(key)
    }

    pub fn prepared_count(&self) -> usize {
        self.cache.len()
    }

    /// Decodes off the UI thread and publishes the result through `poll`.
    pub fn prepare(
        &mut self,
        key: String,
        path: PathBuf,
        start_ms: u64,
        end_ms: Option<u64>,
    ) -> AudioResult<()> {
        if self.cache.contains(&key) || self.preparing.contains(&key) {
            return Ok(());
        }
        let rate = self.sample_rate;
        let tx = self.decode_tx.clone();
        let thread_key = key.clone();
        self.preparing.insert(key.clone());
        let spawn = std::thread::Builder::new()
            .name("nerd-decode".into())
            .spawn(move || {
                let result = decode_stereo(&path, (start_ms, end_ms), rate)
                    .map(Arc::new)
                    .map_err(|err| err.to_string());
                let _ = tx.send(PreparedMessage {
                    key: thread_key,
                    result,
                });
            });
        if let Err(err) = spawn {
            self.preparing.remove(&key);
            return Err(AudioError::Decode(format!(
                "impossibile avviare il decoder: {err}"
            )));
        }
        Ok(())
    }

    /// Synchronous fallback used when a GO arrives before preloading finished.
    pub fn prepare_now(
        &mut self,
        key: String,
        path: &Path,
        start_ms: u64,
        end_ms: Option<u64>,
    ) -> AudioResult<()> {
        if self.cache.contains(&key) {
            return Ok(());
        }
        let audio = decode_stereo(path, (start_ms, end_ms), self.sample_rate)?;
        if !crate::decode::is_usable(&audio) {
            return Err(AudioError::Decode("audio non utilizzabile".into()));
        }
        self.cache.insert(key, Arc::new(audio));
        Ok(())
    }

    /// Drains decode results, recycled buffers and handles device loss.
    pub fn poll(&mut self) -> Vec<AudioEvent> {
        let mut events = Vec::new();
        while self.garbage.pop().is_some() {}

        while let Ok(message) = self.decode_rx.try_recv() {
            self.preparing.remove(&message.key);
            match message.result {
                Ok(audio) => {
                    self.cache.insert(message.key.clone(), audio);
                    events.push(AudioEvent::Prepared { key: message.key });
                }
                Err(error_message) => {
                    events.push(AudioEvent::PrepareFailed {
                        key: message.key,
                        message: error_message,
                    });
                }
            }
        }

        let disconnected = self.shared.disconnected.load(Ordering::SeqCst);
        if disconnected && self.stream.is_some() {
            self.stream = None;
            events.push(AudioEvent::OutputLost);
        }
        if self.stream.is_none() && self.last_connect_attempt.elapsed() >= Duration::from_secs(2) {
            self.last_connect_attempt = Instant::now();
            if self.reconnect() {
                events.push(AudioEvent::OutputRestored);
            }
        }
        events
    }

    fn reconnect(&mut self) -> bool {
        let old_rate = self.sample_rate;
        let song_ms = self.shared.song_pos_ms.load(Ordering::Relaxed);
        let song_frames_ms = self.shared.song_duration_ms.load(Ordering::Relaxed);
        let bg_ms = self.shared.background_pos_ms.load(Ordering::Relaxed);
        while self.commands.pop().is_some() {}
        while self.garbage.pop().is_some() {}

        match open_output(
            self.preferred_device.as_deref(),
            self.commands.clone(),
            self.garbage.clone(),
            self.shared.clone(),
        ) {
            Ok(opened) => {
                info!(device = %opened.name, "audio output restored");
                self.sample_rate = opened.sample_rate;
                self.channels = opened.channels;
                self.device_name = Some(opened.name);
                self.warning = opened.warning;
                self.error = None;
                self.stream = Some(opened.stream);
                if let Ok(mut slot) = self.shared.error.lock() {
                    *slot = None;
                }
                self.shared.disconnected.store(false, Ordering::SeqCst);
                let _ = self.commands.push(Command::SetMasterVolume(self.master));

                if opened.sample_rate != old_rate {
                    self.cache.clear();
                    self.preparing.clear();
                    self.warning = Some(
                        "La nuova uscita audio usa una frequenza diversa: i brani vanno preparati di nuovo."
                            .to_string(),
                    );
                    return true;
                }
                if self.last_play_active && song_ms + 150 < song_frames_ms {
                    if let (Some(audio), Some(params)) =
                        (self.last_song.clone(), self.last_song_params)
                    {
                        if audio.sample_rate == self.sample_rate {
                            let mut replay = params;
                            replay.start_frame = ms_to_frames(song_ms, self.sample_rate);
                            replay.fade_in_ms = 120;
                            let _ = self.commands.push(Command::PlaySong {
                                audio,
                                params: replay,
                            });
                            if self.paused {
                                let _ = self.commands.push(Command::PauseSong);
                            }
                        }
                    }
                }
                if self.bg_started {
                    if let Some(program) = &self.background {
                        let start = if program.audio.frames > 0 {
                            ms_to_frames(bg_ms, self.sample_rate) % program.audio.frames
                        } else {
                            0
                        };
                        let _ = self.commands.push(Command::PlayBackground {
                            audio: program.audio.clone(),
                            volume: program.volume,
                            fade_in_ms: 400,
                            looping: program.looping,
                            start_frame: start,
                        });
                        if self.paused {
                            let _ = self.commands.push(Command::PauseBackground);
                        }
                    }
                }
                true
            }
            Err(err) => {
                self.stream = None;
                self.error = Some(err.to_string());
                false
            }
        }
    }

    fn push(&self, command: Command) -> AudioResult<()> {
        self.commands
            .push(command)
            .map_err(|_| AudioError::QueueFull)
    }

    pub fn play_song(
        &mut self,
        key: &str,
        volume: f32,
        fade_in_ms: u64,
        fade_out_ms: u64,
    ) -> AudioResult<()> {
        let audio = self.cache.get(key).ok_or(AudioError::NotPrepared)?;
        let params = VoiceParams {
            volume: volume.clamp(0.0, 2.0),
            fade_in_ms: fade_in_ms.min(10_000),
            fade_out_ms: fade_out_ms.min(10_000),
            start_frame: 0,
        };
        self.push(Command::PlaySong {
            audio: audio.clone(),
            params,
        })?;
        self.last_song = Some(audio);
        self.last_song_params = Some(params);
        self.last_play_active = true;
        self.paused = false;
        Ok(())
    }

    pub fn stop_song(&mut self, fade_ms: u64) -> AudioResult<()> {
        self.last_play_active = false;
        self.paused = false;
        self.push(Command::StopSong { fade_ms })
    }

    pub fn pause(&mut self) -> AudioResult<()> {
        self.paused = true;
        self.push(Command::PauseSong)
    }

    pub fn resume(&mut self) -> AudioResult<()> {
        self.paused = false;
        self.push(Command::ResumeSong)
    }

    pub fn set_song_volume(&mut self, volume: f32) -> AudioResult<()> {
        self.push(Command::SetSongVolume(volume.clamp(0.0, 2.0)))
    }

    pub fn set_master_volume(&mut self, volume: f32) -> AudioResult<()> {
        self.master = volume.clamp(0.0, 1.0);
        self.push(Command::SetMasterVolume(self.master))
    }

    pub fn set_background_program(&mut self, audio: Arc<DecodedAudio>, looping: bool, volume: f32) {
        self.background = Some(BackgroundProgram {
            audio,
            looping,
            volume: volume.clamp(0.0, 2.0),
        });
        self.bg_cursor_frames = 0;
    }

    pub fn has_background_program(&self) -> bool {
        self.background.is_some()
    }

    /// Drops the prepared playlist. Used when the user removes every
    /// background track, so an old playlist can never start by mistake.
    pub fn clear_background_program(&mut self) {
        self.background = None;
        self.bg_cursor_frames = 0;
        self.bg_started = false;
    }

    pub fn background_duration_ms(&self) -> u64 {
        self.background
            .as_ref()
            .map(|program| program.audio.duration_ms())
            .unwrap_or(0)
    }

    pub fn start_background(&mut self, fade_in_ms: u64, resume: bool) -> AudioResult<()> {
        let program = self
            .background
            .as_ref()
            .ok_or_else(|| AudioError::Decode("sottofondo non preparato".into()))?;
        let start = if resume {
            self.bg_cursor_frames
                .min(program.audio.frames.saturating_sub(1))
        } else {
            0
        };
        let command = Command::PlayBackground {
            audio: program.audio.clone(),
            volume: program.volume,
            fade_in_ms: fade_in_ms.min(30_000),
            looping: program.looping,
            start_frame: start,
        };
        self.bg_started = true;
        self.push(command)
    }

    pub fn stop_background(&mut self, fade_ms: u64) -> AudioResult<()> {
        self.bg_cursor_frames = ms_to_frames(
            self.shared.background_pos_ms.load(Ordering::Relaxed),
            self.sample_rate,
        );
        self.bg_started = false;
        self.push(Command::StopBackground { fade_ms })
    }

    pub fn pause_background(&mut self) -> AudioResult<()> {
        self.paused = true;
        self.push(Command::PauseBackground)
    }

    pub fn resume_background(&mut self) -> AudioResult<()> {
        self.paused = false;
        self.push(Command::ResumeBackground)
    }

    pub fn set_background_volume(&mut self, volume: f32) -> AudioResult<()> {
        if let Some(program) = &mut self.background {
            program.volume = volume.clamp(0.0, 2.0);
        }
        self.push(Command::SetBackgroundVolume(volume.clamp(0.0, 2.0)))
    }

    pub fn reset_background_cursor(&mut self) {
        self.bg_cursor_frames = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn audio(rate: u32, frames: usize, value: f32) -> Arc<DecodedAudio> {
        Arc::new(DecodedAudio::new(rate, vec![value; frames * 2]))
    }

    fn mixer(rate: u32) -> Mixer {
        Mixer::new(
            rate,
            2,
            Arc::new(ArrayQueue::new(16)),
            Arc::new(ArrayQueue::new(16)),
            Arc::new(Shared::new()),
        )
    }

    #[test]
    fn voice_fades_in_and_out() {
        let mut voice = Voice::new(audio(1_000, 1_000, 1.0), 1_000, 1.0, 50, 50, 0, false);
        let first = voice.sample().0;
        assert!(first < 0.05, "fade in must start from silence: {first}");
        for _ in 0..60 {
            voice.sample();
        }
        let mid = voice.sample().0;
        assert!(mid > 0.8, "voice should be at full gain, got {mid}");
        voice.fade_out(50);
        for _ in 0..60 {
            voice.sample();
        }
        assert!(voice.finished);
        assert_eq!(voice.sample(), (0.0, 0.0));
    }

    #[test]
    fn mixer_renders_a_queued_song() {
        let mut mixer = mixer(1_000);
        mixer
            .commands
            .push(Command::PlaySong {
                audio: audio(1_000, 1_000, 0.5),
                params: VoiceParams {
                    volume: 1.0,
                    fade_in_ms: 0,
                    fade_out_ms: 0,
                    start_frame: 0,
                },
            })
            .unwrap();
        let mut buffer = vec![0.0f32; 256];
        mixer.render(&mut buffer);
        assert!(buffer.iter().any(|s| s.abs() > 0.1));
        assert!(mixer.shared.song_pos_ms.load(Ordering::Relaxed) > 0);
    }

    #[test]
    fn stop_fade_removes_the_voice() {
        let mut mixer = mixer(1_000);
        mixer
            .commands
            .push(Command::PlaySong {
                audio: audio(1_000, 5_000, 0.5),
                params: VoiceParams {
                    volume: 1.0,
                    fade_in_ms: 0,
                    fade_out_ms: 0,
                    start_frame: 0,
                },
            })
            .unwrap();
        mixer.render(&mut vec![0.0f32; 64]);
        mixer
            .commands
            .push(Command::StopSong { fade_ms: 20 })
            .unwrap();
        let mut buffer = vec![0.0f32; 128];
        for _ in 0..5 {
            mixer.render(&mut buffer);
        }
        assert!(mixer.songs.iter().all(|voice| voice.is_none()));
    }

    #[test]
    fn background_is_separate_from_the_song_bus() {
        let mut mixer = mixer(1_000);
        mixer
            .commands
            .push(Command::PlayBackground {
                audio: audio(1_000, 5_000, 0.2),
                volume: 0.5,
                fade_in_ms: 0,
                looping: true,
                start_frame: 0,
            })
            .unwrap();
        let mut buffer = vec![0.0f32; 64];
        mixer.render(&mut buffer);
        assert!(mixer.background.is_some());
        assert!(mixer.songs.iter().all(|voice| voice.is_none()));
        assert!(buffer.iter().any(|s| s.abs() > 0.01));
    }

    #[test]
    fn cache_evicts_least_recently_used() {
        let mut cache = Cache::new(2);
        cache.insert("a".into(), audio(1_000, 10, 0.1));
        cache.insert("b".into(), audio(1_000, 10, 0.1));
        assert!(cache.get("a").is_some());
        cache.insert("c".into(), audio(1_000, 10, 0.1));
        assert!(cache.get("a").is_some());
        assert!(cache.get("b").is_none());
        assert!(cache.get("c").is_some());
    }
}
