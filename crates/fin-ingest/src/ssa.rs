//! Read the estimated benefits from a Social Security Statement (the PDF from ssa.gov, as text).
//!
//! The statement lists a monthly amount for claiming at 62, at full retirement age and at 70.
//! Layout varies a little between years, so each amount is found by its label, on the same line
//! or one of the next two. A statement with no readable benefit is refused rather than guessed.
use fin_core::Money;

/// Monthly benefits in today's dollars, as printed on the statement.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SsaStatement {
    pub at_62: Option<Money>,
    pub at_full_retirement: Option<Money>,
    /// The full retirement age in whole years printed on the statement (66 or 67).
    pub full_retirement_age: Option<u32>,
    pub at_70: Option<Money>,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum SsaError {
    #[error("This doesn't look like a Social Security Statement.")]
    NotAStatement,
    #[error("I couldn't find the monthly benefit amounts on this statement.")]
    NoBenefits,
}

/// Dollar amounts on a line, with or without cents: `$1,742`, `$2,490.50`.
fn dollars(line: &str) -> Vec<Money> {
    let mut out = vec![];
    let bytes: Vec<char> = line.chars().collect();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == '$' {
            let mut j = i + 1;
            let mut s = String::new();
            while j < bytes.len()
                && (bytes[j].is_ascii_digit() || bytes[j] == ',' || bytes[j] == '.')
            {
                if bytes[j] != ',' {
                    s.push(bytes[j]);
                }
                j += 1;
            }
            let s = s.trim_end_matches('.');
            if !s.is_empty() {
                if let Ok(m) = Money::parse(s) {
                    out.push(m);
                }
            }
            i = j;
        } else {
            i += 1;
        }
    }
    out
}

/// The first amount on this line or the next two.
fn amount_near(lines: &[&str], at: usize) -> Option<Money> {
    lines
        .iter()
        .skip(at)
        .take(3)
        .find_map(|l| dollars(l).first().copied())
}

fn full_age(line: &str) -> Option<u32> {
    let l = line.to_ascii_lowercase();
    let pos = l.find("full retirement age")?;
    l[pos..]
        .split(|c: char| !c.is_ascii_digit())
        .filter_map(|t| t.parse::<u32>().ok())
        .find(|n| (65..=67).contains(n))
}

pub fn parse(text: &str) -> Result<SsaStatement, SsaError> {
    let lower = text.to_ascii_lowercase();
    if !lower.contains("social security")
        || !(lower.contains("statement") || lower.contains("retirement"))
    {
        return Err(SsaError::NotAStatement);
    }
    let lines: Vec<&str> = text.lines().collect();
    let mut s = SsaStatement::default();
    for (i, line) in lines.iter().enumerate() {
        let l = line.to_ascii_lowercase();
        if s.at_full_retirement.is_none() && l.contains("full retirement age") {
            if let Some(m) = amount_near(&lines, i) {
                s.at_full_retirement = Some(m);
                s.full_retirement_age = full_age(line);
            }
        } else if s.at_62.is_none() && (l.contains("age 62") || l.contains("at 62")) {
            s.at_62 = amount_near(&lines, i);
        } else if s.at_70.is_none() && (l.contains("age 70") || l.contains("at 70")) {
            s.at_70 = amount_near(&lines, i);
        }
    }
    if s.at_62.is_none() && s.at_full_retirement.is_none() && s.at_70.is_none() {
        return Err(SsaError::NoBenefits);
    }
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    const STATEMENT: &str = "Your Social Security Statement\n\
        Retirement\n\
        You have earned enough credits to qualify for benefits.\n\
        At your current earnings, if you continue working until...\n\
        Age 62 (Year 2030): $1,742 a month\n\
        Full retirement age (67 years) (Year 2035): $2,490 a month\n\
        Age 70 (Year 2038): $3,086 a month\n";

    fn d(n: i64) -> Money {
        Money::from_dollars(n)
    }

    #[test]
    fn reads_the_three_monthly_amounts_and_the_retirement_age() {
        let s = parse(STATEMENT).unwrap();
        assert_eq!(s.at_62, Some(d(1_742)));
        assert_eq!(s.at_full_retirement, Some(d(2_490)));
        assert_eq!(s.full_retirement_age, Some(67));
        assert_eq!(s.at_70, Some(d(3_086)));
    }

    #[test]
    fn amounts_may_sit_on_the_next_line() {
        let t = "Social Security Statement\nAge 62\n$1,100.50 a month\nFull retirement age (66)\n$1,600 a month\n";
        let s = parse(t).unwrap();
        assert_eq!(s.at_62, Some(Money::from_cents(110_050)));
        assert_eq!(s.at_full_retirement, Some(d(1_600)));
        assert_eq!(s.full_retirement_age, Some(66));
        assert_eq!(s.at_70, None);
    }

    #[test]
    fn other_documents_and_empty_statements_are_refused() {
        assert_eq!(
            parse("Pay stub\nGross $5,000.00"),
            Err(SsaError::NotAStatement)
        );
        assert_eq!(
            parse("Your Social Security Statement\nYou have not earned enough credits."),
            Err(SsaError::NoBenefits)
        );
    }

    #[test]
    fn dollar_amounts_handle_commas_cents_and_stray_dots() {
        assert_eq!(dollars("$1,742 a month"), vec![d(1_742)]);
        assert_eq!(
            dollars("about $2,490.25."),
            vec![Money::from_cents(249_025)]
        );
        assert!(dollars("no money here").is_empty());
    }
}
