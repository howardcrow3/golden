//! Files the original statement into the library folder, named and sorted by what it contains.
use fin_ingest::library::{file_name, folder, unique_path, StatementMeta};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

fn rel_string(p: &Path) -> String {
    p.to_string_lossy().replace('\\', "/")
}

/// Write `bytes` under `root` at the library path for `meta`. If a file with identical content
/// is already filed under that name (or a numbered variant) nothing new is written. Returns the
/// path relative to `root`, with forward slashes.
pub fn file_statement(root: &Path, bytes: &[u8], meta: &StatementMeta) -> std::io::Result<String> {
    let dir_rel = folder(meta);
    let dir = root.join(&dir_rel);
    std::fs::create_dir_all(&dir)?;
    let mut existing = HashSet::new();
    for e in std::fs::read_dir(&dir)? {
        let e = e?;
        existing.insert(rel_string(&dir_rel.join(e.file_name())));
    }
    let first = dir_rel.join(file_name(meta));
    // Identical content already filed under the first-choice name or a numbered variant?
    let stem = first
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    for name in &existing {
        let fname = name.rsplit('/').next().unwrap_or("");
        let same_family = fname.starts_with(&stem);
        if same_family
            && std::fs::read(root.join(name))
                .map(|b| b == bytes)
                .unwrap_or(false)
        {
            return Ok(name.clone());
        }
    }
    let rel = unique_path(first, &existing);
    std::fs::write(root.join(&rel), bytes)?;
    Ok(rel_string(&rel))
}

pub fn default_root(app_data: &Path) -> PathBuf {
    app_data.join("Statements")
}

/// Setting that holds the folder the person chose.
pub const FOLDER_KEY: &str = "library_folder";

impl crate::App {
    /// Choose where statements are filed. An empty path goes back to the default folder.
    /// The folder is created if needed and must be writable. Files already filed stay where
    /// they are; only new imports go to the new folder.
    pub fn set_library_folder(&mut self, path: &str) -> Result<String, String> {
        self.require_edit()?;
        let path = path.trim();
        if path.is_empty() {
            self.db
                .set_setting(FOLDER_KEY, "")
                .map_err(|e| e.to_string())?;
            self.library_root = self.library_default.clone();
            return Ok(self
                .library_root
                .as_ref()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default());
        }
        let p = Path::new(path);
        if !p.is_absolute() {
            return Err("Enter the full path of the folder, for example /Users/you/Documents/Statements or C:\\Users\\you\\Statements.".into());
        }
        std::fs::create_dir_all(p).map_err(|e| format!("Couldn't use that folder: {e}"))?;
        let probe = p.join(".golden-write-test");
        std::fs::write(&probe, b"ok")
            .map_err(|e| format!("Golden can't write to that folder: {e}"))?;
        let _ = std::fs::remove_file(&probe);
        let canon = p.to_string_lossy().to_string();
        self.db
            .set_setting(FOLDER_KEY, &canon)
            .map_err(|e| e.to_string())?;
        self.library_root = Some(p.to_path_buf());
        Ok(canon)
    }
}
