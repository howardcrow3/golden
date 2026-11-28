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

/// Leading words that name a payment service or the kind of bank line, not the merchant or
/// person behind it: "ZELLE TO", "PAYPAL *", "DEBIT CARD PURCHASE", "ATM WITHDRAWAL".
const GENERIC: &[&str] = &[
    "zelle",
    "venmo",
    "paypal",
    "pp",
    "cash",
    "app",
    "cashapp",
    "sq",
    "square",
    "tst",
    "atm",
    "check",
    "chk",
    "cheque",
    "ach",
    "debit",
    "credit",
    "card",
    "purchase",
    "pos",
    "checkcard",
    "visa",
    "mastercard",
    "online",
    "mobile",
    "web",
    "payment",
    "pmt",
    "transfer",
    "xfer",
    "recurring",
    "electronic",
    "inst",
    "withdrawal",
    "deposit",
    "to",
    "from",
    "sent",
    "received",
];
/// Generic words that say only how the line was made, not which service or kind it is.
const FILLER: &[&str] = &[
    "payment",
    "pmt",
    "purchase",
    "card",
    "online",
    "mobile",
    "web",
    "recurring",
    "electronic",
    "inst",
    "to",
    "from",
    "sent",
    "received",
];
const OUT: &[&str] = &["to", "sent", "withdrawal"];
const IN: &[&str] = &["from", "received", "deposit"];

/// Numbers written on a line (a cheque number, a reference).
fn numbers(s: &str) -> Vec<String> {
    s.split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty() && w.chars().all(|c| c.is_ascii_digit()))
        .map(|w| w.trim_start_matches('0').to_string())
        .collect()
}

/// Do two merchant word lists plausibly name the same merchant? The first words share a
/// prefix of four letters (or are equal when shorter), or half the words overlap.
fn same_words(wa: &[String], wb: &[String]) -> bool {
    let (Some(fa), Some(fb)) = (wa.first(), wb.first()) else {
        return false;
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

/// Do two payee descriptions plausibly name the same merchant? A leading payment-service or
/// bank-line prefix ("ZELLE TO", "PAYPAL *", "ACH DEBIT") is set aside and what follows it is
/// compared, so two people paid through one service are not one payee; money sent and money
/// received are never the same line. Two blank descriptions count as the same, since amount
/// and date are then all there is to go on.
pub fn same_payee(a: &str, b: &str) -> bool {
    let (wa, wb) = (words(a), words(b));
    if wa.is_empty() || wb.is_empty() {
        return wa.is_empty() && wb.is_empty();
    }
    let split = |w: &[String]| {
        let n = w
            .iter()
            .take_while(|x| GENERIC.contains(&x.as_str()))
            .count();
        (w[..n].to_vec(), w[n..].to_vec())
    };
    let ((pa, ra), (pb, rb)) = (split(&wa), split(&wb));
    let dir = |p: &[String]| {
        (
            p.iter().any(|x| OUT.contains(&x.as_str())),
            p.iter().any(|x| IN.contains(&x.as_str())),
        )
    };
    let ((oa, ia), (ob, ib)) = (dir(&pa), dir(&pb));
    if (oa && !ia && ib && !ob) || (ia && !oa && ob && !ib) {
        return false;
    }
    if ra.is_empty() && rb.is_empty() {
        // Nothing but the prefix ("ATM WITHDRAWAL", "CHECK 1234"): the same kind of line,
        // and not two different numbers.
        let kind = |p: &[String]| {
            let mut k: Vec<String> = p
                .iter()
                .filter(|x| !FILLER.contains(&x.as_str()))
                .cloned()
                .collect();
            k.sort();
            k.dedup();
            k
        };
        let (na, nb) = (numbers(a), numbers(b));
        let numbers_clash = !na.is_empty() && !nb.is_empty() && !na.iter().any(|n| nb.contains(n));
        return kind(&pa) == kind(&pb) && !numbers_clash;
    }
    same_words(&ra, &rb)
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

    #[test]
    fn a_shared_payment_service_or_bank_prefix_is_not_enough() {
        // Different people or shops behind the same service.
        assert!(!same_payee("ZELLE TO JOHN SMITH", "ZELLE TO MARY JONES"));
        assert!(!same_payee("VENMO PAYMENT ALICE", "VENMO PAYMENT BOB"));
        assert!(!same_payee("PAYPAL *EBAY", "PAYPAL *JOHNSMITH"));
        assert!(!same_payee("CASH APP*KEVIN", "CASH APP*LUCY"));
        assert!(!same_payee("SQ *BLUE BOTTLE", "SQ *CORNER BAKERY"));
        assert!(!same_payee(
            "DEBIT CARD PURCHASE STARBUCKS",
            "DEBIT CARD PURCHASE SHELL OIL"
        ));
        assert!(!same_payee("ACH DEBIT GEICO", "ACH DEBIT COMCAST"));
        // Opposite directions are different transactions.
        assert!(!same_payee("ZELLE TO JOHN SMITH", "ZELLE FROM JOHN SMITH"));
        assert!(!same_payee("ATM WITHDRAWAL", "ATM DEPOSIT"));
        // Different services, or different cheques.
        assert!(!same_payee("ZELLE PAYMENT", "VENMO PAYMENT"));
        assert!(!same_payee("CHECK 1234", "CHECK 1235"));
    }

    #[test]
    fn the_same_line_behind_a_prefix_still_matches() {
        assert!(same_payee(
            "ZELLE TO JOHN SMITH",
            "Zelle payment to John Smith"
        ));
        assert!(same_payee("PAYPAL *EBAY", "PayPal *eBay Inc"));
        assert!(same_payee(
            "DEBIT CARD PURCHASE STARBUCKS 55",
            "STARBUCKS STORE"
        ));
        assert!(same_payee("ATM WITHDRAWAL", "ATM WITHDRAWAL #0042"));
        assert!(same_payee("CHECK 1234", "CHECK # 1234"));
        assert!(same_payee("ACH DEBIT GEICO", "GEICO AUTO"));
    }
}
