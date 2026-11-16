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
