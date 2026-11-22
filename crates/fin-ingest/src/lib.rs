//! Statement readers: turn bank files (CSV, OFX/QFX, text PDFs) into plain parsed transactions.
//! No database or AI here; reconciliation and categorization happen downstream.
pub mod csv_reader;
pub mod dates;
pub mod docclass;
pub mod ofx;
pub mod paystub;
pub mod pdf;

use fin_core::{Money, NaiveDate};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedTxn {
    pub day: NaiveDate,
    pub amount: Money,
    pub payee: String,
    pub memo: String,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum IngestError {
    #[error("could not find a date column")]
    NoDateColumn,
    #[error("could not find an amount column")]
    NoAmountColumn,
    #[error("row {row}: {msg}")]
    Row { row: usize, msg: String },
    #[error("file is empty")]
    Empty,
    #[error("not a recognized OFX file")]
    NotOfx,
    #[error("{0}")]
    Csv(String),
}

/// The sum of a statement's lines, used to reconcile against its printed totals.
pub fn total(txns: &[ParsedTxn]) -> Money {
    txns.iter().map(|t| t.amount).sum()
}

/// Detect the file type from its bytes and parse it.
pub fn read_statement(bytes: &[u8]) -> Result<Vec<ParsedTxn>, IngestError> {
    if bytes.starts_with(b"%PDF") {
        return pdf::parse_text(&pdf::extract_text(bytes)?);
    }
    let text = String::from_utf8_lossy(bytes);
    let head = text.trim_start();
    if head.starts_with("OFXHEADER") || head.contains("<OFX>") || head.contains("<ofx>") {
        ofx::parse(&text)
    } else {
        csv_reader::parse(&text)
    }
}
pub mod library;

/// The closing balance a statement prints, when the format carries one (OFX/QFX do; CSV does not).
pub fn statement_balance(bytes: &[u8]) -> Option<(NaiveDate, Money)> {
    if bytes.starts_with(b"%PDF") {
        return pdf::closing_balance(&pdf::extract_text(bytes).ok()?);
    }
    let text = String::from_utf8_lossy(bytes);
    let head = text.trim_start();
    if head.starts_with("OFXHEADER") || head.contains("<OFX>") || head.contains("<ofx>") {
        ofx::ledger_balance(&text)
    } else {
        None
    }
}
