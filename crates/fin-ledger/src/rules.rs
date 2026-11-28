use crate::normalize::{merchant_key, merchant_stem};
use fin_core::Money;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Rule {
    /// Matches when the normalized merchant contains this text (lowercase).
    pub contains: String,
    pub category: String,
    /// Optional: only match this sign ("out" = spending, "in" = income).
    pub direction: Option<Direction>,
    /// User-created rules beat built-in ones.
    pub user: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    In,
    Out,
}

#[derive(Debug, Clone, Default)]
pub struct RuleSet {
    rules: Vec<Rule>,
}

impl RuleSet {
    /// Built-in starter rules for common US merchants.
    pub fn starter() -> RuleSet {
        let mut r = RuleSet::default();
        let table: &[(&str, &str)] = &[
            ("trader joe", "Groceries"),
            ("whole foods", "Groceries"),
            ("safeway", "Groceries"),
            ("kroger", "Groceries"),
            ("costco", "Groceries"),
            ("starbucks", "Dining out"),
            ("mcdonald", "Dining out"),
            ("chipotle", "Dining out"),
            ("doordash", "Dining out"),
            ("uber eats", "Dining out"),
            ("shell", "Gas"),
            ("chevron", "Gas"),
            ("exxon", "Gas"),
            ("netflix", "Subscriptions"),
            ("spotify", "Subscriptions"),
            ("hulu", "Subscriptions"),
            ("verizon", "Phone and internet"),
            ("comcast", "Phone and internet"),
            ("t-mobile", "Phone and internet"),
            ("pg&e", "Utilities"),
            ("water", "Utilities"),
            ("geico", "Insurance"),
            ("state farm", "Insurance"),
            ("amazon", "Shopping"),
            ("target", "Shopping"),
            ("walmart", "Shopping"),
            ("cvs", "Health"),
            ("walgreens", "Health"),
            ("rent", "Housing"),
            ("mortgage", "Housing"),
            ("irs", "Taxes"),
        ];
        for (c, cat) in table {
            r.rules.push(Rule {
                contains: (*c).into(),
                category: (*cat).into(),
                direction: None,
                user: false,
            });
        }
        r.rules.push(Rule {
            contains: "payroll".into(),
            category: "Salary".into(),
            direction: Some(Direction::In),
            user: false,
        });
        r.rules.push(Rule {
            contains: "interest".into(),
            category: "Interest".into(),
            direction: Some(Direction::In),
            user: false,
        });
        r
    }

    /// The rules the person taught the app, for saving.
    pub fn user_rules(&self) -> Vec<Rule> {
        self.rules.iter().filter(|r| r.user).cloned().collect()
    }

    /// Restore previously saved user rules (replacing any with the same pattern).
    pub fn load_user_rules(&mut self, saved: Vec<Rule>) {
        for r in saved {
            self.rules.retain(|x| !(x.user && x.contains == r.contains));
            self.rules.push(Rule { user: true, ..r });
        }
    }

    /// Every category name the built-in and learned rules can produce, sorted.
    pub fn category_names(&self) -> Vec<String> {
        let mut v: Vec<String> = self.rules.iter().map(|r| r.category.clone()).collect();
        v.sort();
        v.dedup();
        v
    }

    /// Category names that only ever file money coming in (pay, interest), sorted.
    pub fn income_categories(&self) -> Vec<String> {
        let mut v: Vec<String> = self
            .rules
            .iter()
            .filter(|r| r.direction == Some(Direction::In))
            .map(|r| r.category.clone())
            .collect();
        v.sort();
        v.dedup();
        v
    }

    pub fn add_user_rule(&mut self, contains: &str, category: &str) {
        self.rules
            .retain(|r| !(r.user && r.contains == contains.to_lowercase()));
        self.rules.push(Rule {
            contains: contains.to_lowercase(),
            category: category.into(),
            direction: None,
            user: true,
        });
    }

    /// Learn from a correction: remember the merchant key for next time.
    pub fn learn(&mut self, raw_payee: &str, category: &str) {
        let key = merchant_stem(raw_payee);
        if !key.is_empty() {
            self.add_user_rule(&key, category);
        }
    }

