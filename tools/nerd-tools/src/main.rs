//! Build and maintenance tools.
//!
//! Commands:
//!
//! * `keygen --out FILE` creates an Ed25519 OTA key pair.
//! * `sign --key FILE --file FILE [--out FILE]` signs a checksum file.
//! * `verify --key HEX_OR_FILE --file FILE --sig FILE` checks a signature.
//! * `gen-media --out DIR` writes synthetic WAV files and a test evening.
//! * `soak [--minutes N] [--songs N]` simulates a long automatic show.

use std::fs::{self, File, OpenOptions};
use std::io::{BufWriter, Read, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use nerd_core::model::{PlaybackMode, Project, Song};
use nerd_core::persistence;
use nerd_core::{Effect, GoOutcome, Phase, Show};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("nerd-tools: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(command) = args.first() else {
        usage();
        return Ok(());
    };
    match command.as_str() {
        "keygen" => {
            let out = value(&args, "--out").unwrap_or_else(|| "ota-private.key".to_string());
            keygen(Path::new(&out))
        }
        "sign" => {
            let key = required(&args, "--key")?;
            let file = required(&args, "--file")?;
            let out = value(&args, "--out");
            sign(
                Path::new(&key),
                Path::new(&file),
                out.as_deref().map(Path::new),
            )
        }
        "verify" => {
            let key = required(&args, "--key")?;
            let file = required(&args, "--file")?;
            let sig = required(&args, "--sig")?;
            verify(&key, Path::new(&file), Path::new(&sig))
        }
        "gen-media" => {
            let out = value(&args, "--out").unwrap_or_else(|| "test-media".to_string());
            gen_media(Path::new(&out))
        }
        "soak" => {
            let minutes = value(&args, "--minutes")
                .and_then(|value| value.parse::<u64>().ok())
                .unwrap_or(120);
            let songs = value(&args, "--songs")
                .and_then(|value| value.parse::<usize>().ok())
                .unwrap_or(24);
            soak(minutes, songs)
        }
        "help" | "--help" | "-h" => {
            usage();
            Ok(())
        }
        other => Err(format!("comando sconosciuto: {other}")),
    }
}

fn usage() {
    println!(
        "nerd-tools\n\
         \n\
         keygen --out FILE                         crea coppia Ed25519 (privata + FILE.pub)\n\
         sign --key FILE --file FILE [--out FILE]  firma il file SHA-256 per l'OTA\n\
         verify --key HEX|FILE --file FILE --sig FILE  verifica una firma\n\
         gen-media --out DIR                       genera WAV sintetici e una serata di prova\n\
         soak [--minutes 120] [--songs 24]          simula lo show automatico"
    );
}

fn value(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|arg| arg == name)
        .and_then(|index| args.get(index + 1).cloned())
}

fn required(args: &[String], name: &str) -> Result<String, String> {
    value(args, name).ok_or_else(|| format!("manca {name}"))
}

// ---------------------------------------------------------------------------
// OTA keys
// ---------------------------------------------------------------------------

fn keygen(out: &Path) -> Result<(), String> {
    let mut private = [0u8; 32];
    File::open("/dev/urandom")
        .and_then(|mut file| file.read_exact(&mut private))
        .map_err(|err| format!("entropia di sistema: {err}"))?;
    let signing = SigningKey::from_bytes(&private);
    let public = signing.verifying_key().to_bytes();

    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent).map_err(|err| format!("cartella chiave: {err}"))?;
    }
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(out)
        .map_err(|err| format!("creazione chiave privata: {err}"))?;
    writeln!(file, "{}", hex::encode(private))
        .map_err(|err| format!("scrittura chiave privata: {err}"))?;

    let public_path = PathBuf::from(format!("{}.pub", out.display()));
    fs::write(&public_path, format!("{}\n", hex::encode(public)))
        .map_err(|err| format!("scrittura chiave pubblica: {err}"))?;

    println!("chiave privata: {}", out.display());
    println!("chiave pubblica: {}", public_path.display());
    println!(
        "\nIncolla questa chiave pubblica in UPDATE_PUBLIC_KEY_HEX:\n{}",
        hex::encode(public)
    );
    println!("\nMetti la chiave privata nel secret GitHub OTA_PRIVATE_KEY e non condividerla.");
    Ok(())
}

