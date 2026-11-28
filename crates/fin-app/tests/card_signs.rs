//! Card CSVs that print charges as positive amounts (Amex, Discover) are read the right way
//! round: a purchase is money out, a payment money in.
use fin_app::App;
use fin_core::NaiveDate;

fn app() -> App {
    App::in_memory()
        .unwrap()
        .with_clock(|| NaiveDate::from_ymd_opt(2026, 11, 20).unwrap())
}

/// Amount by payee for October.
fn amounts(a: &App) -> Vec<(String, i64)> {
    let mut v: Vec<(String, i64)> = a
        .transactions("2026-10-01", "2026-10-31", false)
        .unwrap()
        .into_iter()
        .map(|t| (t.payee, t.amount_cents))
        .collect();
    v.sort();
    v
}

fn cents(a: &App, payee: &str) -> i64 {
    amounts(a)
        .into_iter()
        .find(|(p, _)| p == payee)
        .unwrap_or_else(|| panic!("no {payee}"))
        .1
}

/// American Express "extended details" download: charges positive, credits negative.
const AMEX: &[u8] = b"Date,Description,Card Member,Account #,Amount,Extended Details,Appears On Your Statement As,Address,City/State,Zip Code,Country,Reference,Category\n\
10/03/2026,AMAZON MARKETPLACE NA PA,JANE DOE,-41005,52.17,\"AMZN Mktp US AMZN.COM/BILL WA\",AMAZON MARKETPLACE NA PA,410 TERRY AVE N,\"SEATTLE\nWA\",98109,UNITED STATES,'320262770000000001',Merchandise & Supplies-Internet Purchase\n\
10/05/2026,BLUE BOTTLE COFFEE,JANE DOE,-41005,4.50,,BLUE BOTTLE COFFEE,,\"OAKLAND\nCA\",94607,UNITED STATES,'320262790000000002',Restaurant-Restaurant\n\
10/09/2026,DELTA AIR LINES,JANE DOE,-41005,320.40,,DELTA AIR LINES,,\"ATLANTA\nGA\",30320,UNITED STATES,'320262830000000003',Travel-Airline\n\
10/12/2026,AMAZON MARKETPLACE NA PA,JANE DOE,-41005,-20.00,,AMAZON MARKETPLACE NA PA,,\"SEATTLE\nWA\",98109,UNITED STATES,'320262860000000004',Merchandise & Supplies-Internet Purchase\n\
10/15/2026,AUTOPAY PAYMENT - THANK YOU,JANE DOE,-41005,-500.00,,AUTOPAY PAYMENT - THANK YOU,,,,,'320262890000000005',\n";

/// Discover download: charges positive, payments negative, a Category column.
const DISCOVER: &[u8] = b"Trans. Date,Post Date,Description,Amount,Category\r\n\
10/02/2026,10/03/2026,SHELL OIL 57444,41.00,Gasoline\r\n\
10/05/2026,10/05/2026,STARBUCKS STORE 5531,4.50,Restaurants\r\n\
10/10/2026,10/10/2026,INTERNET PAYMENT - THANK YOU,-300.00,Payments and Credits\r\n\
10/12/2026,10/13/2026,TARGET 00012345,63.20,Merchandise\r\n";

/// Chase card download: charges already negative, the payment positive.
const CHASE: &[u8] = b"Transaction Date,Post Date,Description,Category,Type,Amount,Memo\n\
10/02/2026,10/03/2026,STARBUCKS STORE 5531,Food & Drink,Sale,-4.50,\n\
10/10/2026,10/10/2026,Payment Thank You-Mobile,,Payment,300.00,\n\
10/12/2026,10/13/2026,TARGET 00012345,Shopping,Sale,-63.20,\n\
10/14/2026,10/15/2026,TRADER JOE S #552,Groceries,Sale,-24.18,\n";

#[test]
fn an_amex_file_in_a_card_account_has_purchases_as_money_out() {
    let mut a = app();
    a.import_statement(AMEX, "Amex Gold", "credit").unwrap();
    assert_eq!(cents(&a, "BLUE BOTTLE COFFEE"), -450);
    assert_eq!(cents(&a, "DELTA AIR LINES"), -32_040);
    assert_eq!(cents(&a, "AUTOPAY PAYMENT - THANK YOU"), 50_000);
    let amazon: Vec<i64> = amounts(&a)
        .into_iter()
        .filter(|(p, _)| p == "AMAZON MARKETPLACE NA PA")
        .map(|(_, c)| c)
        .collect();
    assert_eq!(amazon, vec![-5_217, 2_000]);
}

