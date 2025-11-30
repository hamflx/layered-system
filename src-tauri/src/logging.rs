use std::{fs, path::Path, sync::Mutex};

use once_cell::sync::OnceCell;
use tracing_appender::{
    non_blocking::{NonBlocking, WorkerGuard},
    rolling,
};
use tracing_subscriber::{
    fmt::{self, format::DefaultFields, format::Format, format::Full},
    layer::SubscriberExt,
    reload, EnvFilter, Registry,
};

use crate::error::{AppError, Result};

type LoggingLayer<S> = fmt::Layer<S, DefaultFields, Format<Full>, NonBlocking>;
type LogHandle = reload::Handle<LoggingLayer<Registry>, Registry>;

static LOG_GUARD: OnceCell<Mutex<Option<WorkerGuard>>> = OnceCell::new();
static LOG_HANDLE: OnceCell<LogHandle> = OnceCell::new();

/// Initialize tracing subscriber writing to the given log file path.
pub fn init_tracing(log_path: &Path) -> Result<()> {
    let (layer, guard) = build_logging_layer(log_path)?;

    if let Some(handle) = LOG_HANDLE.get() {
        handle
            .reload(layer)
            .map_err(|e| AppError::Message(format!("tracing reload failed: {e}")))?;
    } else {
        let env_filter =
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
        let (reloadable_layer, handle): (_, LogHandle) = reload::Layer::new(layer);

        let subscriber = Registry::default().with(reloadable_layer).with(env_filter);

        tracing::subscriber::set_global_default(subscriber)
            .map_err(|e| AppError::Message(format!("tracing init failed: {e}")))?;

        let _ = LOG_HANDLE.set(handle);
    }

    // Keep the background logging worker alive for the active writer.
    *LOG_GUARD
        .get_or_init(|| Mutex::new(None))
        .lock()
        .expect("logging guard poisoned") = Some(guard);
    Ok(())
}

fn build_logging_layer(log_path: &Path) -> Result<(LoggingLayer<Registry>, WorkerGuard)> {
    if let Some(parent) = log_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let file_name = log_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("app.log");
    let dir = log_path
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    let rolling = rolling::never(dir, file_name);
    let (writer, guard) = tracing_appender::non_blocking(rolling);

    let layer: LoggingLayer<Registry> = fmt::Layer::default().with_writer(writer).with_ansi(false);

    Ok((layer, guard))
}
