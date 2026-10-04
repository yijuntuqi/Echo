//! Model download orchestration.
//!
//! Everything AppHandle-shaped lives here — path resolution, the download
//! trigger, and the `model:*` events — so [`crate::embedding`] stays a pure
//! service with no Tauri types in it.

use std::sync::OnceLock;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::embedding::{EmbeddingService, MODEL_ID};

/// Cache directory name for the model: the short name inside [`MODEL_ID`]
/// (`BAAI/bge-small-zh-v1.5` -> `bge-small-zh-v1.5`).
fn model_dir_name() -> &'static str {
    MODEL_ID.rsplit('/').next().unwrap_or(MODEL_ID)
}

/// Resolve the model cache directory under the app data dir.
fn model_cache_dir(app: &AppHandle) -> Option<std::path::PathBuf> {
    Some(app.path().app_data_dir().ok()?.join("models").join(model_dir_name()))
}

/// Resolve the models directory and publish the service into
/// [`crate::state`]. Called once from the setup hook; afterwards every
/// command can rely on `state().embedding` being set.
pub fn init(app: &AppHandle) {
    let Some(dir) = model_cache_dir(app) else {
        tracing::warn!("no app data dir; embedding model unavailable");
        return;
    };
    let svc = EmbeddingService::new(dir);
    // On a restart the files are already cached and `kickoff` never fires, so
    // warm the session in the background now — otherwise the first encode
    // (inside a chat turn) pays the one-off model load inline. Read the
    // service back through `state()` inside the task: it is a &'static and
    // the service itself does not implement Clone.
    if svc.is_ready() {
        tauri::async_runtime::spawn_blocking(|| {
            if let Some(s) = crate::state().embedding.get() {
                s.warm_up();
            }
        });
    }
    let _ = crate::state().embedding.set(svc);
}

/// Serialises download attempts: one at a time. A tokio mutex (not an
/// `AtomicBool`) releases its guard on unwind, so a panicked download can
/// never wedge every future retry.
static DOWNLOAD_LOCK: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();

/// Fire-and-forget download trigger, called on every chat message.
///
/// Returns immediately when no service is registered, the model is already
/// cached, or a download is already running; otherwise spawns the download
/// on the async runtime, streaming progress to the frontend. Never blocks
/// or fails the caller.
pub async fn kickoff(app: &AppHandle) {
    let Some(svc) = crate::state().embedding.get() else {
        return;
    };
    if svc.is_ready() {
        return;
    }
    // `try_lock`, never `.await`: a second trigger while one runs is a no-op.
    let Ok(guard) = DOWNLOAD_LOCK.get_or_init(Default::default).try_lock() else {
        return;
    };

    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let _guard = guard;
        run_download(&app, svc).await;
    });
}

/// Download the model, forwarding every progress tuple as `model:progress`
/// and finishing with one `model:done`. Failures are logged and reported,
/// never raised — chat keeps working without local embeddings.
async fn run_download(app: &AppHandle, svc: &EmbeddingService) {
    let (tx, mut rx) = tokio::sync::mpsc::channel::<(String, u64, u64)>(64);
    let emitter = app.clone();
    let forward = async move {
        while let Some((file, downloaded, total)) = rx.recv().await {
            let _ = emitter.emit("model:progress", ModelProgress { file, downloaded, total });
        }
    };
    // Drain the channel alongside the download so `send` never stalls on a
    // full buffer while the stream keeps arriving.
    let (result, _) = tokio::join!(svc.ensure_model(tx), forward);
    match result {
        Ok(()) => {
            // Warm the inference session here in the background task, so the
            // first real encode (in a chat turn or a search) doesn't pay the
            // one-off ~1s model load inline.
            svc.warm_up();
            let _ = app.emit("model:done", ModelDone { ok: true, error: None });
        }
        Err(e) => {
            tracing::warn!("embedding model download failed: {e}");
            let _ = app.emit(
                "model:done",
                ModelDone { ok: false, error: Some(e.to_string()) },
            );
        }
    }
}

#[derive(Serialize, Clone)]
pub struct ModelProgress {
    pub file: String,
    pub downloaded: u64,
    pub total: u64,
}

#[derive(Serialize, Clone)]
pub struct ModelDone {
    pub ok: bool,
    pub error: Option<String>,
}
