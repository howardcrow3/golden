//! Statement library: rename and file documents by what they contain, e.g.
//! `Statements/2026/Checking/2026-10 Chase Checking 4821 statement.pdf`.
use fin_core::NaiveDate;
use std::collections::HashSet;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatementMeta {
    pub institution: String,
    /// "Checking", "Savings", "Credit card", "Brokerage", "Retirement", "Mortgage", "Paystub", "Tax form"...
    pub kind: String,
    pub last4: Option<String>,
    /// Last day covered by the statement; names the month.
    pub period_end: NaiveDate,
    pub extension: String,
}

/// Characters that are illegal in Windows or macOS file names, or just trouble.
fn clean(s: &str) -> String {
    let t: String = s
        .chars()
        .map(|c| {
            if matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') || c.is_control() {
                ' '
            } else {
                c
            }
        })
        .collect();
    t.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim_matches('.')
        .to_string()
}

/// "2026-10 Chase Checking 4821 statement.pdf"
pub fn file_name(m: &StatementMeta) -> String {
    let mut parts = vec![
        m.period_end.format("%Y-%m").to_string(),
        clean(&m.institution),
        clean(&m.kind),
    ];
    if let Some(l) = &m.last4 {
        parts.push(clean(l));
    }
    parts.push("statement".into());
    let ext = m.extension.trim_start_matches('.').to_ascii_lowercase();
    format!(
        "{}.{}",
        parts
            .into_iter()
            .filter(|p| !p.is_empty())
            .collect::<Vec<_>>()
            .join(" "),
        if ext.is_empty() { "pdf".into() } else { ext }
    )
}

/// Folder inside the library: year, then document type.
pub fn folder(m: &StatementMeta) -> PathBuf {
    PathBuf::from(m.period_end.format("%Y").to_string()).join(clean(&m.kind))
}

/// A path that does not collide with `existing` (compared case-insensitively, as macOS and
/// Windows file systems do): appends " (2)", " (3)"...
pub fn unique_path(rel: PathBuf, existing: &HashSet<String>) -> PathBuf {
    let taken = |p: &PathBuf| {
        existing
            .iter()
            .any(|e| e.eq_ignore_ascii_case(&p.to_string_lossy().replace('\\', "/")))
    };
    let norm = |p: &PathBuf| PathBuf::from(p.to_string_lossy().replace('\\', "/"));
    if !taken(&norm(&rel)) {
        return rel;
    }
    let stem = rel.file_stem().unwrap().to_string_lossy().to_string();
    let ext = rel
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();
    for n in 2.. {
        let cand = rel.with_file_name(format!("{stem} ({n}){ext}"));
        if !taken(&norm(&cand)) {
            return cand;
        }
    }
    unreachable!()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn meta() -> StatementMeta {
        StatementMeta {
            institution: "Chase".into(),
            kind: "Checking".into(),
            last4: Some("4821".into()),
            period_end: NaiveDate::from_ymd_opt(2026, 10, 31).unwrap(),
            extension: "PDF".into(),
        }
    }

    #[test]
    fn names_by_contents() {
        assert_eq!(
            file_name(&meta()),
            "2026-10 Chase Checking 4821 statement.pdf"
        );
        assert_eq!(folder(&meta()), PathBuf::from("2026").join("Checking"));
    }

    #[test]
    fn missing_last4_and_extension() {
        let mut m = meta();
        m.last4 = None;
        m.extension = String::new();
        assert_eq!(file_name(&m), "2026-10 Chase Checking statement.pdf");
    }

    #[test]
    fn illegal_characters_are_removed() {
        let mut m = meta();
        m.institution = "AT&T / Bank: \"Main\"".into();
        assert_eq!(
            file_name(&m),
            "2026-10 AT&T Bank Main Checking 4821 statement.pdf"
        );
    }

    #[test]
    fn collisions_get_a_counter() {
        let rel = folder(&meta()).join(file_name(&meta()));
        let mut existing = HashSet::new();
        assert_eq!(unique_path(rel.clone(), &existing), rel);
        existing.insert("2026/Checking/2026-10 Chase Checking 4821 statement.pdf".to_string());
        let p2 = unique_path(rel.clone(), &existing);
        assert!(p2.to_string_lossy().ends_with("statement (2).pdf"));
        existing.insert(p2.to_string_lossy().replace('\\', "/"));
        assert!(unique_path(rel, &existing)
            .to_string_lossy()
            .ends_with("statement (3).pdf"));
    }

    #[test]
    fn collision_check_ignores_case() {
        let rel = folder(&meta()).join(file_name(&meta()));
        let mut existing = HashSet::new();
        existing.insert("2026/checking/2026-10 CHASE CHECKING 4821 STATEMENT.PDF".to_string());
        assert!(unique_path(rel, &existing)
            .to_string_lossy()
            .contains("(2)"));
    }
}
