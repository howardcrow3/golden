//! Statement folder pickup (IN-1, IN-5, IN-7): import every new statement in a folder such as
//! Downloads in one step. A file whose exact bytes were imported before is skipped, so running it
//! again, or importing a file by hand first, never double-counts or re-files anything.
use crate::{App, ImportReport};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::path::Path;

const SEEN_KEY: &str = "imported_file_hashes";
const MAX_SEEN: usize = 5000;
const MAX_FILES: usize = 200;
const MAX_BYTES: u64 = 20_000_000;

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FolderFailure {
    pub name: String,
    pub error: String,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FolderReport {
    /// Statement files found (csv, ofx, qfx).
    pub found: usize,
    /// Files skipped because the same file was imported before.
    pub already_imported: usize,
    pub imported: Vec<ImportReport>,
    pub failed: Vec<FolderFailure>,
}

pub fn file_hash(bytes: &[u8]) -> String {
    let d = Sha256::digest(bytes);
    d.iter().map(|b| format!("{b:02x}")).collect()
}

fn tag_value(text: &str, name: &str) -> Option<String> {
    let upper = text.to_ascii_uppercase();
    let open = format!("<{name}>");
    let start = upper.find(&open)? + open.len();
    let rest = &text[start..];
    let end = rest.find(['<', '\n', '\r']).unwrap_or(rest.len());
    let v = rest[..end].trim();
    (!v.is_empty()).then(|| v.to_string())
}

/// Where an OFX/QFX statement belongs, from what it says about itself: the bank's name, the
/// account type and the last four digits. None for CSV, which says nothing reliable.
pub fn ofx_account(bytes: &[u8]) -> Option<(String, &'static str)> {
    let text = String::from_utf8_lossy(bytes);
    let upper = text.to_ascii_uppercase();
    if !upper.contains("<OFX>") {
        return None;
    }
    let org = tag_value(&text, "ORG").unwrap_or_else(|| "Bank".into());
    let kind = if upper.contains("<CCSTMTRS>") || upper.contains("<CREDITCARDMSGSRSV1>") {
        "credit"
    } else {
        match tag_value(&text, "ACCTTYPE")
            .map(|t| t.to_ascii_uppercase())
            .as_deref()
        {
            Some("SAVINGS") => "savings",
            _ => "checking",
        }
    };
    let last4 = tag_value(&text, "ACCTID").map(|id| {
        let digits: Vec<char> = id.chars().filter(char::is_ascii_digit).collect();
        digits[digits.len().saturating_sub(4)..]
            .iter()
            .collect::<String>()
    });
    let name = match last4.filter(|l| !l.is_empty()) {
        Some(l) => format!("{org} {l}"),
        None => org,
    };
    Some((name, kind))
}

impl App {
    fn seen_hashes(&self) -> Vec<String> {
        self.db
            .get_setting(SEEN_KEY)
            .ok()
            .flatten()
            .map(|s| s.lines().map(String::from).collect())
            .unwrap_or_default()
    }

    pub(crate) fn remember_file(&self, bytes: &[u8]) -> Result<(), String> {
        let h = file_hash(bytes);
        let mut seen = self.seen_hashes();
        if seen.contains(&h) {
            return Ok(());
        }
        seen.push(h);
        if seen.len() > MAX_SEEN {
            seen.drain(..seen.len() - MAX_SEEN);
        }
        self.db
            .set_setting(SEEN_KEY, &seen.join("\n"))
            .map_err(|e| e.to_string())
    }

    /// True when these exact bytes were imported before.
    pub fn file_already_imported(&self, bytes: &[u8]) -> bool {
        self.seen_hashes().contains(&file_hash(bytes))
    }

    /// Import every new statement in `dir` (not its sub-folders). OFX/QFX files go to the account
    /// they name; CSV files go to `default_account` (they say nothing reliable about themselves).
    pub fn import_folder(
        &mut self,
        dir: &Path,
        default_account: &str,
        default_kind: &str,
    ) -> Result<FolderReport, String> {
        self.require_edit()?;
        let mut files: Vec<_> = std::fs::read_dir(dir)
            .map_err(|e| format!("Could not open that folder: {e}"))?
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
            .filter(|e| {
                e.path()
                    .extension()
                    .map(|x| {
                        let x = x.to_string_lossy().to_ascii_lowercase();
                        matches!(x.as_str(), "csv" | "ofx" | "qfx" | "pdf")
                    })
                    .unwrap_or(false)
            })
            .collect();
        files.sort_by_key(|e| e.file_name());
        files.truncate(MAX_FILES);
        let mut report = FolderReport {
            found: files.len(),
            already_imported: 0,
            imported: vec![],
            failed: vec![],
        };
        for f in files {
            let name = f.file_name().to_string_lossy().to_string();
            let fail = |report: &mut FolderReport, error: String| {
                report.failed.push(FolderFailure {
                    name: name.clone(),
                    error,
                })
            };
            if f.metadata().map(|m| m.len() > MAX_BYTES).unwrap_or(true) {
                fail(
                    &mut report,
                    "This file is too large to be a statement.".into(),
                );
                continue;
            }
            let bytes = match std::fs::read(f.path()) {
                Ok(b) => b,
                Err(e) => {
                    fail(&mut report, e.to_string());
                    continue;
                }
            };
            if self.file_already_imported(&bytes) {
                report.already_imported += 1;
                continue;
            }
            let (account, kind) = match ofx_account(&bytes) {
                Some((a, k)) => (a, k.to_string()),
                None if default_account.trim().is_empty() => {
                    fail(
                        &mut report,
                        "Choose an account for CSV files; they don't say which account they are from.".into(),
                    );
                    continue;
                }
                None => (default_account.trim().to_string(), default_kind.to_string()),
            };
            match self.import_file(&name, &bytes, &account, &kind) {
                Ok(r) => report.imported.push(r),
                Err(e) => fail(&mut report, e),
            }
        }
        Ok(report)
    }
}
