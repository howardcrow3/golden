//! Which model suits this computer, whether it is on disk, and a download you can watch and
//! cancel. The state is plain atomics so any thread (a UI command, a progress poll) can read it
//! while the download task runs on whatever async runtime the app has.
use crate::catalog::{Catalog, ModelEntry, Role};
use crate::download;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;

#[derive(Default)]
pub struct DownloadState {
    downloaded: AtomicU64,
    total: AtomicU64,
    running: AtomicBool,
    cancel: AtomicBool,
    error: Mutex<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModelStatus {
    pub ram_gb: u64,
    pub id: String,
    pub name: String,
    pub size_mb: u64,
    pub installed: bool,
    /// Where the model file lives (or will live) on this computer.
    pub path: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgress {
    pub running: bool,
    pub downloaded_mb: u64,
    pub total_mb: u64,
    pub error: String,
}

fn mb(bytes: u64) -> u64 {
    bytes / 1_000_000
}

/// The general-purpose model for a machine with `ram_gb`, and whether its file is in `dir`.
pub fn status(catalog: &Catalog, ram_gb: u64, dir: &Path) -> Result<ModelStatus, String> {
    let entry = catalog
        .recommend(Role::General, ram_gb)
        .ok_or("This computer does not have enough memory for the built-in models.")?;
    let path = dir.join(&entry.file);
    Ok(ModelStatus {
        ram_gb,
        id: entry.id.clone(),
        name: entry.name.clone(),
        size_mb: mb(entry.size_bytes),
        installed: path.exists(),
        path: path.display().to_string(),
    })
}

/// Where a finished model file is, if it is installed.
pub fn installed_path(catalog: &Catalog, id: &str, dir: &Path) -> Option<PathBuf> {
    let p = dir.join(&catalog.get(id)?.file);
    p.exists().then_some(p)
}

impl DownloadState {
    pub fn progress(&self) -> DownloadProgress {
        DownloadProgress {
            running: self.running.load(Ordering::SeqCst),
            downloaded_mb: mb(self.downloaded.load(Ordering::Relaxed)),
            total_mb: mb(self.total.load(Ordering::Relaxed)),
            error: self.error.lock().map(|e| e.clone()).unwrap_or_default(),
        }
    }

    /// Ask a running download to stop. The partial file is kept so the next start resumes.
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::SeqCst);
    }

    /// Claim the download slot and reset the counters. Err if one is already running.
    pub fn begin(&self, entry: &ModelEntry) -> Result<(), String> {
        if self.running.swap(true, Ordering::SeqCst) {
            return Err("A download is already running.".into());
        }
        self.cancel.store(false, Ordering::SeqCst);
        self.downloaded.store(0, Ordering::SeqCst);
        self.total.store(entry.size_bytes, Ordering::SeqCst);
        if let Ok(mut e) = self.error.lock() {
            e.clear();
        }
        Ok(())
    }

    /// Do the download. Call `begin` first, then run this on any async runtime. Always releases
    /// the slot when it ends; a failure (not a cancel) is left in `progress().error`.
    pub async fn run(&self, entry: &ModelEntry, dir: &Path) {
        let result = download::download(entry, dir, &self.cancel, |p| {
            self.downloaded.store(p.downloaded, Ordering::Relaxed);
            self.total.store(p.total, Ordering::Relaxed);
        })
        .await;
        if let Err(e) = result {
            if !self.cancel.load(Ordering::SeqCst) {
                if let Ok(mut m) = self.error.lock() {
                    *m = e.to_string();
                }
            }
        }
        self.running.store(false, Ordering::SeqCst);
    }
}

/// Remove an installed model (and any half-finished download of it).
pub fn remove(catalog: &Catalog, id: &str, dir: &Path) -> Result<(), String> {
    let entry = catalog.get(id).ok_or("Unknown model.")?;
    for name in [entry.file.clone(), format!("{}.partial", entry.file)] {
        match std::fs::remove_file(dir.join(name)) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(format!("Couldn't remove the model: {e}")),
        }
    }
    Ok(())
}
