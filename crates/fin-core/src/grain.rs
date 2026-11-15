use crate::money::Money;
use chrono::{Datelike, Duration, NaiveDate, Weekday};
use serde::{Deserialize, Serialize};
use std::fmt;

/// The time grain a screen is viewing: every money screen can switch between these.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Grain {
    Week,
    Month,
    Quarter,
    Year,
}

/// A calendar period identified by its grain and first day. Prints as the sortable key
/// stored with each ledger line: `2026-W07`, `2026-10`, `2026-Q4`, `2026`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct PeriodKey {
    pub grain: Grain,
    pub start: NaiveDate,
}

impl Grain {
    pub const ALL: [Grain; 4] = [Grain::Week, Grain::Month, Grain::Quarter, Grain::Year];

    /// Average length in 1/4800ths of a day (Gregorian), all integers so budget
    /// conversion between grains is exact and never drifts.
    const fn units(self) -> i64 {
        match self {
            Grain::Week => 7 * 4800,
            Grain::Month => 146_097,
            Grain::Quarter => 438_291,
            Grain::Year => 1_753_164,
        }
    }

    /// Convert a recurring budget amount from one grain to another by average day count.
    /// e.g. $1,000/month is about $230.14/week.
    pub fn convert(amount: Money, from: Grain, to: Grain) -> Money {
        amount.mul_ratio(to.units(), from.units())
    }

    pub fn label(self) -> &'static str {
        match self {
            Grain::Week => "Week",
            Grain::Month => "Month",
            Grain::Quarter => "Quarter",
            Grain::Year => "Year",
        }
    }
}

impl PeriodKey {
    /// The period of `grain` that contains `date`. Weeks are ISO (Monday start).
    pub fn containing(grain: Grain, date: NaiveDate) -> PeriodKey {
        let start = match grain {
            Grain::Week => date - Duration::days(date.weekday().num_days_from_monday() as i64),
            Grain::Month => NaiveDate::from_ymd_opt(date.year(), date.month(), 1).unwrap(),
            Grain::Quarter => {
                NaiveDate::from_ymd_opt(date.year(), (date.month0() / 3) * 3 + 1, 1).unwrap()
            }
            Grain::Year => NaiveDate::from_ymd_opt(date.year(), 1, 1).unwrap(),
        };
        PeriodKey { grain, start }
    }

    /// First day of the next period.
    pub fn next_start(&self) -> NaiveDate {
        let s = self.start;
        let add_months = |m: u32| {
            let total = s.year() * 12 + s.month0() as i32 + m as i32;
            NaiveDate::from_ymd_opt(total.div_euclid(12), total.rem_euclid(12) as u32 + 1, 1)
                .unwrap()
        };
        match self.grain {
            Grain::Week => s + Duration::days(7),
            Grain::Month => add_months(1),
            Grain::Quarter => add_months(3),
            Grain::Year => add_months(12),
        }
    }

    pub fn next(&self) -> PeriodKey {
        PeriodKey {
            grain: self.grain,
            start: self.next_start(),
        }
    }

    pub fn prev(&self) -> PeriodKey {
        PeriodKey::containing(self.grain, self.start - Duration::days(1))
    }

    /// Last day (inclusive).
    pub fn end(&self) -> NaiveDate {
        self.next_start() - Duration::days(1)
    }

    pub fn days(&self) -> i64 {
        (self.next_start() - self.start).num_days()
    }

    pub fn contains(&self, d: NaiveDate) -> bool {
        d >= self.start && d <= self.end()
    }

    /// Fraction of this period that has elapsed on `date` (0.0 before, 1.0 after).
    pub fn progress(&self, date: NaiveDate) -> f64 {
        if date < self.start {
            0.0
        } else if date > self.end() {
            1.0
        } else {
            ((date - self.start).num_days() + 1) as f64 / self.days() as f64
        }
    }

    /// A budget stated per `from` grain, expressed for this exact period by its real day count.
    pub fn budget_for(&self, amount: Money, from: Grain) -> Money {
        amount.mul_ratio(self.days() * 4800, from.units())
    }

    /// Every period of `grain` that overlaps [from, to] inclusive.
    pub fn range(grain: Grain, from: NaiveDate, to: NaiveDate) -> Vec<PeriodKey> {
        let mut out = vec![];
        let mut p = PeriodKey::containing(grain, from);
        while p.start <= to {
            out.push(p);
            p = p.next();
        }
        out
    }

    /// ISO week-year and week number for the Week key.
    fn iso(&self) -> (i32, u32) {
        let w = self.start.iso_week();
        (w.year(), w.week())
    }
}

impl fmt::Display for PeriodKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.grain {
            Grain::Week => {
                let (y, w) = self.iso();
                write!(f, "{y}-W{w:02}")
            }
            Grain::Month => write!(f, "{}-{:02}", self.start.year(), self.start.month()),
            Grain::Quarter => write!(f, "{}-Q{}", self.start.year(), self.start.month0() / 3 + 1),
            Grain::Year => write!(f, "{}", self.start.year()),
        }
    }
}

