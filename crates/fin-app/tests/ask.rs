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
fn paying_and_being_charged_are_spending_but_getting_paid_is_income() {
    let a = app();
    for q in [
        "how much have I paid in fees last month",
        "what did I pay last month",
        "how much was I charged last month",
        "how much did we pay for things last month",
    ] {
        let r = a.ask(q).unwrap();
        assert!(r.understood, "{q}");
        assert!(r.text.contains("spent"), "{q}: {}", r.text);
        assert!(r.behind.iter().all(|t| t.amount_cents < 0), "{q}");
    }
    for q in [
        "how much did I get paid last month",
        "what was I paid last month",
        "how much did I earn last month",
    ] {
        let r = a.ask(q).unwrap();
        assert_eq!(r.facts[0].cents, 500_000, "{q}: {}", r.text);
        assert!(r.text.contains("earned"), "{q}: {}", r.text);
    }
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

mod model_routing {
    use super::*;
    use fin_app::ask::parse_route;
    use fin_app::model::Completer;
    use std::sync::{Arc, Mutex};

    type Asked = Arc<Mutex<Vec<(String, String)>>>;

    struct Script {
        reply: String,
        asked: Asked,
    }
    impl Completer for Script {
        fn complete(&self, p: &str) -> Result<String, String> {
            self.complete_task("whatif", p)
        }
        fn complete_task(&self, task: &str, prompt: &str) -> Result<String, String> {
            self.asked
                .lock()
                .unwrap()
                .push((task.into(), prompt.into()));
            Ok(self.reply.clone())
        }
    }

    fn with_model(reply: &str) -> (App, Asked) {
        let mut a = app();
        let asked: Asked = Arc::new(Mutex::new(vec![]));
        a.set_embedded_model(Some(Arc::new(Script {
            reply: reply.into(),
            asked: asked.clone(),
        })));
        (a, asked)
    }

    #[test]
    fn the_model_routes_what_the_rules_cannot_read() {
        let (a, asked) = with_model(r#"{"intent":"spending","month":10,"category":"dining out"}"#);
        let r = a
            .ask("Tell me about my restaurant outlay back in October")
            .unwrap();
        assert!(r.understood);
        assert_eq!(r.facts[0].cents, 3_100, "{:?} {}", r.facts, r.text);
        let asked = asked.lock().unwrap();
        assert_eq!(asked.len(), 1);
        assert_eq!(asked[0].0, "ask_route");
        // The model is told what exists, never any amounts or balances.
        assert!(asked[0].1.contains("Dining out"));
        assert!(!asked[0].1.contains("5000") && !asked[0].1.contains("1500"));
    }

    #[test]
    fn rules_win_so_the_model_is_not_asked_for_clear_questions() {
        let (a, asked) = with_model(r#"{"intent":"income"}"#);
        let r = a.ask("how much did I spend last month").unwrap();
        assert_eq!(r.facts[0].cents, 153_100);
        assert!(asked.lock().unwrap().is_empty());
    }

    #[test]
    fn unusable_model_answers_fall_back_to_the_help_text() {
        for reply in [
            "not json",
            r#"{"intent":"transfer_money"}"#,
            r#"{"intent":"spending","month":13}"#,
            r#"{"grain":"month"}"#,
        ] {
            let (a, _) = with_model(reply);
            let r = a.ask("gibberish outlay thing").unwrap();
            assert!(!r.understood, "{reply}");
        }
    }

    #[test]
    fn invented_categories_and_accounts_are_dropped() {
        let today = d(2026, 11, 16);
        let cats = vec!["Dining out".to_string(), "Rent".to_string()];
        let accts = vec!["Checking".to_string()];
        let q = parse_route(
            r#"{"intent":"spending","grain":"quarter","which":"this","category":"Yachts","account":"Swiss"}"#,
            today,
            &cats,
            &accts,
        )
        .unwrap();
        assert_eq!((q.category, q.account), (None, None));
        assert_eq!(q.period.start, d(2026, 10, 1));
        let q = parse_route(
            r#"{"intent":"balance","account":"CHECKING"}"#,
            today,
            &cats,
            &accts,
        )
        .unwrap();
        assert_eq!(q.account.as_deref(), Some("Checking"));
        assert_eq!(q.label, "last month");
    }

    #[test]
    fn long_digit_runs_in_a_question_never_reach_the_model() {
        let (a, asked) = with_model(r#"{"intent":"net_worth"}"#);
        let r = a.ask("outlay on card 4111111111111111 please").unwrap();
        assert!(r.understood);
        assert!(!asked.lock().unwrap()[0].1.contains("4111111111111111"));
    }

    #[test]
    fn no_model_means_the_help_text() {
        let a = app();
        assert!(!a.ask("outlay on stuff").unwrap().understood);
    }
}
