use chrono::{Datelike, NaiveDate};
use serde::{Deserialize, Serialize};

/// A household member. Date of birth drives Social Security, Medicare, age-59½ rules,
/// catch-up limits, RMDs and dependent credits.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Person {
    pub id: String,
    pub name: String,
    pub birth_date: NaiveDate,
    pub is_dependent: bool,
}

/// Whole years of age on `on` (birthday counts on the day; Feb 29 birthdays roll to Mar 1 in non-leap years).
pub fn age_on(birth: NaiveDate, on: NaiveDate) -> i32 {
    let mut age = on.year() - birth.year();
    let had =
        (on.month(), on.day()) >= (birth.month(), effective_day(birth, on.year(), on.month()));
    if !had {
        age -= 1;
    }
    age
}

fn effective_day(birth: NaiveDate, year: i32, month: u32) -> u32 {
    // A Feb 29 birthday is observed on Mar 1 in non-leap years.
    if birth.month() == 2 && birth.day() == 29 && !is_leap(year) {
        if month == 2 {
            return 29; // not yet reached in Feb
        }
        return 1.min(birth.day());
    }
    birth.day()
}

fn is_leap(y: i32) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

impl Person {
    pub fn age_on(&self, on: NaiveDate) -> i32 {
        age_on(self.birth_date, on)
    }

    /// The date this person turns `years` (Feb 29 → Mar 1 in non-leap years).
    pub fn date_at_age(&self, years: i32) -> NaiveDate {
        let y = self.birth_date.year() + years;
        NaiveDate::from_ymd_opt(y, self.birth_date.month(), self.birth_date.day())
            .unwrap_or_else(|| NaiveDate::from_ymd_opt(y, 3, 1).unwrap())
    }

    /// The date this person turns `years` years and `months` months (day clamped to month end).
    pub fn date_at_age_months(&self, years: i32, months: u32) -> NaiveDate {
        let total =
            (self.birth_date.year() + years) * 12 + self.birth_date.month0() as i32 + months as i32;
        let (y, m) = (total.div_euclid(12), total.rem_euclid(12) as u32 + 1);
        let mut day = self.birth_date.day();
        loop {
            if let Some(d) = NaiveDate::from_ymd_opt(y, m, day) {
                return d;
            }
            day -= 1;
        }
    }

    /// Age 59½ rule for penalty-free retirement withdrawals.
    pub fn reaches_59_half(&self) -> NaiveDate {
        self.date_at_age_months(59, 6)
    }

    /// Year the person's Social Security full retirement age is reached, via birth year table.
    /// Returns (years, months) of full retirement age.
    pub fn full_retirement_age(&self) -> (u32, u32) {
        match self.birth_date.year() {
            ..=1937 => (65, 0),
            1938 => (65, 2),
            1939 => (65, 4),
            1940 => (65, 6),
            1941 => (65, 8),
            1942 => (65, 10),
            1943..=1954 => (66, 0),
            1955 => (66, 2),
            1956 => (66, 4),
            1957 => (66, 6),
            1958 => (66, 8),
            1959 => (66, 10),
            _ => (67, 0),
        }
    }

    /// Required minimum distributions start age under SECURE 2.0.
    pub fn rmd_start_age(&self) -> i32 {
        match self.birth_date.year() {
            ..=1950 => 72,
            1951..=1959 => 73,
            _ => 75,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }
    fn p(birth: NaiveDate) -> Person {
        Person {
            id: "p".into(),
            name: "P".into(),
            birth_date: birth,
            is_dependent: false,
        }
    }

    #[test]
    fn age_counts_birthdays() {
        let b = d(1980, 6, 15);
        assert_eq!(age_on(b, d(2026, 6, 14)), 45);
        assert_eq!(age_on(b, d(2026, 6, 15)), 46);
        assert_eq!(age_on(b, d(2027, 1, 1)), 46);
    }

    #[test]
    fn leap_day_birthday() {
        let b = d(2000, 2, 29);
        assert_eq!(age_on(b, d(2025, 2, 28)), 24);
        assert_eq!(age_on(b, d(2025, 3, 1)), 25);
        assert_eq!(age_on(b, d(2028, 2, 29)), 28);
    }

    #[test]
    fn age_59_and_a_half() {
        assert_eq!(p(d(1970, 3, 31)).reaches_59_half(), d(2029, 9, 30));
        assert_eq!(p(d(1970, 8, 15)).reaches_59_half(), d(2030, 2, 15));
    }

    #[test]
    fn full_retirement_age_table() {
        assert_eq!(p(d(1956, 5, 1)).full_retirement_age(), (66, 4));
        assert_eq!(p(d(1960, 1, 1)).full_retirement_age(), (67, 0));
        assert_eq!(p(d(1985, 1, 1)).full_retirement_age(), (67, 0));
    }

    #[test]
    fn rmd_ages() {
        assert_eq!(p(d(1950, 1, 1)).rmd_start_age(), 72);
        assert_eq!(p(d(1955, 1, 1)).rmd_start_age(), 73);
        assert_eq!(p(d(1965, 1, 1)).rmd_start_age(), 75);
    }

    #[test]
    fn date_at_age() {
        assert_eq!(p(d(1980, 6, 15)).date_at_age(65), d(2045, 6, 15));
        assert_eq!(p(d(2000, 2, 29)).date_at_age(25), d(2025, 3, 1));
    }
}
