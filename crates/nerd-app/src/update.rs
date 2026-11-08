//! GitHub Releases updater.
//!
//! Flow: query the GitHub API → compare SemVer → download the universal `.zip`
//! with progress → verify SHA-256 and an Ed25519 signature → stage the new app
//! next to the current bundle → a detached shell helper waits for this process
//! to exit, swaps the bundle atomically and relaunches. If anything fails the
//! previous bundle is restored.

use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::time::{Duration, Instant};

use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use nerd_core::prefs::UpdateChannel;
use semver::Version;
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::paths::AppPaths;

/// Ed25519 public key (hex) used to sign the `.sha256` asset. The private key
/// lives only in GitHub Actions secrets.
const UPDATE_PUBLIC_KEY_HEX: &str =
    "1f4e6a8920eaa96f7927e6574be09462681ae24397c5ab186013203ff3ecb40b";

#[derive(Debug, Clone)]
pub struct ReleaseInfo {
    pub version: Version,
    pub tag: String,
    pub name: String,
    pub notes: String,
    pub published_at: String,
    pub zip_url: String,
    pub checksum_url: String,
    pub signature_url: String,
    pub zip_size: u64,
}

#[derive(Debug, Clone)]
pub struct PreparedUpdate {
    pub release: ReleaseInfo,
    pub app_bundle: PathBuf,
}

#[derive(Debug, Clone)]
pub enum UpdateStatus {
    Idle,
    Checking,
    Available(ReleaseInfo),
    Downloading { release: ReleaseInfo, progress: f32 },
    Ready(PreparedUpdate),
    Installing,
    UpToDate,
    Error(String),
}

impl UpdateStatus {
    pub fn is_busy(&self) -> bool {
        matches!(
            self,
            UpdateStatus::Checking | UpdateStatus::Downloading { .. } | UpdateStatus::Installing
        )
    }

    pub fn release(&self) -> Option<&ReleaseInfo> {
        match self {
            UpdateStatus::Available(release) | UpdateStatus::Downloading { release, .. } => {
                Some(release)
            }
            UpdateStatus::Ready(prepared) => Some(&prepared.release),
            _ => None,
        }
    }

    pub fn description(&self) -> String {
        match self {
            UpdateStatus::Idle => "Nessun controllo recente".to_string(),
            UpdateStatus::Checking => "Controllo aggiornamenti…".to_string(),
            UpdateStatus::Available(release) => {
                format!("Disponibile la versione {}", release.version)
            }
            UpdateStatus::Downloading { progress, .. } => {
                format!("Download in corso… {:.0}%", progress * 100.0)
            }
            UpdateStatus::Ready(prepared) => {
                format!(
                    "Versione {} pronta per l'installazione",
                    prepared.release.version
                )
            }
            UpdateStatus::Installing => "Installazione in corso…".to_string(),
            UpdateStatus::UpToDate => "Hai già l'ultima versione".to_string(),
            UpdateStatus::Error(error) => error.clone(),
        }
    }
}

pub struct Updater {
    pub status: UpdateStatus,
    rx: Option<Receiver<UpdateMessage>>,
    manual: bool,
}

enum UpdateMessage {
    CheckDone(Result<Option<ReleaseInfo>, String>),
    Progress(f32),
    DownloadDone(Result<PreparedUpdate, String>),
}

impl Updater {
    pub fn new() -> Self {
        Self {
            status: UpdateStatus::Idle,
            rx: None,
            manual: false,
        }
    }

    pub fn is_busy(&self) -> bool {
        self.status.is_busy() || self.rx.is_some()
    }

    pub fn available(&self) -> Option<&ReleaseInfo> {
        self.status.release()
    }

    pub fn check(&mut self, repo: &str, channel: UpdateChannel, manual: bool) {
        if self.is_busy() {
            return;
        }
        let repo = repo.trim();
        if repo.is_empty() || !repo.contains('/') {
            self.status = UpdateStatus::Error(
                "Configura un repository GitHub nel formato owner/repo.".to_string(),
            );
            return;
        }
        let (tx, rx) = mpsc::channel();
        self.rx = Some(rx);
        self.manual = manual;
        self.status = UpdateStatus::Checking;
        let repo = repo.to_string();
        std::thread::Builder::new()
            .name("nerd-update-check".into())
            .spawn(move || {
                let result = check_latest(&repo, channel);
                let _ = tx.send(UpdateMessage::CheckDone(result));
            })
            .ok();
    }

