//! Saving reports (HTML to print as PDF), tables (CSV) and data (JSON) the UI has built.

use std::path::{Path, PathBuf};

const KINDS: [&str; 3] = ["html", "csv", "json"];

/// Write `content` into `dir` as `<stem>.<kind>`, numbering the name if it already exists.
/// The stem is reduced to letters, digits, spaces, `-` and `_`, so a name from the UI can't
/// point outside `dir`.
pub fn save_report(dir: &Path, stem: &str, kind: &str, content: &str) -> Result<PathBuf, String> {
    if !KINDS.contains(&kind) {
        return Err(format!("Reports can be saved as {}.", KINDS.join(", ")));
    }
    let clean: String = stem
        .chars()
        .filter(|c| c.is_alphanumeric() || matches!(c, ' ' | '-' | '_'))
        .collect();
    let clean = clean.trim();
    let clean = if clean.is_empty() {
        "golden-report"
    } else {
        clean
    };
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let mut path = dir.join(format!("{clean}.{kind}"));
    let mut n = 2;
    while path.exists() {
        path = dir.join(format!("{clean} ({n}).{kind}"));
        n += 1;
    }
    std::fs::write(&path, content).map_err(|e| e.to_string())?;
    Ok(path)
}
