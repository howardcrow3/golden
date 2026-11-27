//! Reading a shop receipt from its text (a photo run through OCR, or a PDF): who it is from,
//! the date and the total. Label matching only; nothing leaves the computer.
use crate::dates::parse_date;
use fin_core::{Money, NaiveDate};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Receipt {
    pub merchant: String,
    pub total: Option<Money>,
    pub day: Option<NaiveDate>,
}

/// A line that totals the purchase, best label first.
const TOTAL_LABELS: [&str; 5] = [
    "grand total",
    "total due",
    "amount due",
    "balance due",
    "total",
];

/// Lines that contain "total" but are not the amount charged.
const NOT_THE_TOTAL: [&str; 6] = [
    "subtotal",
    "sub total",
    "sub-total",
    "total tax",
    "total savings",
    "total items",
];

fn amounts_in(line: &str) -> Vec<Money> {
    line.split_whitespace()
        .filter_map(|tok| {
            let t = tok
                .trim_matches(|c: char| matches!(c, '$' | '(' | ')' | ','))
                .replace(',', "");
            (t.contains('.') && t.chars().all(|c| c.is_ascii_digit() || c == '.'))
                .then(|| Money::parse(&t).ok())
                .flatten()
        })
        .collect()
}

/// Does this text read like a till receipt (and not a statement or a tax form)?
pub fn looks_like_receipt(text: &str) -> bool {
    let t = text.to_lowercase();
    t.contains("total")
        && (t.contains("subtotal")
            || t.contains("sub total")
            || t.contains("receipt")
            || t.contains("thank you")
            || t.contains("change due"))
}

pub fn parse_receipt(text: &str) -> Receipt {
    let merchant = text
        .lines()
        .map(str::trim)
        .find(|l| {
            let letters = l.chars().filter(|c| c.is_alphabetic()).count();
            letters >= 3 && letters * 2 >= l.chars().filter(|c| !c.is_whitespace()).count()
        })
        .unwrap_or("")
        .to_string();
    let lines: Vec<&str> = text.lines().collect();
    let mut total = None;
    'labels: for label in TOTAL_LABELS {
        // The last matching line wins: the figure at the bottom is the one charged.
        for (i, line) in lines.iter().enumerate().rev() {
            let l = line.to_lowercase();
            if !l.contains(label) || NOT_THE_TOTAL.iter().any(|n| l.contains(n)) {
                continue;
            }
            let found = amounts_in(line)
                .last()
                .copied()
                .or_else(|| lines.get(i + 1).and_then(|n| amounts_in(n).last().copied()));
            if let Some(a) = found {
                total = Some(a);
                break 'labels;
            }
        }
    }
    Receipt {
        merchant,
        total,
        day: find_date(text),
    }
}

/// The first date written in the text, in any of the usual shapes.
fn find_date(text: &str) -> Option<NaiveDate> {
    for line in text.lines() {
        let toks: Vec<&str> = line.split_whitespace().collect();
        for i in 0..toks.len() {
            for n in 1..=3 {
                if i + n > toks.len() {
                    break;
                }
                let s = toks[i..i + n].join(" ");
                let s = s.trim_matches(|c: char| matches!(c, ',' | ';' | ':'));
                if s.chars().any(|c| c.is_ascii_digit()) {
                    if let Some(d) = parse_date(s) {
                        return Some(d);
                    }
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    const RECEIPT: &str = "TRADER JOE'S #123\n123 Main St\n10/17/2026 4:32 PM\nBananas 1.29\nMilk 3.49\nSUBTOTAL 4.78\nTAX 0.00\nTOTAL $4.78\nVISA ****1234 4.78\nThank you for shopping";

    #[test]
    fn reads_merchant_date_and_total() {
        let r = parse_receipt(RECEIPT);
        assert_eq!(r.merchant, "TRADER JOE'S #123");
        assert_eq!(r.total, Some(Money::from_cents(478)));
        assert_eq!(r.day, NaiveDate::from_ymd_opt(2026, 10, 17));
        assert!(looks_like_receipt(RECEIPT));
    }

    #[test]
    fn the_total_is_not_the_subtotal_or_the_tax() {
        let t = "Cafe\nSubtotal 10.00\nTotal tax 0.80\nTotal 10.80\nChange due 0.00";
        assert_eq!(parse_receipt(t).total, Some(Money::from_cents(1_080)));
    }

    #[test]
    fn total_on_the_next_line_and_written_out_dates() {
        let t = "Hardware Store\nOct 17, 2026\nAmount due\n$1,234.56\nThank you";
        let r = parse_receipt(t);
        assert_eq!(r.total, Some(Money::from_cents(123_456)));
        assert_eq!(r.day, NaiveDate::from_ymd_opt(2026, 10, 17));
    }

    #[test]
    fn statements_and_empty_text_are_not_receipts() {
        assert!(!looks_like_receipt(
            "Statement period 10/1 - 10/31 Beginning balance 100.00 Total 5.00"
        ));
        assert!(!looks_like_receipt(""));
        assert_eq!(parse_receipt(""), Receipt::default());
    }
}
