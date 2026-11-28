use ed25519_dalek::SigningKey;
use fin_app::bank::{BankTransport, SecretStore};
use fin_app::App;
use fin_core::NaiveDate;
use fin_license::{issue, LicensePayload, APP_MAJOR};
use rand::rngs::OsRng;
use serde_json::{json, Value};
use std::cell::RefCell;
use std::collections::HashMap;

fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).unwrap()
}

#[derive(Default)]
struct Secrets(RefCell<HashMap<String, String>>);
impl SecretStore for Secrets {
    fn get(&self, k: &str) -> Result<Option<String>, String> {
        Ok(self.0.borrow().get(k).cloned())
    }
    fn set(&self, k: &str, v: &str) -> Result<(), String> {
        self.0.borrow_mut().insert(k.into(), v.into());
        Ok(())
    }
    fn delete(&self, k: &str) -> Result<(), String> {
        self.0.borrow_mut().remove(k);
        Ok(())
    }
}

/// Canned service: answers each path with its latest canned answer and records what was sent.
#[derive(Default)]
struct Fake {
    answers: RefCell<HashMap<String, Vec<(u16, String)>>>,
    sent: RefCell<Vec<(String, Value)>>,
}
impl Fake {
    fn on(&self, path: &str, status: u16, body: Value) {
        // The newest answer for a path replaces any earlier one.
        self.answers
            .borrow_mut()
            .insert(path.into(), vec![(status, body.to_string())]);
    }
}
impl BankTransport for Fake {
    fn post(&self, path: &str, body: &Value) -> Result<(u16, String), String> {
        self.sent.borrow_mut().push((path.into(), body.clone()));
        let mut a = self.answers.borrow_mut();
        let q = a.get_mut(path).ok_or("no answer queued")?;
        Ok(if q.len() > 1 {
            q.remove(0)
        } else {
            q[0].clone()
        })
    }
}

fn app_with_plaid(plaid: bool) -> App {
    let sk = SigningKey::generate(&mut OsRng);
    let token = issue(
        &sk,
        &LicensePayload {
            product: "golden".into(),
            major: APP_MAJOR,
            issued: d(2026, 11, 1),
            plaid_until: plaid.then(|| d(2027, 11, 1)),
            buyer: "b".into(),
        },
    );
    let app = App::in_memory()
        .unwrap()
        .with_public_key(Some(sk.verifying_key()))
        .with_clock(|| d(2026, 11, 23));
    app.activate_license(&token, d(2026, 11, 23)).unwrap();
    app
}

fn page(added: Value, cursor: &str) -> Value {
    json!({
        "accounts": [{"account_id": "pa1", "name": "Total Checking", "mask": "4821",
                      "type": "depository", "subtype": "checking"}],
        "added": added, "modified": [], "removed": [],
        "next_cursor": cursor, "has_more": false
    })
}

fn tx(id: &str, date: &str, amount: f64, name: &str) -> Value {
    json!({"transaction_id": id, "account_id": "pa1", "date": date, "amount": amount,
           "name": name, "pending": false})
}

fn linked(app: &mut App, f: &Fake, s: &Secrets) {
    f.on(
        "/link/exchange",
        200,
        json!({"access_token": "access-1", "item_id": "item-1"}),
    );
    f.on(
        "/transactions/sync",
        200,
        page(
            json!([
                tx("t1", "2026-11-02", 12.5, "SHELL"),
                tx("t2", "2026-11-05", -2000.0, "PAYROLL")
            ]),
            "c1",
        ),
    );
    app.bank_finish_link(f, s, "public-1", "Chase").unwrap();
}

#[test]
fn linking_stores_the_token_in_secrets_and_syncs_the_first_batch() {
    let mut app = app_with_plaid(true);
    let (f, s) = (Fake::default(), Secrets::default());
    linked(&mut app, &f, &s);
    assert_eq!(s.get("plaid:item-1").unwrap().as_deref(), Some("access-1"));
    let items = app.bank_items().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].cursor, "c1");
    assert_eq!(items[0].institution, "Chase");
    let accts = app.accounts().unwrap();
    assert!(accts
        .iter()
        .any(|a| a.name == "Chase Total Checking ••4821"));
    assert_eq!(app.db().txn_count().unwrap(), 2);
    // The token never lands in the database.
    assert!(!app
        .db()
        .get_setting("bank.items")
        .unwrap()
        .unwrap()
        .contains("access-1"));
}