    pub fn download(&mut self, repo: &str, release: ReleaseInfo) {
        if self.is_busy() {
            return;
        }
        let (tx, rx) = mpsc::channel();
        self.rx = Some(rx);
        self.status = UpdateStatus::Downloading {
            release: release.clone(),
            progress: 0.0,
        };
        let repo = repo.to_string();
        std::thread::Builder::new()
            .name("nerd-update-download".into())
            .spawn(move || {
                let result = download_update(&repo, &release, &tx);
                let _ = tx.send(UpdateMessage::DownloadDone(result));
            })
            .ok();
    }

    /// Applies worker messages. Call once per frame.
    pub fn poll(&mut self) {
        let mut messages = Vec::new();
        let disconnected = if let Some(rx) = &self.rx {
            loop {
                match rx.try_recv() {
                    Ok(message) => messages.push(message),
                    Err(TryRecvError::Empty) => break false,
                    Err(TryRecvError::Disconnected) => break true,
                }
            }
        } else {
            false
        };
        for message in messages {
            match message {
                UpdateMessage::CheckDone(Ok(Some(release))) => {
                    self.status = UpdateStatus::Available(release);
                    self.rx = None;
                }
                UpdateMessage::CheckDone(Ok(None)) => {
                    self.status = if self.manual {
                        UpdateStatus::UpToDate
                    } else {
                        UpdateStatus::Idle
                    };
                    self.rx = None;
                }
                UpdateMessage::CheckDone(Err(error)) => {
                    self.status = UpdateStatus::Error(error);
                    self.rx = None;
                }
                UpdateMessage::Progress(progress) => {
                    if let UpdateStatus::Downloading { release, .. } = &self.status {
                        let release = release.clone();
                        self.status = UpdateStatus::Downloading { release, progress };
                    }
                }
                UpdateMessage::DownloadDone(Ok(prepared)) => {
                    self.status = UpdateStatus::Ready(prepared);
                    self.rx = None;
                }
                UpdateMessage::DownloadDone(Err(error)) => {
                    self.status = UpdateStatus::Error(error);
                    self.rx = None;
                }
            }
        }
        if disconnected {
            self.rx = None;
        }
    }

    /// Stages the bundle beside the running app and launches the detached
    /// helper. The caller must quit immediately afterwards.
    pub fn install(&mut self) -> Result<(), String> {
        let prepared = match &self.status {
            UpdateStatus::Ready(prepared) => prepared.clone(),
            UpdateStatus::Available(release) => {
                return Err(format!("Scarica prima la versione {}.", release.version))
            }
            _ => return Err("Nessun aggiornamento pronto.".to_string()),
        };
        let target = current_bundle().ok_or_else(|| {
            "L'app non è in un bundle .app: apri il nuovo .app manualmente dal Finder.".to_string()
        })?;
        let target_dir = target
            .parent()
            .ok_or_else(|| "Percorso dell'app non valido.".to_string())?
            .to_path_buf();
        let staging = target_dir.join(format!(".{}-update.app", nerd_core::APP_ID));
        let backup = target_dir.join(format!(".{}-backup.app", nerd_core::APP_ID));
        if staging.exists() {
            let _ = fs::remove_dir_all(&staging);
        }
        let copy = Command::new("/usr/bin/ditto")
            .arg(&prepared.app_bundle)
            .arg(&staging)
            .status()
            .map_err(|err| format!("Impossibile preparare l'aggiornamento: {err}"))?;
        if !copy.success() {
            let _ = fs::remove_dir_all(&staging);
            return Err("Copia del nuovo bundle non riuscita.".to_string());
        }
        if backup.exists() {
            let _ = fs::remove_dir_all(&backup);
        }
        let script =
            std::env::temp_dir().join(format!("project-nerd-install-{}.sh", std::process::id()));
        let script_body = r#"#!/bin/sh
set -u
pid="$1"
target="$2"
staging="$3"
backup="$4"
while kill -0 "$pid" 2>/dev/null; do sleep 0.4; done
rm -rf "$backup"
if ! mv "$target" "$backup"; then
  exit 1
fi
if mv "$staging" "$target"; then
  /usr/bin/xattr -dr com.apple.quarantine "$target" 2>/dev/null || true
  if /usr/bin/open "$target"; then
    rm -rf "$backup"
    exit 0
  fi
  rm -rf "$target"
  mv "$backup" "$target"
  /usr/bin/open "$target" 2>/dev/null || true
  exit 1
fi
mv "$backup" "$target"
/usr/bin/open "$target" 2>/dev/null || true
exit 1
"#;
        fs::write(&script, script_body)
            .map_err(|err| format!("Impossibile scrivere l'helper: {err}"))?;
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&script, fs::Permissions::from_mode(0o755));
        let spawn = Command::new("/bin/sh")
            .arg(&script)
            .arg(std::process::id().to_string())
            .arg(&target)
            .arg(&staging)
            .arg(&backup)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
        match spawn {
            Ok(_) => {
                self.status = UpdateStatus::Installing;
                tracing::info!(
                    version = %prepared.release.version,
                    target = %target.display(),
                    "update staged; helper launched"
                );
                Ok(())
            }
            Err(err) => {
                let _ = fs::remove_dir_all(&staging);
                Err(format!("Impossibile avviare l'installazione: {err}"))
            }
        }
    }
}

