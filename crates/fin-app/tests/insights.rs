use fin_app::App;
use fin_core::NaiveDate;

fn today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 11, 16).unwrap()
}

fn app(csv_lines: &[&str]) -> App {
    let mut app = App::in_memory().unwrap().with_clock(today);
    let mut s = String::from("Date,Description,Amount\n");
    for l in csv_lines {
        s += l;
        s += "\n";
    }
    app.import_statement(s.as_bytes(), "Checking", "checking")
        .unwrap();
    app
}

fn kinds(a: &App) -> Vec<String> {
    a.insights().unwrap().into_iter().map(|i| i.kind).collect()
}

#[test]
fn no_data_no_insights() {
    assert!(App::in_memory()
        .unwrap()
        .with_clock(today)
        .insights()
        .unwrap()
        .is_empty());
}

#[test]
fn savings_is_reported_for_the_last_full_month() {
    let a = app(&[
        "10/01/2026,ACME PAYROLL,5000.00",
        "10/03/2026,RENT PAYMENT LANDLORD,-1500.00",
        "10/10/2026,POS DEBIT TRADER JOE'S #12,-500.00",
    ]);
    let i = a.insights().unwrap();
    let s = i.iter().find(|x| x.kind == "savings").unwrap();
    assert!(s.text.contains("October"), "{}", s.text);
    assert!(
        s.text.contains("$3,000") && s.text.contains("$5,000") && s.text.contains("60%"),
        "{}",
        s.text
    );
}

#[test]
fn overspending_the_month_is_said_plainly() {
    let a = app(&[
        "10/01/2026,ACME PAYROLL,1000.00",
        "10/03/2026,RENT PAYMENT LANDLORD,-1500.00",
    ]);
    let s = &a.insights().unwrap()[0];
    assert!(
        s.text.contains("spent $500 more than came in"),
        "{}",
        s.text
    );
}

#[test]
fn falls_back_to_this_month_when_last_month_is_empty() {
    let a = app(&[
        "11/01/2026,ACME PAYROLL,1000.00",
        "11/03/2026,RENT PAYMENT LANDLORD,-400.00",
    ]);
    let s = &a.insights().unwrap()[0];
    assert!(s.text.contains("November"), "{}", s.text);
}

#[test]
fn a_big_jump_in_a_category_is_called_out_but_small_ones_are_not() {
    let up = app(&[
        "09/05/2026,POS DEBIT TRADER JOE'S #12,-200.00",
        "09/20/2026,ACME PAYROLL,3000.00",
        "10/05/2026,POS DEBIT TRADER JOE'S #12,-500.00",
        "10/20/2026,ACME PAYROLL,3000.00",
    ]);
    let i = up.insights().unwrap();
    let c = i.iter().find(|x| x.kind == "category_up").expect("a jump");
    assert!(
        c.text.contains("$500") && c.text.contains("up $300") && c.text.contains("September"),
        "{}",
        c.text
    );

    let flat = app(&[
        "09/05/2026,POS DEBIT TRADER JOE'S #12,-200.00",
        "09/20/2026,ACME PAYROLL,3000.00",
        "10/05/2026,POS DEBIT TRADER JOE'S #12,-210.00",
        "10/20/2026,ACME PAYROLL,3000.00",
    ]);
    assert!(!kinds(&flat).contains(&"category_up".to_string()));
}

#[test]
fn blown_budgets_are_listed() {
    let a = app(&[
        "10/20/2026,ACME PAYROLL,3000.00",
        "10/05/2026,POS DEBIT TRADER JOE'S #12,-500.00",
    ]);
    let cat = a.categories_str("2026-10-01", "month", false).unwrap()[0]
        .category
        .clone();
    a.set_budget(&cat, 30_000).unwrap();
    let i = a.insights().unwrap();
    let o = i
        .iter()
        .find(|x| x.kind == "overspent")
        .expect("over budget");
    assert!(
        o.text.contains(&cat) && o.text.contains("$200 over"),
        "{}",
        o.text
    );
}

#[test]
fn uncategorized_transactions_are_counted_with_the_right_grammar() {
    let a = app(&[
        "10/20/2026,ACME PAYROLL,3000.00",
        "10/05/2026,ZZQX UNKNOWN VENDOR,-10.00",
    ]);
    let n = a
        .insights()
        .unwrap()
        .into_iter()
        .find(|x| x.kind == "needs_category")
        .unwrap();
    assert!(
        n.text
            .starts_with("1 transaction in October still needs a category"),
        "{}",
        n.text
    );
    let b = app(&[
        "10/20/2026,ACME PAYROLL,3000.00",
        "10/05/2026,ZZQX UNKNOWN VENDOR,-10.00",
        "10/06/2026,QQXZ OTHER VENDOR,-11.00",
    ]);
    let n = b
        .insights()
        .unwrap()
        .into_iter()
        .find(|x| x.kind == "needs_category")
        .unwrap();
    assert!(
        n.text
            .starts_with("2 transactions in October still need a category"),
        "{}",
        n.text
    );
}