fn sign(key_path: &Path, file_path: &Path, out: Option<&Path>) -> Result<(), String> {
    let key_text = fs::read_to_string(key_path)
        .map_err(|err| format!("lettura chiave {}: {err}", key_path.display()))?;
    let signing = signing_key(key_text.trim())?;
    let data = fs::read(file_path)
        .map_err(|err| format!("lettura file {}: {err}", file_path.display()))?;
    let signature = signing.sign(&data);
    let target = out
        .map(|path| path.to_path_buf())
        .unwrap_or_else(|| PathBuf::from(format!("{}.sig", file_path.display())));
    fs::write(&target, format!("{}\n", hex::encode(signature.to_bytes())))
        .map_err(|err| format!("scrittura firma: {err}"))?;
    println!("firma: {}", target.display());
    Ok(())
}

fn verify(key: &str, file_path: &Path, sig_path: &Path) -> Result<(), String> {
    let public_text = if Path::new(key).is_file() {
        fs::read_to_string(key).map_err(|err| format!("lettura chiave: {err}"))?
    } else {
        key.to_string()
    };
    let bytes =
        hex::decode(public_text.trim()).map_err(|err| format!("chiave non valida: {err}"))?;
    let array: [u8; 32] = bytes
        .as_slice()
        .try_into()
        .map_err(|_| "la chiave pubblica deve essere di 32 byte".to_string())?;
    let verifying = VerifyingKey::from_bytes(&array)
        .map_err(|err| format!("chiave pubblica non valida: {err}"))?;
    let data = fs::read(file_path).map_err(|err| format!("lettura file: {err}"))?;
    let sig_text = fs::read_to_string(sig_path).map_err(|err| format!("lettura firma: {err}"))?;
    let sig_bytes =
        hex::decode(sig_text.trim()).map_err(|err| format!("firma non valida: {err}"))?;
    let sig_array: [u8; 64] = sig_bytes
        .as_slice()
        .try_into()
        .map_err(|_| "la firma deve essere di 64 byte".to_string())?;
    let signature = Signature::from_bytes(&sig_array);
    verifying
        .verify(&data, &signature)
        .map_err(|_| "firma Ed25519 non valida".to_string())?;
    println!("firma valida");
    Ok(())
}

fn signing_key(text: &str) -> Result<SigningKey, String> {
    let bytes = hex::decode(text).map_err(|err| format!("chiave privata non valida: {err}"))?;
    let array: [u8; 32] = bytes
        .as_slice()
        .try_into()
        .map_err(|_| "la chiave privata deve essere di 32 byte".to_string())?;
    Ok(SigningKey::from_bytes(&array))
}

// ---------------------------------------------------------------------------
// Synthetic media
// ---------------------------------------------------------------------------

fn gen_media(out: &Path) -> Result<(), String> {
    fs::create_dir_all(out).map_err(|err| format!("cartella media: {err}"))?;
    let rate = 44_100u32;
    let tracks = [
        ("01-tono-la.wav", 8.0, Tone::Sine(440.0)),
        (
            "02-accordo-la.wav",
            12.0,
            Tone::Chord(&[440.0, 554.37, 659.25]),
        ),
        ("03-glissando.wav", 10.0, Tone::Sweep(220.0, 880.0)),
        ("04-pausa.wav", 6.0, Tone::Sine(220.0)),
    ];

    let mut project = Project::new("Test media");
    project.venue = "Sala prove".to_string();
    for (name, seconds, tone) in tracks {
        let path = out.join(name);
        write_wav(&path, rate, seconds, tone)?;
        let mut song = Song {
            title: name.trim_end_matches(".wav").replace('-', " "),
            audio: Some(path),
            duration_ms: (seconds * 1000.0) as u64,
            ..Song::default()
        };
        song.info.authors = "Generato da nerd-tools".to_string();
        project.add_song(song);
    }
    let project_path = out.join("test-media.nerdshow");
    persistence::save_project(&project, &project_path)
        .map_err(|err| format!("salvataggio serata di prova: {err}"))?;
    println!("media sintetici in {}", out.display());
    println!("serata di prova: {}", project_path.display());
    Ok(())
}

