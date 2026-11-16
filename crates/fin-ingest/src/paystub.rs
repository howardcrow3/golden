//! Read a paystub from its text (copied from a PDF or a payroll portal export).
//!
//! Payroll providers label things differently, so each field is found by a list of common
//! labels. Per line, the first amount that follows a label wins, which on a typical stub is the
//! current-period column (year-to-date columns come after). Nothing here guesses silently: a stub
//! without a gross and a net pay is refused with a readable reason.
use crate::dates::parse_date;
use fin_core::{Money, NaiveDate};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Paystub {
    pub employer: String,
    pub pay_date: Option<NaiveDate>,
    pub period_start: Option<NaiveDate>,
    pub period_end: Option<NaiveDate>,
    pub gross: Money,
    pub federal_tax: Money,
    pub state_tax: Money,
    pub social_security: Money,
    pub medicare: Money,
    /// Pre-tax retirement contributions (401k, 403b, TSP).
    pub retirement: Money,
    /// Other pre-tax deductions: health, dental, vision, HSA, FSA.
    pub benefits: Money,
    pub net: Money,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum PaystubError {
    #[error("I couldn't find the gross pay on this paystub.")]
    NoGross,
    #[error("I couldn't find the net pay on this paystub.")]
    NoNet,
    #[error("The paystub's numbers don't add up (gross minus deductions is far from net pay).")]
    DoesNotReconcile,
}

/// Amounts on a line: 1,234.56 or $1,234.56 or (12.34) or 12.34-. Returns cents, positive.
fn amounts(line: &str) -> Vec<Money> {
    let mut out = vec![];
    for tok in line.split_whitespace() {
        let t = tok.trim_matches(|c: char| matches!(c, '$' | '(' | ')' | '-' | ','));
        let t = t.replace(',', "");
        if t.contains('.') && t.chars().all(|c| c.is_ascii_digit() || c == '.') {
            if let Ok(m) = Money::parse(&t) {
                out.push(m);
            }
        }
    }
    out
}

fn first_amount(text: &str, labels: &[&str]) -> Option<Money> {
    for line in text.lines() {
        let l = line.to_ascii_lowercase();
        if labels.iter().any(|w| l.contains(w)) {
            if let Some(m) = amounts(line).first() {
                return Some(*m);
            }
        }
    }
    None
}

fn sum_all(text: &str, labels: &[&str]) -> Money {
    text.lines()
        .filter(|line| {
            let l = line.to_ascii_lowercase();
            labels.iter().any(|w| l.contains(w))
        })
        .filter_map(|line| amounts(line).first().copied())
        .sum()
}

fn date_after(text: &str, labels: &[&str]) -> Option<NaiveDate> {
    for line in text.lines() {
        let l = line.to_ascii_lowercase();
        if let Some(label) = labels.iter().find(|w| l.contains(**w)) {
            let rest = &line[l.find(label).unwrap() + label.len()..];
            for tok in rest
                .split(|c: char| c.is_whitespace() || c == ':')
                .filter(|t| !t.is_empty())
            {
                if let Some(d) = parse_date(tok) {
                    return Some(d);
                }
            }
        }
    }
    None
}

pub fn parse(text: &str) -> Result<Paystub, PaystubError> {
    let gross = first_amount(
        text,
        &[
            "gross pay",
            "gross earnings",
            "total gross",
            "gross wages",
            "total earnings",
        ],
    )
    .ok_or(PaystubError::NoGross)?;
    let net = first_amount(text, &["net pay", "take home", "take-home", "net amount"])
        .ok_or(PaystubError::NoNet)?;
    let employer = text
        .lines()
        .map(str::trim)
        .find(|l| {
            let low = l.to_ascii_lowercase();
            low.starts_with("employer") || low.starts_with("company")
        })
        .map(|l| {
            l.split_once(':')
                .map(|x| x.1)
                .unwrap_or(l)
                .trim()
                .to_string()
        })
        .unwrap_or_default();
    let s = Paystub {
        employer,
        pay_date: date_after(text, &["pay date", "check date", "payment date"]),
        period_start: date_after(
            text,
            &["period start", "period beginning", "pay period start"],
        ),
        period_end: date_after(text, &["period end", "period ending", "pay period end"]),
        gross,
        federal_tax: sum_all(
            text,
            &[
                "federal income tax",
                "federal withholding",
                "fed withholding",
                "fit",
            ],
        ),
        state_tax: sum_all(
            text,
            &["state income tax", "state withholding", "state tax"],
        ),
        social_security: sum_all(text, &["social security", "oasdi", "fica ss"]),
        medicare: sum_all(text, &["medicare", "fica med"]),
        retirement: sum_all(text, &["401k", "401(k)", "403b", "403(b)", "tsp"]),
        benefits: sum_all(
            text,
            &["health", "medical", "dental", "vision", "hsa", "fsa"],
        ),
        net,
    };
    // Gross minus everything listed should be within a few dollars of net (other deductions
    // such as garnishments or after-tax items are not listed, so only a lower bound is firm).
    let listed =
        s.federal_tax + s.state_tax + s.social_security + s.medicare + s.retirement + s.benefits;
    if s.net > s.gross || s.gross - listed < s.net - Money::from_cents(500) {
        return Err(PaystubError::DoesNotReconcile);
    }
    Ok(s)
}
