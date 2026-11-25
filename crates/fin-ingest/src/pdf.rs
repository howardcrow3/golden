//! Text-based PDF statements (IN-1). The PDF's text layer is read, then each line that starts
//! with a date and ends with an amount becomes a transaction. A PDF doesn't say which column an
//! amount was in, so the sign comes from, in order: the running balance column (the change from
//! the line before), an explicit minus/parentheses/CR/DR marker, and finally the kind of
//! statement. When none of that settles it the file is refused rather than guessed at; the
//! balance check afterwards catches anything that still slipped through.
//!
//! Scanned (image-only) PDFs have no text layer and need OCR, which isn't built in yet.
use crate::dates::parse_date;
use crate::{IngestError, ParsedTxn};
use chrono::{Datelike, NaiveDate};
use fin_core::Money;

/// Read the text layer of a PDF.
pub fn extract_text(bytes: &[u8]) -> Result<String, IngestError> {
    let text = pdf_extract::extract_text_from_mem(bytes)
        .map_err(|e| IngestError::Csv(format!("Could not read this PDF: {e}")))?;
    if text.trim().chars().filter(|c| c.is_alphanumeric()).count() < 20 {
        return Err(IngestError::Csv(
            "This PDF has no readable text; it looks scanned. Reading scanned pages (OCR) isn't built in yet, so download a CSV or OFX file from your bank instead.".into(),
        ));
    }
    Ok(text)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Marker {
    None,
    Negative,
    Positive,
}

#[derive(Debug, Clone, Copy)]
struct Amount {
    abs_cents: i64,
    marker: Marker,
}

fn parse_money(tok: &str) -> Option<Amount> {
    let mut t = tok.trim();
    let mut marker = Marker::None;
    if let Some(r) = t.strip_suffix("CR").or_else(|| t.strip_suffix("cr")) {
        marker = Marker::Positive;
        t = r;
    } else if let Some(r) = t.strip_suffix("DR").or_else(|| t.strip_suffix("dr")) {
        marker = Marker::Negative;
        t = r;
    }
    if t.starts_with('(') && t.ends_with(')') && t.len() > 2 {
        marker = Marker::Negative;
        t = &t[1..t.len() - 1];
    }
    if let Some(r) = t.strip_suffix('-') {
        marker = Marker::Negative;
        t = r;
    }
    if let Some(r) = t.strip_prefix('-') {
        marker = Marker::Negative;
        t = r;
    } else if let Some(r) = t.strip_prefix('+') {
        marker = Marker::Positive;
        t = r;
    }
    let t = t.strip_prefix('$').unwrap_or(t);
    let (whole, frac) = t.rsplit_once('.')?;
    if frac.len() != 2 || !frac.bytes().all(|b| b.is_ascii_digit()) || whole.is_empty() {
        return None;
    }
    let groups: Vec<&str> = whole.split(',').collect();
    if groups.iter().enumerate().any(|(i, g)| {
        g.is_empty()
            || !g.bytes().all(|b| b.is_ascii_digit())
            || (i > 0 && g.len() != 3)
            || (i == 0 && g.len() > 3 && groups.len() > 1)
    }) {
        return None;
    }
    let digits: String = whole.chars().filter(|c| *c != ',').collect();
    let dollars: i64 = digits.parse().ok()?;
    let cents: i64 = frac.parse().ok()?;
    Some(Amount {
        abs_cents: dollars.checked_mul(100)?.checked_add(cents)?,
        marker,
    })
}

const MONTHS: [&str; 12] = [
    "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
];

/// A leading date and how many whitespace-separated tokens it used.
fn leading_date(tokens: &[&str], year: i32) -> Option<(NaiveDate, usize)> {
    let first = *tokens.first()?;
    // Month names: "Oct 17" or "Oct 17, 2026".
    let lower = first.to_ascii_lowercase();
    if let Some(m) = MONTHS.iter().position(|m| lower.starts_with(m)) {
        if lower.len() <= 9 && lower.chars().all(|c| c.is_ascii_alphabetic() || c == '.') {
            let day: u32 = tokens.get(1)?.trim_end_matches(',').parse().ok()?;
            if let Some(y) = tokens.get(2).and_then(|t| t.parse::<i32>().ok()) {
                if (1990..=2100).contains(&y) {
                    return NaiveDate::from_ymd_opt(y, m as u32 + 1, day).map(|d| (d, 3));
                }
            }
            return NaiveDate::from_ymd_opt(year, m as u32 + 1, day).map(|d| (d, 2));
        }
        return None;
    }
    if let Some(d) = parse_date(first) {
        return Some((d, 1));
    }
    // "10/17" with no year.
    let (m, d) = first.split_once('/')?;
    if d.contains('/') {
        return None;
    }
    NaiveDate::from_ymd_opt(year, m.parse().ok()?, d.parse().ok()?).map(|dt| (dt, 1))
}

/// The year the statement is for: the most common four-digit year in the text.
fn statement_year(text: &str) -> i32 {
    let mut counts = std::collections::BTreeMap::new();
    for w in text.split(|c: char| !c.is_ascii_digit()) {
        if w.len() == 4 {
            if let Ok(y) = w.parse::<i32>() {
                if (1990..=2100).contains(&y) {
                    *counts.entry(y).or_insert(0) += 1;
                }
            }
        }
    }
    counts
        .into_iter()
        .max_by_key(|(y, n)| (*n, *y))
        .map(|(y, _)| y)
        .unwrap_or_else(|| chrono::Local::now().year())
}

fn is_card(text: &str) -> bool {
    let l = text.to_ascii_lowercase();
    [
        "minimum payment",
        "new balance",
        "credit limit",
        "payment due",
    ]
    .iter()
    .any(|k| l.contains(k))
}

struct Row {
    day: NaiveDate,
    payee: String,
    amount: Amount,
    balance: Option<Amount>,
}

fn rows(text: &str, year: i32) -> Vec<Row> {
    let mut out = vec![];
    for line in text.lines() {
        let tokens: Vec<&str> = line.split_whitespace().collect();
        let Some((day, used)) = leading_date(&tokens, year) else {
            continue;
        };
        let rest = &tokens[used..];
        // Trailing amounts: the transaction, then optionally a running balance.
        let mut n = rest.len();
        let mut money = vec![];
        while n > 0 && money.len() < 2 {
            match parse_money(rest[n - 1]) {
                Some(a) => {
                    money.push(a);
                    n -= 1;
                }
                None => break,
            }
        }
        if money.is_empty() {
            continue;
        }
        money.reverse();
        let mut desc = &rest[..n];
        // A second (posting) date right after the first isn't part of the description.
        if let Some((_, k)) = leading_date(desc, year) {
            desc = &desc[k..];
        }
        let payee = desc.join(" ");
        if payee.is_empty() {
            continue;
        }
        let low = payee.to_ascii_lowercase();
        if low.contains("balance") && !low.contains("transfer") {
            continue; // "Beginning balance" and friends are not transactions.
        }
        out.push(Row {
            day,
            payee,
            amount: money[0],
            balance: money.get(1).copied(),
        });
    }
    out
}

fn opening_balance(text: &str) -> Option<i64> {
    for line in text.lines() {
        let l = line.to_ascii_lowercase();
        if l.contains("beginning balance")
            || l.contains("previous balance")
            || l.contains("opening balance")
        {
            if let Some(a) = line.split_whitespace().rev().find_map(parse_money) {
                return Some(match a.marker {
                    Marker::Negative => -a.abs_cents,
                    _ => a.abs_cents,
                });
            }
        }
    }
    None
}

/// Transactions from the text of a statement PDF.
pub fn parse_text(text: &str) -> Result<Vec<ParsedTxn>, IngestError> {
    let year = statement_year(text);
    let found = rows(text, year);
    if found.is_empty() {
        return Err(IngestError::Csv(
            "Found no dated transaction lines in this PDF. Download a CSV or OFX file from your bank instead.".into(),
        ));
    }
    let card = is_card(text);
    let mut prev_balance: Option<i64> = opening_balance(text);
    let mut out = Vec::with_capacity(found.len());
    for (i, r) in found.iter().enumerate() {
        let abs = r.amount.abs_cents;
        let signed = if let Some(bal) = r.balance {
            // A running balance settles the sign: it went up (credit) or down (debit).
            let b = match bal.marker {
                Marker::Negative => -bal.abs_cents,
                _ => bal.abs_cents,
            };
            let sign = match prev_balance {
                Some(p) if b - p == abs => Some(1),
                Some(p) if b - p == -abs => Some(-1),
                Some(_) => {
                    return Err(IngestError::Row {
                        row: i + 1,
                        msg: format!(
                            "the balance after \"{}\" doesn't match the amount, so the lines can't be read reliably",
                            r.payee
                        ),
                    })
                }
                None => match r.amount.marker {
                    Marker::Negative => Some(-1),
                    Marker::Positive => Some(1),
                    Marker::None => None,
                },
            };
            prev_balance = Some(b);
            match sign {
                Some(s) => s * abs,
                // No opening balance and no marker: the balances alone can't say which
                // way the first line went.
                None => return Err(unclear(i, &r.payee)),
            }
        } else {
            match (r.amount.marker, card) {
                // A card statement prints purchases plain and credits (payments, refunds)
                // with a minus, parentheses or CR.
                (Marker::None, true) => -abs,
                (_, true) => abs,
                // A bank statement marks money out; plain amounts are deposits.
                (Marker::Negative, false) => -abs,
                (_, false) => abs,
            }
        };
        out.push(ParsedTxn {
            day: r.day,
            amount: Money::from_cents(signed),
            payee: r.payee.clone(),
            memo: String::new(),
        });
    }
    Ok(out)
}

fn unclear(i: usize, payee: &str) -> IngestError {
    IngestError::Row {
        row: i + 1,
        msg: format!("can't tell whether \"{payee}\" is money in or out. Download a CSV or OFX file instead."),
    }
}

/// The closing balance the statement prints: the last running balance, or a labelled line.
pub fn closing_balance(text: &str) -> Option<(NaiveDate, Money)> {
    let year = statement_year(text);
    let found = rows(text, year);
    let last_day = found.iter().map(|r| r.day).max()?;
    for line in text.lines().rev() {
        let l = line.to_ascii_lowercase();
        if l.contains("ending balance") || l.contains("closing balance") {
            if let Some(a) = line.split_whitespace().rev().find_map(parse_money) {
                let c = if a.marker == Marker::Negative {
                    -a.abs_cents
                } else {
                    a.abs_cents
                };
                return Some((last_day, Money::from_cents(c)));
            }
        }
    }
    let b = found.last()?.balance?;
    let c = if b.marker == Marker::Negative {
        -b.abs_cents
    } else {
        b.abs_cents
    };
    Some((last_day, Money::from_cents(c)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cents(v: &[ParsedTxn]) -> Vec<i64> {
        v.iter().map(|t| t.amount.cents()).collect()
    }

    #[test]
    fn money_tokens() {
        let m = |s| parse_money(s).map(|a| (a.abs_cents, a.marker));
        assert_eq!(m("1,234.56"), Some((123_456, Marker::None)));
        assert_eq!(m("$12.50"), Some((1250, Marker::None)));
        assert_eq!(m("-$12.50"), Some((1250, Marker::Negative)));
        assert_eq!(m("(45.00)"), Some((4500, Marker::Negative)));
        assert_eq!(m("45.00-"), Some((4500, Marker::Negative)));
        assert_eq!(m("45.00CR"), Some((4500, Marker::Positive)));
        assert_eq!(m("45.00DR"), Some((4500, Marker::Negative)));
        assert_eq!(m("12,34.56"), None);
        assert_eq!(m("1234"), None);
        assert_eq!(m("12.5"), None);
        assert_eq!(m("abc"), None);
    }

    #[test]
    fn running_balance_gives_the_sign() {
        let text = "Checking statement October 2026\nBeginning balance $1,000.00\n\
            10/01 PAYROLL ACME 5,000.00 6,000.00\n\
            10/06 CAFE LUNCH 18.40 5,981.60\n\
            10/09 RENT 1,500.00 4,481.60\nEnding balance $4,481.60\n";
        let t = parse_text(text).unwrap();
        assert_eq!(cents(&t), vec![500_000, -1_840, -150_000]);
        assert_eq!(t[0].day, NaiveDate::from_ymd_opt(2026, 10, 1).unwrap());
        assert_eq!(t[1].payee, "CAFE LUNCH");
        let (_, bal) = closing_balance(text).unwrap();
        assert_eq!(bal.cents(), 448_160);
    }

    #[test]
    fn a_first_line_with_no_way_to_know_its_sign_is_refused() {
        let text = "Statement 2026\n10/01 PAYROLL 5,000.00 6,000.00\n10/06 CAFE 18.40 5,981.60\n";
        assert!(parse_text(text).is_err());
        // A minus sign on that first line settles it, and the rest follow the balances.
        let text = "Statement 2026\n10/01 RENT -5,000.00 6,000.00\n10/06 CAFE 18.40 5,981.60\n";
        assert_eq!(cents(&parse_text(text).unwrap()), vec![-500_000, -1_840]);
    }

    #[test]
    fn a_balance_that_does_not_add_up_is_refused() {
        let text = "Statement 2026\nBeginning balance 100.00\n10/01 THING 10.00 500.00\n";
        let e = parse_text(text).unwrap_err();
        assert!(e.to_string().contains("doesn't match"), "{e}");
    }

    #[test]
    fn bank_statement_markers_without_balances() {
        let text = "Savings 2026\nOct 3 INTEREST 1.20\nOct 9 ATM WITHDRAWAL -40.00\nOct 12, 2026 FEE (5.00)\n";
        let t = parse_text(text).unwrap();
        assert_eq!(cents(&t), vec![120, -4_000, -500]);
        assert_eq!(t[2].day, NaiveDate::from_ymd_opt(2026, 10, 12).unwrap());
    }

    #[test]
    fn credit_card_purchases_are_negative_and_payments_positive() {
        let text = "Card statement 2026\nMinimum payment due $25.00\n\
            10/02 10/03 COFFEE SHOP 4.50\n10/10 10/11 PAYMENT THANK YOU 200.00CR\n";
        let t = parse_text(text).unwrap();
        assert_eq!(cents(&t), vec![-450, 20_000]);
        assert_eq!(t[0].payee, "COFFEE SHOP");
    }

    #[test]
    fn unreadable_or_empty_text_is_refused() {
        assert!(parse_text("nothing useful here\njust words").is_err());
        assert!(extract_text(b"not a pdf at all").is_err());
    }
}

/// The JPEG pictures embedded in a PDF, in page order. A scanned statement is normally one JPEG
/// per page, so these are what an OCR engine needs. Pictures stored in other ways (raw pixels,
/// fax encodings) are skipped.
pub fn embedded_jpegs(bytes: &[u8]) -> Vec<Vec<u8>> {
    let Ok(doc) = lopdf::Document::load_mem(bytes) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (_, page) in doc.get_pages() {
        let Ok(images) = doc.get_page_images(page) else {
            continue;
        };
        for img in images {
            let jpeg = img
                .filters
                .as_ref()
                .is_some_and(|f| f.len() == 1 && f[0] == "DCTDecode")
                && img.content.starts_with(&[0xFF, 0xD8]);
            if jpeg {
                out.push(img.content.to_vec());
            }
        }
    }
    out
}

#[cfg(test)]
mod embedded_tests {
    use super::*;
    use lopdf::{dictionary, Document, Object, Stream};

    fn pdf_with(images: &[(&str, Vec<u8>)]) -> Vec<u8> {
        let mut doc = Document::with_version("1.5");
        let pages_id = doc.new_object_id();
        let mut xobjects = lopdf::Dictionary::new();
        for (i, (filter, data)) in images.iter().enumerate() {
            let id = doc.add_object(Stream::new(
                dictionary! {
                    "Type" => "XObject",
                    "Subtype" => "Image",
                    "Width" => 2,
                    "Height" => 2,
                    "ColorSpace" => "DeviceGray",
                    "BitsPerComponent" => 8,
                    "Filter" => *filter,
                },
                data.clone(),
            ));
            xobjects.set(format!("Im{i}"), Object::Reference(id));
        }
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "MediaBox" => vec![0.into(), 0.into(), 100.into(), 100.into()],
            "Resources" => dictionary! { "XObject" => xobjects },
        });
        doc.objects.insert(
            pages_id,
            Object::Dictionary(dictionary! {
                "Type" => "Pages",
                "Kids" => vec![page_id.into()],
                "Count" => 1,
            }),
        );
        let catalog = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
        doc.trailer.set("Root", catalog);
        let mut out = Vec::new();
        doc.save_to(&mut out).unwrap();
        out
    }

    #[test]
    fn finds_jpegs_and_skips_other_pictures() {
        let jpeg = vec![0xFF, 0xD8, 0xFF, 0xE0, 1, 2, 3];
        let pdf = pdf_with(&[("DCTDecode", jpeg.clone()), ("FlateDecode", vec![1, 2, 3])]);
        assert_eq!(embedded_jpegs(&pdf), vec![jpeg]);
    }

    #[test]
    fn a_pdf_without_pictures_or_not_a_pdf_gives_nothing() {
        assert!(embedded_jpegs(&pdf_with(&[])).is_empty());
        assert!(embedded_jpegs(b"not a pdf").is_empty());
        // Marked as a JPEG but not one.
        assert!(embedded_jpegs(&pdf_with(&[("DCTDecode", vec![9, 9, 9])])).is_empty());
    }
}