#[derive(Clone, Copy)]
enum Tone<'a> {
    Sine(f32),
    Chord(&'a [f32]),
    Sweep(f32, f32),
}

fn write_wav(path: &Path, rate: u32, seconds: f32, tone: Tone<'_>) -> Result<(), String> {
    let frames = (seconds * rate as f32) as u32;
    let data_len = frames * 2 * 2;
    let mut file = BufWriter::new(
        File::create(path).map_err(|err| format!("creazione {}: {err}", path.display()))?,
    );
    let write = |file: &mut BufWriter<File>, bytes: &[u8]| -> Result<(), String> {
        file.write_all(bytes)
            .map_err(|err| format!("scrittura WAV: {err}"))
    };
    write(&mut file, b"RIFF")?;
    write(&mut file, &(36 + data_len).to_le_bytes())?;
    write(&mut file, b"WAVEfmt ")?;
    write(&mut file, &16u32.to_le_bytes())?;
    write(&mut file, &1u16.to_le_bytes())?;
    write(&mut file, &2u16.to_le_bytes())?;
    write(&mut file, &rate.to_le_bytes())?;
    write(&mut file, &(rate * 4).to_le_bytes())?;
    write(&mut file, &4u16.to_le_bytes())?;
    write(&mut file, &16u16.to_le_bytes())?;
    write(&mut file, b"data")?;
    write(&mut file, &data_len.to_le_bytes())?;

    for frame in 0..frames {
        let t = frame as f32 / rate as f32;
        let value = match tone {
            Tone::Sine(frequency) => (std::f32::consts::TAU * frequency * t).sin(),
            Tone::Chord(frequencies) => {
                let sum: f32 = frequencies
                    .iter()
                    .map(|frequency| (std::f32::consts::TAU * frequency * t).sin())
                    .sum();
                sum / frequencies.len() as f32
            }
            Tone::Sweep(start, end) => {
                let progress = t / seconds;
                let frequency = start + (end - start) * progress;
                (std::f32::consts::TAU * frequency * t).sin()
            }
        };
        let fade = (t / 0.25).min((seconds - t) / 0.25).clamp(0.0, 1.0);
        let sample = (value * fade * 0.24 * i16::MAX as f32) as i16;
        for _ in 0..2 {
            write(&mut file, &sample.to_le_bytes())?;
        }
    }
    file.flush().map_err(|err| format!("chiusura WAV: {err}"))?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Soak simulation
// ---------------------------------------------------------------------------

fn soak(minutes: u64, songs: usize) -> Result<(), String> {
    if songs == 0 {
        return Err("servono almeno un brano".to_string());
    }
    let mut project = Project::new("Soak");
    project.show.mode = PlaybackMode::Automatic;
    project.show.default_gap_ms = 1_200;
    project.background.tracks = vec![PathBuf::from("/dev/null")];
    for index in 0..songs {
        let mut song = Song {
            title: format!("Brano {index:02}"),
            duration_ms: 150_000 + (index as u64 % 5) * 20_000,
            ..Song::default()
        };
        song.audio = Some(PathBuf::from(format!("/tmp/soak-{index:02}.wav")));
        project.add_song(song);
    }

    let mut show = Show::from_project(&project);
    let first = show.go();
    let mut started = match first {
        GoOutcome::Started(effects) => effects.len(),
        _ => return Err("lo show non è partito".to_string()),
    };
    let mut restarts = 0usize;
    let ticks = minutes.saturating_mul(60_000) / 16;
    for tick in 0..ticks {
        let effects = show.tick(16);
        started += effects
            .iter()
            .filter(|effect| matches!(effect, Effect::PlaySong { .. }))
            .count();
        if show.phase() == Phase::Finished {
            // The real app keeps the background running and waits for Stop;
            // the simulation restarts the running order to test many cycles.
            let encore = show.go();
            if matches!(encore, GoOutcome::Started(_)) {
                restarts += 1;
            } else {
                return Err(format!("lo show si è fermato al tick {tick}"));
            }
        }
    }
    println!(
        "soak OK: {} minuti simulati, {} brani avviati, {} bis, fase finale {:?}",
        minutes,
        started,
        restarts,
        show.phase()
    );
    Ok(())
}
