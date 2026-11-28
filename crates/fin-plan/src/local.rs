//! City and county income taxes on top of the state's. Each is a percent of earned income, or
//! of taxable income for New York City and the Maryland counties. Rates are approximate and
//! change often: Philadelphia's 3.735% resident wage tax (from July 1, 2026) was read from
//! phila.gov; New York City's top 3.876% rate was read from tax.ny.gov; Pittsburgh's 3% (1% city
//! plus 2% school district) from pittsburghpa.gov; and the 3.2% Baltimore City and Montgomery
//! County rates from Maryland's 2026 local tax rate table (dls.maryland.gov), all in Nov 2026.
//! The other Maryland counties use the 2025 Local Tax Rate Chart in the Form 502 instructions,
//! including Anne Arundel's brackets and Frederick's income-based rate (its `rate` field holds the
//! top rate only).
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
    Locality {
        key: "allegany_md",
        name: "Allegany County, Maryland",
        state: "MD",
        rate: 0.0303,
        base: Base::Taxable,
    },
    Locality {
        key: "anne_arundel_md",
        name: "Anne Arundel County, Maryland",
        state: "MD",
        rate: 0.032,
        base: Base::Taxable,
    },
    Locality {
        key: "baltimore_county_md",
        name: "Baltimore County, Maryland",
        state: "MD",
        rate: 0.032,
        base: Base::Taxable,
    },
    Locality {
        key: "calvert_md",
        name: "Calvert County, Maryland",
        state: "MD",
        rate: 0.032,
        base: Base::Taxable,
    },
    Locality {
        key: "caroline_md",
        name: "Caroline County, Maryland",
        state: "MD",
        rate: 0.032,
        base: Base::Taxable,
    },
    Locality {
        key: "carroll_md",
        name: "Carroll County, Maryland",
        state: "MD",
        rate: 0.0303,
        base: Base::Taxable,
    },
    Locality {
        key: "cecil_md",
        name: "Cecil County, Maryland",
        state: "MD",
        rate: 0.0274,
        base: Base::Taxable,
    },
    Locality {
        key: "charles_md",
        name: "Charles County, Maryland",
        state: "MD",
        rate: 0.0303,
        base: Base::Taxable,
    },
    Locality {
        key: "dorchester_md",
        name: "Dorchester County, Maryland",
        state: "MD",
        rate: 0.033,
        base: Base::Taxable,
    },
    Locality {
        key: "frederick_md",
        name: "Frederick County, Maryland",
        state: "MD",
        rate: 0.032,
        base: Base::Taxable,
    },
    Locality {
        key: "garrett_md",
        name: "Garrett County, Maryland",
        state: "MD",
        rate: 0.0265,
        base: Base::Taxable,
    },
    Locality {
        key: "harford_md",
        name: "Harford County, Maryland",
        state: "MD",
        rate: 0.0306,
        base: Base::Taxable,
    },
    Locality {
        key: "howard_md",
        name: "Howard County, Maryland",
        state: "MD",
        rate: 0.032,
        base: Base::Taxable,
    },
    Locality {
        key: "kent_md",
        name: "Kent County, Maryland",
        state: "MD",
        rate: 0.032,
        base: Base::Taxable,
    },
    Locality {
        key: "prince_georges_md",
        name: "Prince George's County, Maryland",
        state: "MD",
        rate: 0.032,
        base: Base::Taxable,
    },
    Locality {
        key: "queen_annes_md",
        name: "Queen Anne's County, Maryland",
        state: "MD",
        rate: 0.032,
        base: Base::Taxable,
    },
    Locality {
        key: "st_marys_md",
        name: "St. Mary's County, Maryland",
        state: "MD",
        rate: 0.032,
        base: Base::Taxable,
    },
    Locality {
        key: "somerset_md",
        name: "Somerset County, Maryland",
        state: "MD",
        rate: 0.032,
        base: Base::Taxable,
    },
    Locality {
        key: "talbot_md",
        name: "Talbot County, Maryland",
        state: "MD",
        rate: 0.024,
        base: Base::Taxable,
    },
    Locality {
        key: "washington_md",
        name: "Washington County, Maryland",
        state: "MD",
        rate: 0.0295,
        base: Base::Taxable,
    },
    Locality {
        key: "wicomico_md",
        name: "Wicomico County, Maryland",
        state: "MD",
        rate: 0.032,
        base: Base::Taxable,
    },
    Locality {
        key: "worcester_md",
        name: "Worcester County, Maryland",
        state: "MD",
        rate: 0.0225,
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

/// Anne Arundel County: 2.7%, then 2.94% over $50,000 ($75,000 joint or head of household),
/// then 3.2% over $400,000 ($480,000).
fn anne_arundel(fs: FilingStatus, taxable: Money) -> Money {
    let (a, b) = match fs {
        FilingStatus::MarriedJoint | FilingStatus::HeadOfHousehold => (75_000, 480_000),
        _ => (50_000, 400_000),
    };
    let t = taxable.cents().max(0) as f64 / 100.0;
    let band = |lo: i64, hi: i64, r: f64| (t.min(hi as f64) - lo as f64).max(0.0) * r;
    let tax = band(0, a, 0.027) + band(a, b, 0.0294) + band(b, i64::MAX / 2, 0.032);
    Money::from_cents((tax * 100.0).round() as i64)
}

/// Frederick County: one rate on all taxable income, set by its level (2.25% up to $25,000,
/// 2.75% to $50,000 ($100,000 joint), 2.96% to $150,000 ($250,000), then 3.2%).
fn frederick(fs: FilingStatus, taxable: Money) -> Money {
    let (b, c) = match fs {
        FilingStatus::MarriedJoint | FilingStatus::HeadOfHousehold => (100_000, 250_000),
        _ => (50_000, 150_000),
    };
    let t = taxable.max(Money::ZERO);
    let rate = if t <= Money::from_dollars(25_000) {
        0.0225
    } else if t <= Money::from_dollars(b) {
        0.0275
    } else if t <= Money::from_dollars(c) {
        0.0296
    } else {
        0.032
    };
    t.mul_rate(rate)
}

/// The tax for a year. `earned` is wages and self-employment; `taxable` is taxable income
/// after the state's deductions (use what the plan knows; it is approximate).
pub fn tax(loc: &Locality, fs: FilingStatus, earned: Money, taxable: Money) -> Money {
    match loc.key {
        "nyc" => return nyc(fs, taxable),
        "anne_arundel_md" => return anne_arundel(fs, taxable),
        "frederick_md" => return frederick(fs, taxable),
        _ => {}
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
    fn every_maryland_county_is_listed() {
        // Baltimore City plus the 23 counties.
        assert_eq!(LOCALITIES.iter().filter(|l| l.state == "MD").count(), 24);
        let t = |k: &str, fs: FilingStatus, inc: i64| tax(find(k).unwrap(), fs, d(0), d(inc));
        assert_eq!(t("talbot_md", FilingStatus::Single, 50_000), d(1_200));
        assert_eq!(t("worcester_md", FilingStatus::Single, 40_000), d(900));
        // Anne Arundel: $1,350 plus 2.94% over $50,000; joint $2,025 plus 2.94% over $75,000.
        assert_eq!(t("anne_arundel_md", FilingStatus::Single, 60_000), d(1_644));
        assert_eq!(
            t("anne_arundel_md", FilingStatus::MarriedJoint, 85_000),
            d(2_319)
        );
        assert_eq!(
            t("anne_arundel_md", FilingStatus::Single, 500_000),
            d(14_840)
        );
        // Frederick: one rate on the whole amount.
        assert_eq!(t("frederick_md", FilingStatus::Single, 20_000), d(450));
        assert_eq!(t("frederick_md", FilingStatus::Single, 60_000), d(1_776));
        assert_eq!(
            t("frederick_md", FilingStatus::MarriedJoint, 60_000),
            d(1_650)
        );
        assert_eq!(t("frederick_md", FilingStatus::Single, 200_000), d(6_400));
    }

    #[test]
    fn the_app_lists_the_same_localities() {
        let ts = include_str!("../../../apps/desktop/src/lib/api.ts");
        for l in LOCALITIES {
            assert!(ts.contains(&format!("key: \"{}\"", l.key)), "{}", l.key);
        }
        let listed = ts.matches("state: \"").count();
        assert!(listed >= LOCALITIES.len());
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
