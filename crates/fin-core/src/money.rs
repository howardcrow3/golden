use serde::{Deserialize, Serialize};
use std::fmt;
use std::iter::Sum;
use std::ops::{Add, AddAssign, Neg, Sub, SubAssign};
use thiserror::Error;

/// An amount of US dollars stored as whole cents. Never uses floats for storage.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct Money(i64);

#[derive(Debug, Error, PartialEq, Eq)]
pub enum MoneyError {
    #[error("empty amount")]
    Empty,
    #[error("invalid amount: {0}")]
    Invalid(String),
    #[error("amount out of range")]
    Overflow,
}

impl Money {
    pub const ZERO: Money = Money(0);

    pub const fn from_cents(cents: i64) -> Self {
        Money(cents)
    }
    pub const fn cents(self) -> i64 {
        self.0
    }
    pub const fn from_dollars(d: i64) -> Self {
        Money(d * 100)
    }
    pub fn abs(self) -> Self {
        Money(self.0.abs())
    }
    pub fn is_negative(self) -> bool {
        self.0 < 0
    }
    pub fn is_zero(self) -> bool {
        self.0 == 0
    }

    /// Multiply by a rate (e.g. a tax rate), rounding half away from zero to the cent.
    pub fn mul_rate(self, rate: f64) -> Money {
        Money((self.0 as f64 * rate).round() as i64)
    }

    /// Multiply by a ratio of integers, rounding half away from zero. Exact for i64 inputs.
    pub fn mul_ratio(self, num: i64, den: i64) -> Money {
        assert!(den != 0, "zero denominator");
        let n = self.0 as i128 * num as i128;
        let d = den as i128;
        let half = d.abs() / 2;
        let q = if (n >= 0) == (d > 0) {
            (n.abs() + half) / d.abs()
        } else {
            -((n.abs() + half) / d.abs())
        };
        Money(q as i64)
    }

    /// Split into `weights.len()` parts proportional to `weights` so the parts always sum
    /// to exactly `self` (largest-remainder method). All weights must be non-negative
    /// and not all zero.
    pub fn allocate(self, weights: &[u64]) -> Vec<Money> {
        let total: u128 = weights.iter().map(|w| *w as u128).sum();
        assert!(total > 0, "weights must not all be zero");
        let amount = self.0.unsigned_abs() as u128;
        let sign = if self.0 < 0 { -1i64 } else { 1 };
        let mut parts: Vec<(u128, u128)> = weights
            .iter()
            .map(|w| (amount * *w as u128 / total, amount * *w as u128 % total))
            .collect();
        let assigned: u128 = parts.iter().map(|p| p.0).sum();
        let mut left = (amount - assigned) as usize;
        let mut order: Vec<usize> = (0..parts.len()).collect();
        order.sort_by(|a, b| parts[*b].1.cmp(&parts[*a].1).then(a.cmp(b)));
        for i in order {
            if left == 0 {
                break;
            }
            parts[i].0 += 1;
            left -= 1;
        }
        parts
            .into_iter()
            .map(|p| Money(sign * p.0 as i64))
            .collect()
    }

    /// Split evenly into `n` parts that sum exactly to `self`.
    pub fn split_even(self, n: usize) -> Vec<Money> {
        assert!(n > 0);
        self.allocate(&vec![1; n])
    }

    /// Parse "1,234.56", "$1,234.56", "-12.5", "(12.50)" (accounting negative), "12" or "1234.5".
    pub fn parse(s: &str) -> Result<Money, MoneyError> {
        let t = s.trim();
        if t.is_empty() {
            return Err(MoneyError::Empty);
        }
        let mut neg = false;
        let mut t = t.to_string();
        if t.starts_with('(') && t.ends_with(')') {
            neg = true;
            t = t[1..t.len() - 1].to_string();
        }
        if let Some(r) = t.strip_prefix('-') {
            neg = !neg;
            t = r.to_string();
        } else if let Some(r) = t.strip_prefix('+') {
            t = r.to_string();
        }
        if let Some(r) = t.strip_prefix('$') {
            t = r.trim().to_string();
        }
        if let Some(r) = t.strip_prefix('-') {
            neg = !neg;
            t = r.to_string();
        }
        if t.ends_with("CR") || t.ends_with("cr") {
            t.truncate(t.len() - 2);
            t = t.trim().to_string();
        } else if t.ends_with("DR") || t.ends_with("dr") {
            t.truncate(t.len() - 2);
            neg = !neg;
            t = t.trim().to_string();
        }
        let t = t.replace(',', "");
        let (whole, frac) = match t.split_once('.') {
            Some((w, f)) => (w, f),
            None => (t.as_str(), ""),
        };
        let bad = || MoneyError::Invalid(s.to_string());
        if (whole.is_empty() && frac.is_empty())
            || !whole.chars().all(|c| c.is_ascii_digit())
            || !frac.chars().all(|c| c.is_ascii_digit())
            || frac.len() > 2
        {
            return Err(bad());
        }
        let w: i64 = if whole.is_empty() {
            0
        } else {
            whole.parse().map_err(|_| MoneyError::Overflow)?
        };
        let f: i64 = match frac.len() {
            0 => 0,
            1 => frac.parse::<i64>().unwrap() * 10,
            _ => frac.parse().unwrap(),
        };
        let cents = w
            .checked_mul(100)
            .and_then(|v| v.checked_add(f))
            .ok_or(MoneyError::Overflow)?;
        Ok(Money(if neg { -cents } else { cents }))
    }
}

