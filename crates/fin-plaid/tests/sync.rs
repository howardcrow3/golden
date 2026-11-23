use fin_core::Money;
use fin_plaid::proxy::*;
use fin_plaid::*;
use std::cell::RefCell;

struct Fake {
    pages: RefCell<Vec<Result<SyncPage, PlaidError>>>,
    cursors: RefCell<Vec<Option<String>>>,
}

impl Fake {
    fn new(pages: Vec<Result<SyncPage, PlaidError>>) -> Fake {
        Fake {
            pages: RefCell::new(pages.into_iter().rev().collect()),
            cursors: RefCell::new(vec![]),
        }
    }
}

impl PlaidApi for Fake {
    fn sync(&self, cursor: Option<&str>) -> Result<SyncPage, PlaidError> {
        self.cursors.borrow_mut().push(cursor.map(str::to_string));
        self.pages.borrow_mut().pop().expect("no more pages")
    }
}

fn txn(id: &str, amount: f64) -> PlaidTxn {
    serde_json::from_value(serde_json::json!({
        "transaction_id": id, "account_id": "acc1", "date": "2026-11-02",
        "amount": amount, "name": "SQ *BLUE BOTTLE 123", "merchant_name": "Blue Bottle",
        "pending": false, "personal_finance_category": {"primary": "FOOD_AND_DRINK", "detailed": "X"}
    }))
    .unwrap()
}

fn page(added: Vec<PlaidTxn>, next: &str, more: bool) -> SyncPage {
    SyncPage {
        added,
        modified: vec![],
        removed: vec![],
        next_cursor: next.into(),
        has_more: more,
    }
}

#[test]
fn spending_is_negative_and_payee_prefers_merchant() {
    let m = map_txn(&txn("t1", 4.75));
    assert_eq!(m.amount, Money::from_cents(-475));
    assert_eq!(m.payee, "Blue Bottle");
    assert_eq!(m.memo, "SQ *BLUE BOTTLE 123");
    assert_eq!(m.category_hint.as_deref(), Some("FOOD_AND_DRINK"));
}

#[test]
fn refunds_and_pay_come_in_as_positive() {
    assert_eq!(
        map_txn(&txn("t", -2500.0)).amount,
        Money::from_cents(250000)
    );
}

#[test]
fn cents_round_cleanly() {
    assert_eq!(to_cents(19.99), 1999);
    assert_eq!(to_cents(0.1 + 0.2), 30);
    assert_eq!(to_cents(-5.005), -501);
}

#[test]
fn missing_merchant_falls_back_to_name() {
    let t: PlaidTxn = serde_json::from_value(serde_json::json!({
        "transaction_id": "t", "account_id": "a", "date": "2026-01-05", "amount": 10.0,
        "name": "CHECK 1042"
    }))
    .unwrap();
    let m = map_txn(&t);
    assert_eq!(m.payee, "CHECK 1042");
    assert_eq!(m.memo, "");
    assert!(!m.pending);
}

#[test]
fn pages_are_followed_with_the_new_cursor() {
    let api = Fake::new(vec![
        Ok(page(vec![txn("a", 1.0)], "c1", true)),
        Ok(page(vec![txn("b", 2.0)], "c2", false)),
    ]);
    let out = sync_all(&api, Some("c0")).unwrap();
    assert_eq!(out.added.len(), 2);
    assert_eq!(out.cursor, "c2");
    assert_eq!(
        *api.cursors.borrow(),
        vec![Some("c0".to_string()), Some("c1".to_string())]
    );
}

#[test]
fn removed_and_modified_are_collected() {
    let mut p = page(vec![], "c9", false);
    p.modified = vec![txn("m", 3.0)];
    p.removed = vec![RemovedTxn {
        transaction_id: "gone".into(),
    }];
    let out = sync_all(&Fake::new(vec![Ok(p)]), None).unwrap();
    assert_eq!(out.modified[0].id, "m");
    assert_eq!(out.removed, vec!["gone".to_string()]);
}

#[test]
fn mutation_restarts_from_the_original_cursor_and_drops_partial_results() {
    let api = Fake::new(vec![
        Ok(page(vec![txn("stale", 1.0)], "c1", true)),
        Err(PlaidError::MutationDuringPagination),
        Ok(page(vec![txn("fresh", 2.0)], "c5", false)),
    ]);
    let out = sync_all(&api, Some("c0")).unwrap();
    assert_eq!(out.added.len(), 1);
    assert_eq!(out.added[0].id, "fresh");
    assert_eq!(
        *api.cursors.borrow(),
        vec![
            Some("c0".to_string()),
            Some("c1".to_string()),
            Some("c0".to_string())
        ]
    );
}

#[test]
fn restarts_are_capped() {
    let mut pages = vec![];
    for _ in 0..=MAX_RESTARTS {
        pages.push(Err(PlaidError::MutationDuringPagination));
    }
    assert_eq!(
        sync_all(&Fake::new(pages), None),
        Err(PlaidError::MutationDuringPagination)
    );
}

#[test]
fn other_errors_stop_at_once() {
    let api = Fake::new(vec![Err(PlaidError::LoginRequired)]);
    assert_eq!(sync_all(&api, None), Err(PlaidError::LoginRequired));
}

#[test]
fn error_codes_map() {
    assert_eq!(
        parse_error(r#"{"error_code":"ITEM_LOGIN_REQUIRED","error_message":"x"}"#),
        PlaidError::LoginRequired
    );
    assert_eq!(
        parse_error(r#"{"error_code":"RATE_LIMIT","error_message":"slow down"}"#),
        PlaidError::Api {
            code: "RATE_LIMIT".into(),
            message: "slow down".into()
        }
    );
    assert!(matches!(parse_error("<html>"), PlaidError::Malformed(_)));
}

#[test]
fn bodies_carry_the_licence_and_skip_an_empty_cursor() {
    let b = sync_body("LIC", "access-1", Some(""));
    assert_eq!(b["licence"], "LIC");
    assert!(b.get("cursor").is_none());
    assert_eq!(sync_body("L", "a", Some("c"))["cursor"], "c");
    assert_eq!(exchange_body("L", "public-1")["public_token"], "public-1");
}

#[test]
fn answers_parse() {
    let t: LinkToken = parse_ok(r#"{"link_token":"link-sandbox-1"}"#).unwrap();
    assert_eq!(t.link_token, "link-sandbox-1");
    let e: Exchanged = parse_ok(r#"{"access_token":"a","item_id":"i"}"#).unwrap();
    assert_eq!(e.item_id, "i");
    assert!(parse_ok::<LinkToken>("{}").is_err());
}

#[test]
fn a_real_looking_page_deserializes() {
    let body = r#"{"added":[{"transaction_id":"x","account_id":"a","date":"2026-11-02","amount":12.5,
      "name":"Shell","pending":true,"iso_currency_code":"USD","extra":1}],
      "modified":[],"removed":[{"transaction_id":"y","account_id":"a"}],
      "next_cursor":"abc","has_more":false,"request_id":"r"}"#;
    let p: SyncPage = parse_ok(body).unwrap();
    assert!(p.added[0].pending);
    assert_eq!(p.removed[0].transaction_id, "y");
}