/// Weekday used for week starts (ISO).
pub const WEEK_START: Weekday = Weekday::Mon;

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    #[test]
    fn keys_print_sortably() {
        assert_eq!(
            PeriodKey::containing(Grain::Month, d(2026, 10, 17)).to_string(),
            "2026-10"
        );
        assert_eq!(
            PeriodKey::containing(Grain::Quarter, d(2026, 10, 17)).to_string(),
            "2026-Q4"
        );
        assert_eq!(
            PeriodKey::containing(Grain::Year, d(2026, 10, 17)).to_string(),
            "2026"
        );
        assert_eq!(
            PeriodKey::containing(Grain::Week, d(2026, 2, 12)).to_string(),
            "2026-W07"
        );
    }

    #[test]
    fn iso_week_year_boundary() {
        // 2027-01-01 is a Friday in ISO week 53 of 2026.
        assert_eq!(
            PeriodKey::containing(Grain::Week, d(2027, 1, 1)).to_string(),
            "2026-W53"
        );
        assert_eq!(
            PeriodKey::containing(Grain::Week, d(2027, 1, 1)).start,
            d(2026, 12, 28)
        );
    }

    #[test]
    fn ends_and_lengths() {
        let feb = PeriodKey::containing(Grain::Month, d(2028, 2, 10));
        assert_eq!(feb.days(), 29);
        assert_eq!(feb.end(), d(2028, 2, 29));
        assert_eq!(
            PeriodKey::containing(Grain::Year, d(2028, 6, 1)).days(),
            366
        );
        assert_eq!(
            PeriodKey::containing(Grain::Quarter, d(2026, 11, 15)).end(),
            d(2026, 12, 31)
        );
    }

    #[test]
    fn next_prev_roundtrip_across_year() {
        let dec = PeriodKey::containing(Grain::Month, d(2026, 12, 31));
        assert_eq!(dec.next().start, d(2027, 1, 1));
        assert_eq!(dec.next().prev(), dec);
    }

    #[test]
    fn budget_conversion() {
        let monthly = Money::from_dollars(1000);
        let weekly = Grain::convert(monthly, Grain::Month, Grain::Week);
        assert_eq!(weekly.cents(), 22_998); // 1000 * 7 / 30.436875
        let yearly = Grain::convert(monthly, Grain::Month, Grain::Year);
        assert_eq!(yearly, Money::from_dollars(12_000));
        let q = Grain::convert(Money::from_dollars(3000), Grain::Quarter, Grain::Month);
        assert_eq!(q, Money::from_dollars(1000));
    }

    #[test]
    fn budget_for_real_period() {
        let feb = PeriodKey::containing(Grain::Month, d(2027, 2, 1)); // 28 days
        let per_week = Money::from_dollars(70);
        assert_eq!(
            feb.budget_for(per_week, Grain::Week),
            Money::from_dollars(280)
        );
    }

    #[test]
    fn progress_fraction() {
        let m = PeriodKey::containing(Grain::Month, d(2026, 11, 1));
        assert_eq!(m.progress(d(2026, 10, 31)), 0.0);
        assert!((m.progress(d(2026, 11, 15)) - 0.5).abs() < 1e-9);
        assert_eq!(m.progress(d(2026, 12, 1)), 1.0);
    }

    #[test]
    fn range_covers_dates() {
        let r = PeriodKey::range(Grain::Month, d(2026, 11, 20), d(2027, 2, 3));
        assert_eq!(r.len(), 4);
        assert_eq!(r[0].to_string(), "2026-11");
        assert_eq!(r[3].to_string(), "2027-02");
    }

    proptest! {
        #[test]
        fn period_contains_its_date_and_tiles(days in 0i64..40_000, gi in 0usize..4) {
            let date = d(1950, 1, 1) + Duration::days(days);
            let g = Grain::ALL[gi];
            let p = PeriodKey::containing(g, date);
            prop_assert!(p.contains(date));
            prop_assert!(!p.next().contains(date));
            prop_assert_eq!(p.next().start, p.end() + Duration::days(1));
            prop_assert_eq!(PeriodKey::containing(g, p.start), p);
            prop_assert_eq!(PeriodKey::containing(g, p.end()), p);
        }

        #[test]
        fn convert_roundtrip_is_close(dollars in 1i64..1_000_000, a in 0usize..4, b in 0usize..4) {
            let m = Money::from_dollars(dollars);
            let there = Grain::convert(m, Grain::ALL[a], Grain::ALL[b]);
            let back = Grain::convert(there, Grain::ALL[b], Grain::ALL[a]);
            prop_assert!((back - m).abs().cents() <= 100 + m.cents() / 1000);
        }
    }
}