#[test]
fn a_discover_file_is_recognised_by_its_layout() {
    let mut a = app();
    // An account kind that is neither a card nor a bank account: the layout settles it.
    a.import_statement(DISCOVER, "Discover It", "other")
        .unwrap();
    assert_eq!(cents(&a, "SHELL OIL 57444"), -4_100);
    assert_eq!(cents(&a, "INTERNET PAYMENT - THANK YOU"), 30_000);
    let m = a.cash_flow(
        NaiveDate::from_ymd_opt(2026, 10, 1).unwrap(),
        NaiveDate::from_ymd_opt(2026, 10, 31).unwrap(),
        "month",
    );
    assert_eq!(m.unwrap()[0].spending_cents, 4_100 + 450 + 6_320);
}

#[test]
fn a_card_file_that_already_signs_purchases_negative_is_left_alone() {
    let mut a = app();
    a.import_statement(CHASE, "Chase Sapphire", "credit")
        .unwrap();
    assert_eq!(cents(&a, "STARBUCKS STORE 5531"), -450);
    assert_eq!(cents(&a, "Payment Thank You-Mobile"), 30_000);
}

#[test]
fn a_checking_account_is_never_flipped() {
    let mut a = app();
    a.import_statement(DISCOVER, "Checking", "checking")
        .unwrap();
    assert_eq!(cents(&a, "SHELL OIL 57444"), 4_100);
}

#[test]
fn without_a_clear_signal_nothing_is_flipped() {
    let mut a = app();
    // A plain three-column file with no payment line: nothing says which way it runs.
    a.import_statement(
        b"Date,Description,Amount\n10/05/2026,BLUE BOTTLE COFFEE,4.50\n10/09/2026,DELTA AIR LINES,320.40\n",
        "Amex",
        "credit",
    )
    .unwrap();
    assert_eq!(cents(&a, "BLUE BOTTLE COFFEE"), 450);
    // Capital One splits Debit and Credit columns, so the signs are already known.
    a.import_statement(
        b"Transaction Date,Posted Date,Card No.,Description,Category,Debit,Credit\n\
2026-10-02,2026-10-03,1234,SHELL OIL 57444,Gas/Automotive,41.00,\n\
2026-10-10,2026-10-10,1234,CAPITAL ONE MOBILE PYMT,Payment/Credit,,300.00\n",
        "Capital One",
        "credit",
    )
    .unwrap();
    assert_eq!(cents(&a, "SHELL OIL 57444"), -4_100);
    assert_eq!(cents(&a, "CAPITAL ONE MOBILE PYMT"), 30_000);
}

#[test]
fn reimporting_a_flipped_file_adds_nothing() {
    let mut a = app();
    a.import_statement(AMEX, "Amex Gold", "credit").unwrap();
    let r = a.import_statement(AMEX, "Amex Gold", "credit").unwrap();
    assert_eq!(r.added, 0);
}

fn account_id(a: &App, name: &str) -> i64 {
    a.accounts()
        .unwrap()
        .into_iter()
        .find(|x| x.name == name)
        .unwrap()
        .id
}

#[test]
fn an_account_marked_reversed_is_always_flipped() {
    let mut a = app();
    let plain = b"Date,Description,Amount\n10/05/2026,BLUE BOTTLE COFFEE,4.50\n";
    a.import_statement(plain, "Amex", "credit").unwrap();
    assert_eq!(cents(&a, "BLUE BOTTLE COFFEE"), 450);
    let id = account_id(&a, "Amex");
    assert_eq!(a.accounts().unwrap()[0].amounts_reversed, None);
    a.set_amounts_reversed(id, Some(true)).unwrap();
    assert_eq!(a.accounts().unwrap()[0].amounts_reversed, Some(true));
    // No payment line to go on, but the person said so.
    a.import_statement(
        b"Date,Description,Amount\n10/09/2026,DELTA AIR LINES,320.40\n",
        "Amex",
        "credit",
    )
    .unwrap();
    assert_eq!(cents(&a, "DELTA AIR LINES"), -32_040);
}

#[test]
fn an_account_marked_normal_is_never_flipped() {
    let mut a = app();
    a.import_statement(
        b"Date,Description,Amount\n10/01/2026,OPENING,1.00\n",
        "Discover It",
        "other",
    )
    .unwrap();
    a.set_amounts_reversed(account_id(&a, "Discover It"), Some(false))
        .unwrap();
    a.import_statement(DISCOVER, "Discover It", "other")
        .unwrap();
    assert_eq!(cents(&a, "SHELL OIL 57444"), 4_100);
    // Back to detecting.
    a.set_amounts_reversed(account_id(&a, "Discover It"), None)
        .unwrap();
    assert!(a.set_amounts_reversed(999, Some(true)).is_err());
}
