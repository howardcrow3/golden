use fin_app::ask::parse_period;
use fin_app::App;
use fin_core::NaiveDate;

fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).unwrap()
}

const CSV: &str = "Date,Description,Amount\n\
10/01/2026,ACME PAYROLL,5000.00\n\
10/06/2026,STARBUCKS,-18.40\n\
10/09/2026,CHIPOTLE,-12.60\n\
10/12/2026,LANDLORD RENT,-1500.00\n\
09/05/2026,ACME PAYROLL,5000.00\n\
09/10/2026,MCDONALDS,-40.00\n";

fn app() -> App {
    let mut a = App::in_memory().unwrap().with_clock(|| d(2026, 11, 16));
    a.import_statement(CSV.as_bytes(), "Checking", "checking")
        .unwrap();
    a
}

#[test]
fn periods_in_words() {
    let today = d(2026, 11, 16);
    let (p, l) = parse_period("what did I spend last month", today);
    assert_eq!((p.start, l.as_str()), (d(2026, 10, 1), "last month"));
    let (p, _) = parse_period("spending this month", today);
    assert_eq!(p.start, d(2026, 11, 1));
    let (p, l) = parse_period("income last year", today);
    assert_eq!((p.start, l.as_str()), (d(2025, 1, 1), "last year"));
    let (p, _) = parse_period("what did I earn this quarter", today);
    assert_eq!(p.start, d(2026, 10, 1));
    let (p, l) = parse_period("spent in September", today);
    assert_eq!((p.start, l.as_str()), (d(2026, 9, 1), "in September 2026"));
    // A month later this year than today means last year.
    let (p, _) = parse_period("spent in December", today);
    assert_eq!(p.start, d(2025, 12, 1));
    let (p, l) = parse_period("how much did I spend", today);
    assert_eq!((p.start, l.as_str()), (d(2026, 10, 1), "last month"));
}

#[test]
fn total_spending_names_the_biggest_categories_and_the_lines_behind() {
    let a = app();
    let r = a.ask("How much did I spend last month?").unwrap();
    assert!(r.understood);
    assert_eq!(r.facts[0].cents, 153_100, "{:?}", r.facts);
    assert!(r.text.contains("$1,531.00"), "{}", r.text);
    assert!(r.behind.len() >= 3);
    assert_eq!(r.behind[0].payee, "LANDLORD RENT", "biggest first");
}

#[test]
fn spending_in_a_named_category() {
    let a = app();
    let r = a
        .ask("how much did I spend on dining out last month")
        .unwrap();
    assert_eq!(r.facts[0].cents, 3_100, "{:?} {}", r.facts, r.text);
    assert!(r.text.contains("$31.00"));
    assert!(r
        .behind
        .iter()
        .all(|t| t.category.as_deref() == Some("Dining out")));
    let sep = a
        .ask("what did I spend on dining out in September")
        .unwrap();
    assert_eq!(sep.facts[0].cents, 4_000);
}

#[test]
fn income_and_saving() {
    let a = app();
    let inc = a.ask("what did I earn last month").unwrap();
    assert_eq!(inc.facts[0].cents, 500_000);
    assert!(inc.behind.iter().all(|t| t.amount_cents > 0));
    let save = a.ask("how much did I save last month?").unwrap();
    assert_eq!(save.facts[2].cents, 500_000 - 153_100);
    assert!(save.text.contains("saved"), "{}", save.text);
    assert!(save.text.contains("69%"), "{}", save.text);
}

#[test]
fn overspending_is_said_plainly() {
    let a = app();
    let r = a.ask("did I save anything in September").unwrap();
    assert!(r.text.contains("saved"));
    let mut b = App::in_memory().unwrap().with_clock(|| d(2026, 11, 16));
    b.import_statement(
        b"Date,Description,Amount\n10/05/2026,CAFE,-50.00\n",
        "Checking",
        "checking",
    )
    .unwrap();
    let r = b.ask("how much did I save last month").unwrap();
    assert!(r.text.contains("overspent by $50.00"), "{}", r.text);
}

#[test]
fn net_worth_and_balances() {
    let a = app();
    let nw = a.ask("What is my net worth?").unwrap();
    assert!(nw.understood);
    assert_eq!(nw.facts.len(), 3);
    let bal = a.ask("what's the balance of checking").unwrap();
    assert_eq!(bal.facts[0].label, "Checking");
    assert!(bal.text.starts_with("Checking holds"), "{}", bal.text);
    let which = a.ask("what is my balance").unwrap();
    assert!(which.text.starts_with("Which account?"), "{}", which.text);
}

#[test]
fn an_empty_period_says_so_and_unknown_questions_help() {
    let a = app();
    let r = a.ask("how much did I spend last year").unwrap();
    assert!(r.text.contains("nothing recorded"), "{}", r.text);
    for q in ["", "  ", "tell me a joke", "what's the weather"] {
        let r = a.ask(q).unwrap();
        assert!(!r.understood, "{q}");
        assert!(r.text.contains("I can answer"));
    }
}

#[test]
fn no_data_does_not_error() {
    let a = App::in_memory().unwrap().with_clock(|| d(2026, 11, 16));
    assert!(a.ask("how much did I spend last month").unwrap().understood);
    assert!(a
        .ask("what is the balance of checking")
        .unwrap()
        .text
        .contains("no accounts"));
}
