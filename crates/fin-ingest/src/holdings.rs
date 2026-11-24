//! Read a broker's positions export (CSV) into plain holdings. No database here; the app turns
//! these into saved holdings after the person has looked them over.
use crate::IngestError;
use fin_core::Money;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetKind {
    Stock,
    Bond,
    Cash,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedHolding {
    pub name: String,
    pub value: Money,
    pub kind: AssetKind,
    /// Hundredths of a percent a year (7 is 0.07%); 0 when the file has no fee column.
    pub expense_bp: i64,
    /// An individual company rather than a fund.
    pub single_stock: bool,
}

const NAME: &[&str] = &[
    "name",
    "description",
    "security",
    "security name",
    "security description",
    "investment",
    "investment name",
    "fund name",
    "holding",
    "symbol",
    "ticker",
];
const VALUE: &[&str] = &[
    "market value",
    "current value",
    "value",
    "total value",
    "ending balance",
    "balance",
    "position value",
];
const KIND: &[&str] = &[
    "asset class",
    "asset type",
    "security type",
    "investment type",
    "type",
    "category",
];
const FEE: &[&str] = &[
    "expense ratio",
    "expense ratio (%)",
    "net expense ratio",
    "expenses",
];

fn find(h: &[String], names: &[&str]) -> Option<usize> {
    // Earlier names win, so "security name" beats a bare "symbol" when both exist.
    names.iter().find_map(|n| h.iter().position(|c| c == n))
}

/// Sort a position into stock, bond or cash from its type text, falling back to its name.
pub fn classify(kind_text: &str, name: &str) -> AssetKind {
    let k = kind_text.to_ascii_lowercase();
    let n = name.to_ascii_lowercase();
    let any = |s: &str, words: &[&str]| words.iter().any(|w| s.contains(w));
    if any(&k, &["bond", "fixed income", "treasury", "debt"]) {
        AssetKind::Bond
    } else if any(&k, &["cash", "money market", "sweep", "mmf"]) {
        AssetKind::Cash
    } else if any(&k, &["stock", "equity", "equities"]) {
        AssetKind::Stock
    } else if any(&n, &["bond", "treasury", "fixed income"]) {
        AssetKind::Bond
    } else if any(&n, &["money market", "cash"]) {
        AssetKind::Cash
    } else if any(&n, &["stock", "equity", "s&p 500", "total market", "index"]) {
        AssetKind::Stock
    } else {
        AssetKind::Other
    }
}

fn is_single_stock(kind_text: &str) -> bool {
    let k = kind_text.to_ascii_lowercase();
    (k.contains("common stock") || k == "stock" || k == "stocks")
        && !k.contains("fund")
        && !k.contains("etf")
}

fn fee_bp(raw: &str) -> i64 {
    let t = raw.trim().trim_end_matches('%').trim();
    t.parse::<f64>()
        .ok()
        .filter(|p| p.is_finite() && (0.0..=10.0).contains(p))
        .map_or(0, |p| (p * 100.0).round() as i64)
}

/// Parse a positions CSV. Needs a name (or symbol) column and a value column; skips total lines,
/// blank lines and anything with no positive value.
pub fn parse(text: &str) -> Result<Vec<ParsedHolding>, IngestError> {
    if text.trim().is_empty() {
        return Err(IngestError::Empty);
    }
    // Skip a preamble: the header is the first line naming both a value and a name column.
    let mut start = 0usize;
    for line in text.split_inclusive('\n') {
        let cells: Vec<String> = line
            .split([',', ';', '\t'])
            .map(|c| c.trim().trim_matches('"').to_ascii_lowercase())
            .collect();
        if find(&cells, VALUE).is_some() && find(&cells, NAME).is_some() {
            break;
        }
        start += line.len();
    }
    if start >= text.len() {
        return Err(IngestError::Csv(
            "could not find a name column and a value column".into(),
        ));
    }
    let mut rdr = csv::ReaderBuilder::new()
        .flexible(true)
        .trim(csv::Trim::All)
        .from_reader(&text.as_bytes()[start..]);
    let headers: Vec<String> = rdr
        .headers()
        .map_err(|e| IngestError::Csv(e.to_string()))?
        .iter()
        .map(|s| s.to_ascii_lowercase())
        .collect();
    let (name_c, value_c) = (
        find(&headers, NAME).unwrap(),
        find(&headers, VALUE).unwrap(),
    );
    let (kind_c, fee_c) = (find(&headers, KIND), find(&headers, FEE));
    let mut out = vec![];
    for (i, rec) in rdr.records().enumerate() {
        let rec = rec.map_err(|e| IngestError::Csv(e.to_string()))?;
        let name = rec.get(name_c).unwrap_or("").trim().to_string();
        let lower = name.to_ascii_lowercase();
        // A total line, but not a fund that merely starts with the word ("Total Stock Market").
        let total_line = matches!(
            lower.as_str(),
            "total" | "totals" | "grand total" | "total value" | "total holdings"
        ) || lower.ends_with(" total");
        if name.is_empty() || total_line {
            continue;
        }
        let raw = rec.get(value_c).unwrap_or("").trim();
        if raw.is_empty() {
            continue;
        }
        let value = Money::parse(raw).map_err(|e| IngestError::Row {
            row: i + 2,
            msg: e.to_string(),
        })?;
        if value <= Money::ZERO {
            continue;
        }
        let kind_text = kind_c.and_then(|c| rec.get(c)).unwrap_or("");
        out.push(ParsedHolding {
            kind: classify(kind_text, &name),
            single_stock: is_single_stock(kind_text),
            expense_bp: fee_c.and_then(|c| rec.get(c)).map_or(0, fee_bp),
            name,
            value,
        });
    }
    Ok(out)
}
