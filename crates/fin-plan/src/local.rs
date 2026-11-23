//! City and county income taxes on top of the state's. Each is a percent of earned income, or
//! of taxable income for New York City and the Maryland counties. Rates are approximate and
//! change often: Philadelphia's 3.735% resident wage tax (from July 1, 2026) was read from
//! phila.gov; New York City's top 3.876% rate was read from tax.ny.gov; Pittsburgh's 3% (1% city
//! plus 2% school district) from pittsburghpa.gov; and the 3.2% Baltimore City and Montgomery
//! County rates from Maryland's 2026 local tax rate table (dls.maryland.gov), all in Nov 2026.
//! New York City's bracket thresholds and the Ohio and Michigan rates are the usual published
//! figures that still need checking against an official page; re-check them each year.
use fin_core::Money;

use crate::tax::FilingStatus;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Base {
    /// Wages and self-employment income only.
    Earned,
    /// Everything that is taxable income.
    Taxable,
}

#[derive(Debug, Clone, Copy)]
pub struct Locality {
    pub key: &'static str,
    pub name: &'static str,
    pub state: &'static str,
    pub rate: f64,
    pub base: Base,
}

pub const LOCALITIES: &[Locality] = &[
    Locality {
        key: "nyc",
        name: "New York City (resident)",
        state: "NY",
        rate: 0.03876,
        base: Base::Taxable,
    },
    Locality {
        key: "philadelphia",
        name: "Philadelphia (resident wage tax)",
        state: "PA",
        rate: 0.03735,
        base: Base::Earned,
    },
    Locality {
        key: "pittsburgh",
        name: "Pittsburgh (resident)",
        state: "PA",
        rate: 0.03,
        base: Base::Earned,
    },
    Locality {
        key: "columbus",
        name: "Columbus, Ohio",
        state: "OH",
        rate: 0.025,
        base: Base::Earned,
    },
    Locality {
        key: "cleveland",
        name: "Cleveland, Ohio",
        state: "OH",
        rate: 0.025,
        base: Base::Earned,
    },
    Locality {
        key: "detroit",
        name: "Detroit (resident)",
        state: "MI",
        rate: 0.024,
        base: Base::Earned,
    },
    Locality {
        key: "baltimore",
        name: "Baltimore City",
        state: "MD",
        rate: 0.032,
        base: Base::Taxable,
    },
    Locality {
        key: "montgomery_md",
        name: "Montgomery County, Maryland",
        state: "MD",
        rate: 0.032,
        base: Base::Taxable,
    },
];

pub fn find(key: &str) -> Option<&'static Locality> {
    LOCALITIES.iter().find(|l| l.key == key)
}

/// New York City resident tax on taxable income, with its four brackets.
fn nyc(fs: FilingStatus, taxable: Money) -> Money {
    let (a, b, c) = match fs {
        FilingStatus::MarriedJoint => (21_600, 45_000, 90_000),
        FilingStatus::HeadOfHousehold => (14_400, 30_000, 60_000),
        _ => (12_000, 25_000, 50_000),
    };
    let t = taxable.cents().max(0) as f64 / 100.0;
    let band = |lo: i64, hi: i64, r: f64| (t.min(hi as f64) - lo as f64).max(0.0) * r;
    let tax = band(0, a, 0.03078)
        + band(a, b, 0.03762)
        + band(b, c, 0.03819)
        + band(c, i64::MAX / 2, 0.03876);
    Money::from_cents((tax * 100.0).round() as i64)
}

/// The tax for a year. `earned` is wages and self-employment; `taxable` is taxable income
/// after the state's deductions (use what the plan knows; it is approximate).
pub fn tax(loc: &Locality, fs: FilingStatus, earned: Money, taxable: Money) -> Money {
    if loc.key == "nyc" {
        return nyc(fs, taxable);
    }
    let base = match loc.base {
        Base::Earned => earned,
        Base::Taxable => taxable,
    };
    base.max(Money::ZERO).mul_rate(loc.rate)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(n: i64) -> Money {
        Money::from_dollars(n)
    }

    #[test]
    fn philadelphia_taxes_wages_only() {
        let p = find("philadelphia").unwrap();
        assert_eq!(
            tax(p, FilingStatus::Single, d(100_000), d(80_000)),
            d(3_735)
        );
        assert_eq!(
            tax(p, FilingStatus::Single, Money::ZERO, d(80_000)),
            Money::ZERO
        );
    }

    #[test]
    fn nyc_brackets_add_up() {
        let n = find("nyc").unwrap();
        // Single, $60,000: 12k at 3.078, 13k at 3.762, 25k at 3.819, 10k at 3.876.
        let want =
            12_000.0 * 0.03078 + 13_000.0 * 0.03762 + 25_000.0 * 0.03819 + 10_000.0 * 0.03876;
        let got = tax(n, FilingStatus::Single, d(60_000), d(60_000)).cents() as f64 / 100.0;
        assert!((got - want).abs() < 0.02, "{got} vs {want}");
        // Joint thresholds are wider, so the same income pays less.
        assert!(
            tax(n, FilingStatus::MarriedJoint, d(60_000), d(60_000))
                < tax(n, FilingStatus::Single, d(60_000), d(60_000))
        );
        assert_eq!(tax(n, FilingStatus::Single, d(0), d(0)), Money::ZERO);
    }

    #[test]
    fn maryland_counties_use_taxable_income() {
        let m = find("baltimore").unwrap();
        assert_eq!(
            tax(m, FilingStatus::Single, Money::ZERO, d(50_000)),
            d(1_600)
        );
    }

    #[test]
    fn every_locality_has_a_unique_key_and_a_state() {
        for (i, l) in LOCALITIES.iter().enumerate() {
            assert_eq!(l.state.len(), 2);
            assert!(LOCALITIES.iter().skip(i + 1).all(|o| o.key != l.key));
        }
        assert!(find("nowhere").is_none());
    }
}
