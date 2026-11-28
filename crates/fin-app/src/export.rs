//! Your data is yours: exports work in every licence state, including after the trial ends.
use crate::App;
use std::path::{Path, PathBuf};

/// RFC 4180 quoting, plus a guard so a payee like "=cmd()" cannot run as a spreadsheet formula.
fn cell(text: &str) -> String {
    let risky = text.starts_with(['=', '+', '@', '-', '\t', '\r']) && text.parse::<f64>().is_err();
    let guarded = if risky {
        format!("'{text}")
    } else {
        text.to_string()
    };
    if guarded.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", guarded.replace('"', "\"\""))
    } else {
        guarded
    }
}

fn dollars(cents: i64) -> String {
    let sign = if cents < 0 { "-" } else { "" };
    let a = cents.abs();
    format!("{sign}{}.{:02}", a / 100, a % 100)
}

impl App {
    /// Every transaction between two dates (oldest first) as CSV text.
    pub fn transactions_csv(&self, from: &str, to: &str) -> Result<String, String> {
        // `transactions` caps what the screen shows; an export must be complete.
        let from_d = crate::parse_day(from)?;
        let to_d = crate::parse_day(to)?;
        let names: std::collections::HashMap<i64, String> = self
            .db
            .accounts()
            .map_err(|e| e.to_string())?
            .into_iter()
            .map(|(id, n, _, _)| (id.0, n))
            .collect();
        let mut rows = self
            .db
            .txns_between(from_d, to_d)
            .map_err(|e| e.to_string())?;
        rows.sort_by(|a, b| a.day.cmp(&b.day).then(a.id.cmp(&b.id)));
        let mut out = String::from("Date,Account,Payee,Memo,Amount,Category\r\n");
        for t in rows {
            out.push_str(&format!(
                "{},{},{},{},{},{}\r\n",
                t.day.format("%Y-%m-%d"),
                cell(names.get(&t.account.0).map(String::as_str).unwrap_or("")),
                cell(&t.payee),
                cell(&t.memo),
                dollars(t.amount.cents()),
                cell(t.category.as_deref().unwrap_or("")),
            ));
        }
        Ok(out)
    }

    /// Every transaction between two dates (oldest first) as an Excel workbook, with amounts as
    /// numbers so they add up in the spreadsheet.
    pub fn transactions_xlsx(&self, from: &str, to: &str) -> Result<Vec<u8>, String> {
        use crate::xlsx::{workbook, Cell, Sheet};
        let from_d = crate::parse_day(from)?;
        let to_d = crate::parse_day(to)?;
        let names: std::collections::HashMap<i64, String> = self
            .db
            .accounts()
            .map_err(|e| e.to_string())?
            .into_iter()
            .map(|(id, n, _, _)| (id.0, n))
            .collect();
        let mut rows = self
            .db
            .txns_between(from_d, to_d)
            .map_err(|e| e.to_string())?;
        rows.sort_by(|a, b| a.day.cmp(&b.day).then(a.id.cmp(&b.id)));
        let text = |s: &str| Cell::Text(s.to_string());
        let mut out = vec![["Date", "Account", "Payee", "Memo", "Amount", "Category"]
            .iter()
            .map(|h| text(h))
            .collect::<Vec<_>>()];
        for t in rows {
            out.push(vec![
                Cell::Text(t.day.format("%Y-%m-%d").to_string()),
                text(names.get(&t.account.0).map(String::as_str).unwrap_or("")),
                text(&t.payee),
                text(&t.memo),
                Cell::Number(t.amount.cents() as f64 / 100.0),
                text(t.category.as_deref().unwrap_or("")),
            ]);
        }
        Ok(workbook(&[Sheet {
            name: "Transactions".into(),
            rows: out,
        }]))
    }

    /// Write the CSV into `dir` and return the file's path.
    pub fn export_transactions(&self, dir: &Path, from: &str, to: &str) -> Result<PathBuf, String> {
        let csv = self.transactions_csv(from, to)?;
        write_new(
            dir,
            &format!("golden-transactions-{from}-to-{to}"),
            "csv",
            csv.as_bytes(),
        )
    }

    /// Write the Excel workbook into `dir` and return the file's path.
    pub fn export_transactions_xlsx(
        &self,
        dir: &Path,
        from: &str,
        to: &str,
    ) -> Result<PathBuf, String> {
        let book = self.transactions_xlsx(from, to)?;
        write_new(
            dir,
            &format!("golden-transactions-{from}-to-{to}"),
            "xlsx",
            &book,
        )
    }
}

/// Write `data` to `dir/base.ext`, adding " (2)", " (3)" and so on rather than overwrite.
fn write_new(dir: &Path, base: &str, ext: &str, data: &[u8]) -> Result<PathBuf, String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let mut path = dir.join(format!("{base}.{ext}"));
    let mut n = 2;
    while path.exists() {
        path = dir.join(format!("{base} ({n}).{ext}"));
        n += 1;
    }
    std::fs::write(&path, data).map_err(|e| e.to_string())?;
    Ok(path)
}