#[test]
fn a_second_sync_sends_the_saved_cursor_and_adds_only_what_is_new() {
    let mut app = app_with_plaid(true);
    let (f, s) = (Fake::default(), Secrets::default());
    linked(&mut app, &f, &s);
    f.on(
        "/transactions/sync",
        200,
        page(json!([tx("t3", "2026-11-09", 4.0, "COFFEE")]), "c2"),
    );
    let r = app.bank_sync(&f, &s, "item-1").unwrap();
    assert_eq!((r.added, r.updated), (1, 0));
    let last = f.sent.borrow().last().unwrap().clone();
    assert_eq!(last.1["cursor"], "c1");
    assert_eq!(app.bank_items().unwrap()[0].cursor, "c2");
    assert_eq!(app.db().txn_count().unwrap(), 3);
}

#[test]
fn without_the_plaid_addon_nothing_is_sent() {
    let mut app = app_with_plaid(false);
    let (f, s) = (Fake::default(), Secrets::default());
    assert!(app
        .bank_link_token(&f)
        .unwrap_err()
        .contains("Plaid add-on"));
    assert!(app.bank_finish_link(&f, &s, "p", "Chase").is_err());
    assert!(f.sent.borrow().is_empty());
}

#[test]
fn link_token_request_carries_the_licence_and_a_stable_random_user() {
    let app = app_with_plaid(true);
    let f = Fake::default();
    f.on("/link/token", 200, json!({"link_token": "link-1"}));
    assert_eq!(app.bank_link_token(&f).unwrap(), "link-1");
    assert_eq!(app.bank_link_token(&f).unwrap(), "link-1");
    let sent = f.sent.borrow();
    assert!(sent[0].1["licence"].as_str().unwrap().contains('.'));
    assert_eq!(sent[0].1["client_user_id"], sent[1].1["client_user_id"]);
    assert_eq!(sent[0].1["client_user_id"].as_str().unwrap().len(), 24);
}

#[test]
fn synced_lines_that_a_statement_already_covers_are_dropped() {
    let mut app = app_with_plaid(true);
    let (f, s) = (Fake::default(), Secrets::default());
    // The statement for the account is already imported, with the Shell line a day later.
    app.import_statement(
        b"Date,Description,Amount\n11/03/2026,SHELL OIL 123,-12.50\n",
        "Chase Total Checking \u{2022}\u{2022}4821",
        "checking",
    )
    .unwrap();
    f.on(
        "/link/exchange",
        200,
        json!({"access_token": "a", "item_id": "item-1"}),
    );
    f.on(
        "/transactions/sync",
        200,
        page(
            json!([
                tx("t1", "2026-11-02", 12.5, "SHELL"),
                tx("t2", "2026-11-05", 7.0, "COFFEE")
            ]),
            "c1",
        ),
    );
    let r = app.bank_finish_link(&f, &s, "p", "Chase").unwrap();
    assert_eq!((r.added, r.reconciled), (2, 1));
    // The statement line and the unmatched coffee remain.
    assert_eq!(app.db().txn_count().unwrap(), 2);
}

#[test]
fn login_required_marks_the_bank_and_stops_further_syncs() {
    let mut app = app_with_plaid(true);
    let (f, s) = (Fake::default(), Secrets::default());
    linked(&mut app, &f, &s);
    f.on(
        "/transactions/sync",
        400,
        json!({"error_code": "ITEM_LOGIN_REQUIRED", "error_message": "x"}),
    );
    let e = app.bank_sync(&f, &s, "item-1").unwrap_err();
    assert!(e.contains("sign in again"));
    assert_eq!(app.bank_items().unwrap()[0].status, "login_required");
    let before = f.sent.borrow().len();
    assert!(app.bank_sync(&f, &s, "item-1").is_err());
    assert_eq!(f.sent.borrow().len(), before);
}

