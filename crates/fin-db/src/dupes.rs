//! Spotting the same transaction twice when it arrives from two sources that describe it
//! differently, such as a CSV and an OFX file of one account, or two statements that overlap.
//! The exact-row key catches a file imported twice; this catches the near matches.

/// How many days apart the same transaction may be dated by two sources (posted versus made).
pub const NEAR_DAYS: i64 = 2;

/// Words that tell one merchant from another: lower case letters and digits-free words, minus
/// one-letter fragments ("s" from "Joe's").
fn words(s: &str) -> Vec<String> {
    s.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() > 1 && !w.chars().any(|c| c.is_ascii_digit()))
        .map(|w| w.to_string())
        .collect()
}

/// Do two payee descriptions plausibly name the same merchant? They do when the first words
/// share a prefix of four letters (or are equal when shorter), or half the words overlap. Two
/// blank descriptions count as the same, since amount and date are then all there is to go on.
pub fn same_payee(a: &str, b: &str) -> bool {
    let (wa, wb) = (words(a), words(b));
    let (Some(fa), Some(fb)) = (wa.first(), wb.first()) else {
        return wa.is_empty() && wb.is_empty();
    };
    let common = fa
        .chars()
        .zip(fb.chars())
        .take_while(|(x, y)| x == y)
        .count();
    if common >= 4.min(fa.len()).min(fb.len()) {
        return true;
    }
    let shared = wa.iter().filter(|w| wb.contains(w)).count();
    let union = wa.len() + wb.len() - shared;
    union > 0 && shared * 2 >= union
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_merchant_described_differently() {
        assert!(same_payee("AMAZON MKTPLACE PMTS", "Amazon.com*2K4TT1"));
        assert!(same_payee("TRADER JOE'S #123", "Trader Joes"));
        assert!(same_payee("Shell Oil 5744", "SHELL OIL"));
        assert!(same_payee("", ""));
    }

    #[test]
    fn different_merchants_are_not_the_same() {
        assert!(!same_payee("Starbucks", "Shell Oil"));
        assert!(!same_payee("Starbucks", ""));
        assert!(!same_payee("Target", "Tarot Reader"));
    }
}