fn http_client() -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .user_agent(format!(
            "ProjectNerd/{} (+https://github.com/)",
            env!("CARGO_PKG_VERSION")
        ))
        .timeout(Duration::from_secs(45))
        .build()
        .map_err(|err| format!("Client HTTP: {err}"))
}

fn check_latest(repo: &str, channel: UpdateChannel) -> Result<Option<ReleaseInfo>, String> {
    let client = http_client()?;
    let current = Version::parse(env!("CARGO_PKG_VERSION"))
        .map_err(|err| format!("Versione locale non valida: {err}"))?;
    let url = if channel == UpdateChannel::Alpha {
        format!("https://api.github.com/repos/{repo}/releases?per_page=30")
    } else {
        format!("https://api.github.com/repos/{repo}/releases/latest")
    };
    let response = client
        .get(&url)
        .header("Accept", "application/vnd.github+json")
        .send()
        .map_err(|err| format!("Rete non disponibile: {err}"))?;
    let status = response.status();
    if status == reqwest::StatusCode::FORBIDDEN || status == reqwest::StatusCode::TOO_MANY_REQUESTS
    {
        return Err(
            "GitHub ha limitato le richieste. Riprova più tardi o controlla i limiti API."
                .to_string(),
        );
    }
    if status == reqwest::StatusCode::NOT_FOUND {
        // GitHub returns 404 for `/releases/latest` while only prereleases exist.
        if channel == UpdateChannel::Stable {
            return Ok(None);
        }
        return Err(format!("Release non trovata per {repo}."));
    }
    if !status.is_success() {
        return Err(format!("GitHub ha risposto con HTTP {status}."));
    }
    let body: Value = response
        .json()
        .map_err(|err| format!("Risposta GitHub non valida: {err}"))?;
    let candidates: Vec<&Value> = if body.is_array() {
        body.as_array()
            .map(|a| a.iter().collect())
            .unwrap_or_default()
    } else {
        vec![&body]
    };
    let mut best: Option<ReleaseInfo> = None;
    for candidate in candidates {
        if candidate
            .get("draft")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            continue;
        }
        let prerelease = candidate
            .get("prerelease")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        if prerelease && !channel.accepts_prerelease() {
            continue;
        }
        let Some(tag) = candidate.get("tag_name").and_then(Value::as_str) else {
            continue;
        };
        let Ok(version) = Version::parse(tag.trim_start_matches('v')) else {
            continue;
        };
        if version <= current {
            continue;
        }
        if let Some(release) = parse_release(candidate, tag, version) {
            if best
                .as_ref()
                .map(|existing| release.version > existing.version)
                .unwrap_or(true)
            {
                best = Some(release);
            }
        }
    }
    Ok(best)
}

