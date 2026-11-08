//! File logging. No telemetry, no network.

use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::EnvFilter;

use crate::paths::AppPaths;

pub fn init(paths: &AppPaths) -> WorkerGuard {
    let appender = tracing_appender::rolling::daily(&paths.logs, "project-nerd.log");
    let (writer, guard) = tracing_appender::non_blocking(appender);
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,nerd_audio=info,nerd_video=info"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(writer)
        .with_ansi(false)
        .with_target(true)
        .try_init();
    tracing::info!(
        version = nerd_core::APP_VERSION,
        support = %paths.support.display(),
        "Project Nerd starting"
    );
    guard
}