#[test]
fn a_failed_sync_does_not_move_the_cursor() {
    let mut app = app_with_plaid(true);
    let (f, s) = (Fake::default(), Secrets::default());
    linked(&mut app, &f, &s);
    f.on(
        "/transactions/sync",
        500,
        json!({"error_code": "INTERNAL", "error_message": "boom"}),
    );
    assert!(app.bank_sync(&f, &s, "item-1").is_err());
    assert_eq!(app.bank_items().unwrap()[0].cursor, "c1");
}

#[test]
fn paused_banks_do_not_sync_and_resume_works() {
    let mut app = app_with_plaid(true);
    let (f, s) = (Fake::default(), Secrets::default());
    linked(&mut app, &f, &s);
    app.bank_set_paused("item-1", true).unwrap();
    assert!(app
        .bank_sync(&f, &s, "item-1")
        .unwrap_err()
        .contains("paused"));
    f.on("/transactions/sync", 200, page(json!([]), "c9"));
    app.bank_set_paused("item-1", false).unwrap();
    app.bank_sync(&f, &s, "item-1").unwrap();
    assert_eq!(app.bank_items().unwrap()[0].cursor, "c9");
}

#[test]
fn unlinking_deletes_the_token_synced_lines_and_the_item_but_keeps_statements() {
    let mut app = app_with_plaid(true);
    let (f, s) = (Fake::default(), Secrets::default());
    linked(&mut app, &f, &s);
    app.import_statement(
        b"Date,Description,Amount\n10/01/2026,OLD,-1.00\n",
        "Savings",
        "savings",
    )
    .unwrap();
    f.on("/item/remove", 200, json!({}));
    assert_eq!(app.bank_unlink(&f, &s, "item-1").unwrap(), 2);
    assert!(s.get("plaid:item-1").unwrap().is_none());
    assert!(app.bank_items().unwrap().is_empty());
    assert_eq!(app.db().txn_count().unwrap(), 1);
    assert!(f.sent.borrow().iter().any(|(p, _)| p == "/item/remove"));
}

#[test]
fn unlinking_still_cleans_up_when_plaid_is_unreachable() {
    let mut app = app_with_plaid(true);
    let (f, s) = (Fake::default(), Secrets::default());
    linked(&mut app, &f, &s);
    // No /item/remove answer queued: the transport errors.
    app.bank_unlink(&f, &s, "item-1").unwrap();
    assert!(s.get("plaid:item-1").unwrap().is_none());
    assert!(app.bank_items().unwrap().is_empty());
}

#[test]
fn sync_all_reports_each_bank_separately() {
    let mut app = app_with_plaid(true);
    let (f, s) = (Fake::default(), Secrets::default());
    linked(&mut app, &f, &s);
    f.on("/transactions/sync", 200, page(json!([]), "c5"));
    let out = app.bank_sync_all(&f, &s).unwrap();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].bank, "Chase");
    assert!(out[0].report.is_some() && out[0].error.is_none());
}

#[test]
fn unknown_banks_are_refused() {
    let mut app = app_with_plaid(true);
    let (f, s) = (Fake::default(), Secrets::default());
    assert!(app.bank_sync(&f, &s, "nope").is_err());
    assert!(app.bank_unlink(&f, &s, "nope").is_err());
    assert!(app.bank_set_paused("nope", true).is_err());
}

#[test]
fn erasing_everything_does_not_orphan_a_linked_bank() {
    // Erasing would forget the bank while its access token stayed in the keychain and the
    // Plaid connection stayed live, so it asks for the banks to be unlinked first.
    let mut app = app_with_plaid(true);
    let (f, s) = (Fake::default(), Secrets::default());
    linked(&mut app, &f, &s);
    let err = app.erase_all_data("DELETE").unwrap_err();
    assert!(err.contains("nlink"), "{err}");
    assert_eq!(app.bank_items().unwrap().len(), 1, "nothing was erased");
    f.on("/item/remove", 200, json!({}));
    app.bank_unlink(&f, &s, "item-1").unwrap();
    app.erase_all_data("DELETE").unwrap();
    assert!(s.get("plaid:item-1").unwrap().is_none());
}