    /// Category for a transaction, or None if nothing matches. User rules win, then the
    /// longest (most specific) matching pattern.
    pub fn categorize(&self, raw_payee: &str, amount: Money) -> Option<&str> {
        let key = merchant_key(raw_payee);
        let raw = raw_payee.to_lowercase();
        // Built-in patterns are short ("irs", "rent"), so they must start a word: "first" is
        // not the IRS and "parent" is not rent. The person's own rules match anywhere.
        let at_word_start = |hay: &str, pat: &str| {
            hay.match_indices(pat).any(|(i, _)| {
                hay[..i]
                    .chars()
                    .next_back()
                    .is_none_or(|c| !c.is_alphanumeric())
            })
        };
        self.rules
            .iter()
            .filter(|r| {
                if r.user {
                    key.contains(&r.contains) || raw.contains(&r.contains)
                } else {
                    at_word_start(&key, &r.contains) || at_word_start(&raw, &r.contains)
                }
            })
            .filter(|r| match r.direction {
                None => true,
                Some(Direction::In) => amount > Money::ZERO,
                Some(Direction::Out) => amount < Money::ZERO,
            })
            .max_by_key(|r| (r.user, r.contains.len()))
            .map(|r| r.category.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn out(d: i64) -> Money {
        Money::from_dollars(-d)
    }

    #[test]
    fn user_rules_save_and_restore() {
        let mut a = RuleSet::starter();
        a.learn("MYSTERY VENDOR 22", "Hobbies");
        let saved = a.user_rules();
        assert_eq!(saved.len(), 1);
        let json = serde_json::to_string(&saved).unwrap();
        let mut b = RuleSet::starter();
        b.load_user_rules(serde_json::from_str(&json).unwrap());
        assert_eq!(b.categorize("MYSTERY VENDOR 31", out(4)), Some("Hobbies"));
        assert!(b.category_names().contains(&"Hobbies".to_string()));
        assert!(b.category_names().contains(&"Groceries".to_string()));
    }

    #[test]
    fn starter_rules_match_messy_descriptions() {
        let r = RuleSet::starter();
        assert_eq!(
            r.categorize("POS DEBIT TRADER JOE'S #123", out(40)),
            Some("Groceries")
        );
        assert_eq!(
            r.categorize("CHECKCARD STARBUCKS 5531", out(6)),
            Some("Dining out")
        );
        assert_eq!(r.categorize("NETFLIX.COM", out(15)), Some("Subscriptions"));
        assert_eq!(r.categorize("Random Shop", out(10)), None);
    }

    #[test]
    fn starter_rules_do_not_match_inside_other_words() {
        let r = RuleSet::starter();
        // A card payment to a bank named "First ..." is not a tax payment, and must stay
        // uncategorized so it can pair with the card's side as a transfer.
        assert_eq!(r.categorize("FIRST NATIONAL CARD PMT", out(500)), None);
        assert_eq!(r.categorize("ONLINE PMT CURRENT CARD", out(80)), None);
        assert_eq!(r.categorize("PARENTSQUARE SCHOOL", out(20)), None);
        // Whole-word and word-start matches still work.
        assert_eq!(r.categorize("IRS USATAXPYMT", out(900)), Some("Taxes"));
        assert_eq!(r.categorize("RENT PAYMENT OCT", out(2000)), Some("Housing"));
        assert_eq!(r.categorize("SHELL OIL 5744", out(40)), Some("Gas"));
    }

    #[test]
    fn direction_matters() {
        let r = RuleSet::starter();
        assert_eq!(
            r.categorize("ACME PAYROLL", Money::from_dollars(4000)),
            Some("Salary")
        );
        assert_eq!(r.categorize("ACME PAYROLL", out(4000)), None);
    }

    #[test]
    fn user_rules_beat_starter_rules() {
        let mut r = RuleSet::starter();
        r.add_user_rule("amazon", "Books");
        assert_eq!(r.categorize("AMAZON.COM*123", out(20)), Some("Books"));
    }

    #[test]
    fn longer_pattern_wins_among_starters() {
        let mut r = RuleSet::default();
        r.rules.push(Rule {
            contains: "uber".into(),
            category: "Transport".into(),
            direction: None,
            user: false,
        });
        r.rules.push(Rule {
            contains: "uber eats".into(),
            category: "Dining out".into(),
            direction: None,
            user: false,
        });
        assert_eq!(r.categorize("UBER EATS 123", out(25)), Some("Dining out"));
        assert_eq!(r.categorize("UBER TRIP", out(25)), Some("Transport"));
    }

    #[test]
    fn learning_from_a_correction_generalizes() {
        let mut r = RuleSet::starter();
        assert_eq!(r.categorize("BLUE BOTTLE COFFEE 221", out(7)), None);
        r.learn("POS DEBIT BLUE BOTTLE COFFEE 221 OAKLAND", "Dining out");
        assert_eq!(
            r.categorize("BLUE BOTTLE COFFEE 998 SF", out(5)),
            Some("Dining out")
        );
        assert_eq!(r.categorize("BLUE SKY BAKERY", out(5)), None);
    }

    #[test]
    fn replacing_a_user_rule_does_not_duplicate() {
        let mut r = RuleSet::default();
        r.add_user_rule("gym", "Health");
        r.add_user_rule("gym", "Fitness");
        assert_eq!(r.categorize("City Gym", out(30)), Some("Fitness"));
        assert_eq!(r.rules.len(), 1);
    }

    #[test]
    fn rules_serialize() {
        let r = RuleSet::starter();
        let j = serde_json::to_string(&r.rules[0]).unwrap();
        assert_eq!(serde_json::from_str::<Rule>(&j).unwrap(), r.rules[0]);
    }
}