impl fmt::Display for Money {
    /// "$1,234.56" or "-$1,234.56".
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let abs = self.0.unsigned_abs();
        let dollars = (abs / 100).to_string();
        let mut grouped = String::new();
        for (i, c) in dollars.chars().enumerate() {
            if i > 0 && (dollars.len() - i).is_multiple_of(3) {
                grouped.push(',');
            }
            grouped.push(c);
        }
        let sign = if self.0 < 0 { "-" } else { "" };
        write!(f, "{sign}${grouped}.{:02}", abs % 100)
    }
}

impl Add for Money {
    type Output = Money;
    fn add(self, o: Money) -> Money {
        Money(self.0 + o.0)
    }
}
impl Sub for Money {
    type Output = Money;
    fn sub(self, o: Money) -> Money {
        Money(self.0 - o.0)
    }
}
impl Neg for Money {
    type Output = Money;
    fn neg(self) -> Money {
        Money(-self.0)
    }
}
impl AddAssign for Money {
    fn add_assign(&mut self, o: Money) {
        self.0 += o.0;
    }
}
impl SubAssign for Money {
    fn sub_assign(&mut self, o: Money) {
        self.0 -= o.0;
    }
}
impl Sum for Money {
    fn sum<I: Iterator<Item = Money>>(iter: I) -> Money {
        Money(iter.map(|m| m.0).sum())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn display_formats() {
        assert_eq!(Money::from_cents(123456).to_string(), "$1,234.56");
        assert_eq!(Money::from_cents(-5).to_string(), "-$0.05");
        assert_eq!(Money::ZERO.to_string(), "$0.00");
        assert_eq!(
            Money::from_cents(100_000_000_00).to_string(),
            "$100,000,000.00"
        );
    }

    #[test]
    fn parse_variants() {
        for (s, c) in [
            ("1,234.56", 123456),
            ("$1,234.56", 123456),
            ("-12.5", -1250),
            ("(12.50)", -1250),
            ("12", 1200),
            ("0.07", 7),
            (".5", 50),
            ("-$3.00", -300),
            ("$-3.00", -300),
            ("45.10 CR", 4510),
            ("45.10 DR", -4510),
            ("+7", 700),
        ] {
            assert_eq!(Money::parse(s).unwrap().cents(), c, "{s}");
        }
    }

    #[test]
    fn parse_errors() {
        assert_eq!(Money::parse("  "), Err(MoneyError::Empty));
        assert!(Money::parse("abc").is_err());
        assert!(Money::parse("1.234").is_err());
        assert!(Money::parse("1.2.3").is_err());
        assert!(Money::parse(".").is_err());
        assert_eq!(
            Money::parse("99999999999999999999"),
            Err(MoneyError::Overflow)
        );
    }

    #[test]
    fn allocate_keeps_every_cent() {
        let parts = Money::from_cents(100).split_even(3);
        assert_eq!(parts, vec![Money(34), Money(33), Money(33)]);
        assert_eq!(parts.iter().copied().sum::<Money>(), Money(100));
        let neg = Money::from_cents(-100).split_even(3);
        assert_eq!(neg.iter().copied().sum::<Money>(), Money(-100));
        let w = Money::from_cents(1000).allocate(&[1, 2, 7]);
        assert_eq!(w, vec![Money(100), Money(200), Money(700)]);
    }

    #[test]
    fn mul_ratio_rounds_half_away() {
        assert_eq!(Money(5).mul_ratio(1, 2), Money(3));
        assert_eq!(Money(-5).mul_ratio(1, 2), Money(-3));
        assert_eq!(Money(10).mul_ratio(1, 3), Money(3));
        assert_eq!(Money(10).mul_ratio(-1, 3), Money(-3));
    }

    #[test]
    fn mul_rate_rounds() {
        assert_eq!(Money::from_dollars(1000).mul_rate(0.0765), Money(7650));
        assert_eq!(Money(1).mul_rate(0.5), Money(1));
    }

    #[test]
    fn serde_is_transparent() {
        assert_eq!(serde_json::to_string(&Money(42)).unwrap(), "42");
        assert_eq!(serde_json::from_str::<Money>("-7").unwrap(), Money(-7));
    }

    proptest! {
        #[test]
        fn allocate_sums_exactly(cents in -1_000_000_000_000i64..1_000_000_000_000, ws in proptest::collection::vec(0u64..10_000, 1..12)) {
            prop_assume!(ws.iter().sum::<u64>() > 0);
            let parts = Money(cents).allocate(&ws);
            prop_assert_eq!(parts.len(), ws.len());
            prop_assert_eq!(parts.iter().copied().sum::<Money>(), Money(cents));
        }

        #[test]
        fn display_parse_roundtrip(cents in -1_000_000_000_000i64..1_000_000_000_000) {
            let m = Money(cents);
            prop_assert_eq!(Money::parse(&m.to_string()).unwrap(), m);
        }

        #[test]
        fn add_sub_inverse(a in -1_000_000_000i64..1_000_000_000, b in -1_000_000_000i64..1_000_000_000) {
            prop_assert_eq!(Money(a) + Money(b) - Money(b), Money(a));
        }
    }
}