fn parse_release(value: &Value, tag: &str, version: Version) -> Option<ReleaseInfo> {
    let assets = value.get("assets")?.as_array()?;
    let asset_url = |predicate: &dyn Fn(&str) -> bool| -> Option<(String, u64)> {
        assets.iter().find_map(|asset| {
            let name = asset.get("name")?.as_str()?;
            if predicate(name) {
                Some((
                    asset.get("browser_download_url")?.as_str()?.to_string(),
                    asset.get("size").and_then(Value::as_u64).unwrap_or(0),
                ))
            } else {
                None
            }
        })
    };
    let (zip_url, zip_size) = asset_url(&|name| name.to_ascii_lowercase().ends_with(".zip"))?;
    let (checksum_url, _) = asset_url(&|name| name.to_ascii_lowercase().ends_with(".sha256"))?;
    let (signature_url, _) = asset_url(&|name| {
        let name = name.to_ascii_lowercase();
        name.ends_with(".sig") || name.ends_with(".ed25519")
    })?;
    Some(ReleaseInfo {
        version,
        tag: tag.to_string(),
        name: value
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or(tag)
            .to_string(),
        notes: value
            .get("body")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        published_at: value
            .get("published_at")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        zip_url,
        checksum_url,
        signature_url,
        zip_size,
    })
}

fn download_update(
    repo: &str,
    release: &ReleaseInfo,
    tx: &mpsc::Sender<UpdateMessage>,
) -> Result<PreparedUpdate, String> {
    let _ = repo;
    let client = http_client()?;
    let work_dir = std::env::temp_dir().join(format!(
        "ProjectNerd-{}-{}",
        release.version,
        std::process::id()
    ));
    if work_dir.exists() {
        let _ = fs::remove_dir_all(&work_dir);
    }
    fs::create_dir_all(&work_dir).map_err(|err| format!("Cartella temporanea: {err}"))?;
    let zip_path = work_dir.join("update.zip");
    download_file(&client, &release.zip_url, &zip_path, Some(tx))?;
    let checksum_bytes = fetch_bytes(&client, &release.checksum_url)?;
    let signature_bytes = fetch_bytes(&client, &release.signature_url)?;
    verify_update(&zip_path, &checksum_bytes, &signature_bytes)?;
    let extract_dir = work_dir.join("extracted");
    fs::create_dir_all(&extract_dir).map_err(|err| format!("Cartella di estrazione: {err}"))?;
    let status = Command::new("/usr/bin/ditto")
        .arg("-x")
        .arg("-k")
        .arg(&zip_path)
        .arg(&extract_dir)
        .status()
        .map_err(|err| format!("Estrazione non riuscita: {err}"))?;
    if !status.success() {
        return Err("Il pacchetto .zip non è valido.".to_string());
    }
    let app_bundle = find_app(&extract_dir)
        .ok_or_else(|| "Il pacchetto non contiene un'app .app.".to_string())?;
    if !app_bundle.join("Contents").join("Info.plist").is_file() {
        return Err("Il bundle scaricato non è un'app macOS valida.".to_string());
    }
    Ok(PreparedUpdate {
        release: release.clone(),
        app_bundle,
    })
}

fn fetch_bytes(client: &reqwest::blocking::Client, url: &str) -> Result<Vec<u8>, String> {
    let mut response = client
        .get(url)
        .send()
        .map_err(|err| format!("Download non riuscito: {err}"))?;
    if !response.status().is_success() {
        return Err(format!("Download HTTP {}.", response.status()));
    }
    let mut bytes = Vec::new();
    response
        .read_to_end(&mut bytes)
        .map_err(|err| format!("Lettura download: {err}"))?;
    Ok(bytes)
}

