/// Reduce a raw bank description to a stable merchant key:
/// "POS DEBIT 10/17 TRADER JOE'S #123 SEATTLE WA" -> "trader joe's seattle wa".
pub fn merchant_key(raw: &str) -> String {
    const NOISE: [&str; 14] = [
        "pos",
        "debit",
        "credit",
        "purchase",
        "card",
        "checkcard",
        "visa",
        "recurring",
        "payment",
        "ach",
        "web",
        "id",
        "pmt",
        "withdrawal",
    ];
    let lower = raw.to_lowercase();
    let mut words: Vec<String> = vec![];
    for w in lower.split(|c: char| c.is_whitespace() || c == '*' || c == '/') {
        let w = w.trim_matches(|c: char| !c.is_alphanumeric() && c != '\'' && c != '&');
        if w.is_empty() || NOISE.contains(&w) {
            continue;
        }
        let has_digit = w.chars().any(|c| c.is_ascii_digit());
        if has_digit && w.chars().filter(|c| c.is_alphabetic()).count() <= 1 {
            continue; // store numbers, dates, reference ids like "#123", "10/17", "x8821"
        }
        words.push(w.to_string());
    }
    words.join(" ")
}

/// The leading words of a merchant key that identify the merchant (drops trailing city/state):
/// up to three words.
pub fn merchant_stem(raw: &str) -> String {
    merchant_key(raw)
        .split(' ')
        .take(3)
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_noise() {
        assert_eq!(
            merchant_key("POS DEBIT 10/17 TRADER JOE'S #123 SEATTLE WA"),
            "trader joe's seattle wa"
        );
        assert_eq!(
            merchant_key("CHECKCARD 1017 STARBUCKS STORE 5531"),
            "starbucks store"
        );
        assert_eq!(
            merchant_key("ACH PMT *VERIZON WIRELESS"),
            "verizon wireless"
        );
    }

    #[test]
    fn stable_across_variants() {
        assert_eq!(
            merchant_stem("Amazon.com*MK1234 AMZN.COM/BILL"),
            merchant_stem("AMAZON.COM*MK1234 amzn.com/bill WA")
        );
    }

    #[test]
    fn empty_and_noise_only() {
        assert_eq!(merchant_key(""), "");
        assert_eq!(merchant_key("POS DEBIT 10/17"), "");
    }
}
