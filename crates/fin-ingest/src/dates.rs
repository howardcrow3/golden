use chrono::Datelike;
use fin_core::NaiveDate;

/// Parse the date formats banks actually use: 2026-10-17, 10/17/2026, 10/17/26, 17-Oct-2026,
/// Oct 17, 2026, 20261017.
pub fn parse_date(s: &str) -> Option<NaiveDate> {
    let t = s.trim();
    for f in [
        "%Y-%m-%d",
        "%m/%d/%Y",
        "%m/%d/%y",
        "%d-%b-%Y",
        "%b %d, %Y",
        "%B %d, %Y",
        "%Y/%m/%d",
        "%m-%d-%Y",
        "%Y%m%d",
    ] {
        if let Ok(d) = NaiveDate::parse_from_str(t, f) {
            // %Y happily reads "26" as year 0026; a two-digit year belongs to the %y format.
            if d.year() >= 1900 {
                return Some(d);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    #[test]
    fn common_formats() {
        for s in [
            "2026-10-17",
            "10/17/2026",
            "10/17/26",
            "17-Oct-2026",
            "Oct 17, 2026",
            "October 17, 2026",
            "2026/10/17",
            "10-17-2026",
            "20261017",
            " 10/17/2026 ",
        ] {
            assert_eq!(parse_date(s), Some(d(2026, 10, 17)), "{s}");
        }
    }

    #[test]
    fn rejects_garbage() {
        for s in ["", "tomorrow", "13/45/2026", "2026-02-30"] {
            assert_eq!(parse_date(s), None, "{s}");
        }
    }
}