fn download_file(
    client: &reqwest::blocking::Client,
    url: &str,
    path: &Path,
    progress: Option<&mpsc::Sender<UpdateMessage>>,
) -> Result<(), String> {
    let mut response = client
        .get(url)
        .send()
        .map_err(|err| format!("Download non riuscito: {err}"))?;
    if !response.status().is_success() {
        return Err(format!("Download HTTP {}.", response.status()));
    }
    let total = response.content_length().unwrap_or(0).max(1);
    let mut file = fs::File::create(path).map_err(|err| format!("File temporaneo: {err}"))?;
    let mut buffer = [0u8; 64 * 1024];
    let mut downloaded = 0u64;
    let mut last_report = Instant::now();
    loop {
        let read = response
            .read(&mut buffer)
            .map_err(|err| format!("Lettura download: {err}"))?;
        if read == 0 {
            break;
        }
        file.write_all(&buffer[..read])
            .map_err(|err| format!("Scrittura download: {err}"))?;
        downloaded += read as u64;
        if let Some(tx) = progress {
            if last_report.elapsed() > Duration::from_millis(120) {
                last_report = Instant::now();
                let _ = tx.send(UpdateMessage::Progress(
                    (downloaded as f32 / total as f32).clamp(0.0, 0.99),
                ));
            }
        }
    }
    file.flush()
        .map_err(|err| format!("Scrittura download: {err}"))?;
    if let Some(tx) = progress {
        let _ = tx.send(UpdateMessage::Progress(1.0));
    }
    Ok(())
}

fn verify_update(
    zip_path: &Path,
    checksum_file: &[u8],
    signature_file: &[u8],
) -> Result<(), String> {
    let key_bytes = hex::decode(UPDATE_PUBLIC_KEY_HEX)
        .map_err(|err| format!("Chiave pubblica OTA non valida: {err}"))?;
    if key_bytes.iter().all(|b| *b == 0) {
        return Err(
            "La chiave pubblica OTA non è configurata in questa build: installazione annullata."
                .to_string(),
        );
    }
    let key_array: [u8; 32] = key_bytes
        .as_slice()
        .try_into()
        .map_err(|_| "Chiave pubblica OTA di lunghezza errata.".to_string())?;
    let verifying_key = VerifyingKey::from_bytes(&key_array)
        .map_err(|err| format!("Chiave pubblica OTA non valida: {err}"))?;
    let checksum_text = std::str::from_utf8(checksum_file)
        .map_err(|_| "File SHA-256 non leggibile.".to_string())?;
    let expected = checksum_text
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    if expected.len() != 64 {
        return Err("File SHA-256 malformato.".to_string());
    }

    let mut hasher = Sha256::new();
    let mut file = fs::File::open(zip_path).map_err(|err| format!("Apertura zip: {err}"))?;
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|err| format!("Lettura zip: {err}"))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    let actual = hex::encode(hasher.finalize());
    if actual != expected {
        return Err(format!(
            "Checksum non corrispondente: atteso {expected}, ottenuto {actual}."
        ));
    }

    let signature = parse_signature(signature_file)?;
    verifying_key
        .verify(checksum_file, &signature)
        .map_err(|_| "Firma Ed25519 non valida: download rifiutato.".to_string())?;
    Ok(())
}

fn parse_signature(bytes: &[u8]) -> Result<Signature, String> {
    if bytes.len() == 64 {
        let array: [u8; 64] = bytes.try_into().unwrap();
        return Ok(Signature::from_bytes(&array));
    }
    if let Ok(text) = std::str::from_utf8(bytes) {
        let trimmed = text.trim();
        if trimmed.len() == 128 {
            if let Ok(decoded) = hex::decode(trimmed) {
                if decoded.len() == 64 {
                    let array: [u8; 64] = decoded.try_into().unwrap();
                    return Ok(Signature::from_bytes(&array));
                }
            }
        }
    }
    Err("File di firma malformato (attesi 64 byte o 128 caratteri esadecimali).".to_string())
}

fn find_app(root: &Path) -> Option<PathBuf> {
    if root.extension().and_then(|e| e.to_str()) == Some("app") {
        return Some(root.to_path_buf());
    }
    let mut entries: Vec<PathBuf> = fs::read_dir(root)
        .ok()?
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .collect();
    entries.sort();
    for entry in &entries {
        if entry.extension().and_then(|e| e.to_str()) == Some("app") {
            return Some(entry.clone());
        }
    }
    for entry in entries {
        if entry.is_dir() {
            if let Some(found) = find_app(&entry) {
                return Some(found);
            }
        }
    }
    None
}

fn current_bundle() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    exe.ancestors()
        .nth(3)
        .filter(|path| path.extension().and_then(|e| e.to_str()) == Some("app"))
        .map(|path| path.to_path_buf())
}

/// Removes stale staging directories after a previous failed attempt.
pub fn cleanup_stale(paths: &AppPaths) {
    let _ = fs::remove_dir_all(paths.cache.join("updates"));
}
