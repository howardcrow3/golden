//! A watched folder: Golden checks it for new statements now and then and imports them, so
//! downloading a statement is all it takes. The same rules as a manual folder import apply
//! (files already imported are skipped; trial and closed months are respected).
use crate::folder::FolderReport;
use crate::App;
use serde::{Deserialize, Serialize};
use std::path::Path;

const KEY: &str = "watched_folder";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Watch {
    pub path: String,
    /// Account for CSV files (OFX/QFX name their own).
    pub account: String,
    pub kind: String,
}

impl App {
    pub fn watched_folder(&self) -> Result<Option<Watch>, String> {
        match self.db.get_setting(KEY).map_err(|e| e.to_string())? {
            Some(j) if !j.is_empty() => Ok(serde_json::from_str(&j).ok()),
            _ => Ok(None),
        }
    }

    /// Start watching `path`. The folder must exist now.
    pub fn watch_folder(&self, path: &str, account: &str, kind: &str) -> Result<Watch, String> {
        self.require_edit()?;
        let path = path.trim();
        if path.is_empty() || !Path::new(path).is_dir() {
            return Err("That folder doesn't exist.".into());
        }
        let w = Watch {
            path: path.to_string(),
            account: account.trim().to_string(),
            kind: kind.to_string(),
        };
        self.db
            .set_setting(KEY, &serde_json::to_string(&w).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        Ok(w)
    }

    pub fn stop_watching(&self) -> Result<(), String> {
        self.db.set_setting(KEY, "").map_err(|e| e.to_string())
    }

    /// Import anything new in the watched folder. `None` when nothing is watched, the folder
    /// has gone, or editing is locked, so a background timer can call this without fuss.
    pub fn scan_watched(&mut self) -> Option<FolderReport> {
        let w = self.watched_folder().ok().flatten()?;
        self.import_folder(Path::new(&w.path), &w.account, &w.kind)
            .ok()
    }
}
