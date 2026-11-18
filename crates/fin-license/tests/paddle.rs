use ed25519_dalek::SigningKey;
use fin_core::NaiveDate;
use fin_license::paddle::{licence_from_event, sign, verify_signature, PriceIds, WebhookError};
use fin_license::{verify, VerifyingKey, APP_MAJOR};

const SECRET: &str = "pdl_ntfset_test_secret";
const NOW: i64 = 1_800_000_000;

fn prices() -> PriceIds {
    PriceIds {
        core: "pri_01m7h6qqb0y4f9r6mftw05y772".into(),
        plaid: "pri_01m7h71ez1v1pen447723fmeje".into(),
    }
}
fn key() -> SigningKey {
    SigningKey::from_bytes(&[7u8; 32])
}
fn public() -> VerifyingKey {
    key().verifying_key()
}
fn day(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
}

fn event(kind: &str, price_ids: &[&str], extra: &str) -> String {
    let items: Vec<String> = price_ids
        .iter()
        .map(|p| format!(r#"{{"price":{{"id":"{p}"}},"quantity":1}}"#))
        .collect();
    format!(
        r#"{{"event_id":"evt_1","event_type":"{kind}","occurred_at":"2026-11-20T10:11:12.000Z","data":{{"id":"txn_01abc","customer_id":"ctm_01xyz","items":[{}]{extra}}}}}"#,
        items.join(",")
    )
}

#[test]
fn a_correct_signature_verifies() {
    let body = br#"{"a":1}"#;
    let h = sign(SECRET, NOW, body);
    assert!(verify_signature(&h, body, SECRET, NOW, 300).is_ok());
}

#[test]
fn a_tampered_body_or_wrong_secret_fails() {
    let h = sign(SECRET, NOW, br#"{"a":1}"#);
    assert_eq!(
        verify_signature(&h, br#"{"a":2}"#, SECRET, NOW, 300),
        Err(WebhookError::BadSignature)
    );
    assert_eq!(
        verify_signature(&h, br#"{"a":1}"#, "other", NOW, 300),
        Err(WebhookError::BadSignature)
    );
}

#[test]
fn old_and_future_timestamps_are_refused() {
    let body = b"{}";
    let h = sign(SECRET, NOW - 1000, body);
    assert_eq!(
        verify_signature(&h, body, SECRET, NOW, 300),
        Err(WebhookError::Stale)
    );
    let h = sign(SECRET, NOW + 1000, body);
    assert_eq!(
        verify_signature(&h, body, SECRET, NOW, 300),
        Err(WebhookError::Stale)
    );
    let h = sign(SECRET, NOW - 299, body);
    assert!(verify_signature(&h, body, SECRET, NOW, 300).is_ok());
}

#[test]
fn malformed_headers_are_refused() {
    for bad in ["", "garbage", "ts=abc;h1=00", "ts=1800000000", "h1=00"] {
        assert_eq!(
            verify_signature(bad, b"{}", SECRET, NOW, 300),
            Err(WebhookError::BadHeader),
            "{bad:?}"
        );
    }
    // Non-hex h1 is a mismatch, not a crash.
    assert_eq!(
        verify_signature("ts=1800000000;h1=zz", b"{}", SECRET, NOW, 300),
        Err(WebhookError::BadSignature)
    );
}

#[test]
fn a_rotated_secret_sends_two_signatures_and_either_may_match() {
    let body = b"{}";
    let old = sign("old-secret", NOW, body);
    let new = sign(SECRET, NOW, body);
    let new_h1 = new.split("h1=").nth(1).unwrap();
    let both = format!("{old};h1={new_h1}");
    assert!(verify_signature(&both, body, SECRET, NOW, 300).is_ok());
}

#[test]
fn core_purchase_issues_a_working_licence_without_plaid() {
    let body = event("transaction.completed", &[&prices().core], "");
    let got = licence_from_event(&body, &prices(), &key())
        .unwrap()
        .unwrap();
    assert!(got.bought_core);
    assert_eq!(got.plaid_until, None);
    assert_eq!(got.customer_id, "ctm_01xyz");
    assert_eq!(got.transaction_id, "txn_01abc");
    let p = verify(&public(), &got.token, APP_MAJOR).unwrap();
    assert_eq!(p.issued, day(2026, 11, 20));
    assert_eq!(p.buyer, "ctm_01xyz");
    assert_eq!(p.plaid_until, None);
}

#[test]
fn core_plus_plaid_in_one_checkout() {
    let body = event(
        "transaction.completed",
        &[&prices().core, &prices().plaid],
        r#","billing_period":{"starts_at":"2026-11-20T10:11:12Z","ends_at":"2026-12-20T10:11:12Z"}"#,
    );
    let got = licence_from_event(&body, &prices(), &key())
        .unwrap()
        .unwrap();
    assert!(got.bought_core);
    assert_eq!(got.plaid_until, Some(day(2026, 12, 20)));
    let p = verify(&public(), &got.token, APP_MAJOR).unwrap();
    assert_eq!(p.plaid_until, Some(day(2026, 12, 20)));
}

#[test]
fn plaid_without_a_billing_period_gets_one_month() {
    let body = event("transaction.completed", &[&prices().plaid], "");
    let got = licence_from_event(&body, &prices(), &key())
        .unwrap()
        .unwrap();
    assert!(!got.bought_core);
    assert_eq!(got.plaid_until, Some(day(2026, 12, 20)));
}

#[test]
fn a_monthly_renewal_extends_the_date() {
    let jan = event(
        "transaction.completed",
        &[&prices().plaid],
        r#","billing_period":{"ends_at":"2027-01-20T10:11:12Z"}"#,
    );
    let got = licence_from_event(&jan, &prices(), &key())
        .unwrap()
        .unwrap();
    assert_eq!(got.plaid_until, Some(day(2027, 1, 20)));
}

#[test]
fn other_events_and_other_prices_are_ignored_not_errors() {
    let other_event = event("transaction.created", &[&prices().core], "");
    assert_eq!(
        licence_from_event(&other_event, &prices(), &key()),
        Ok(None)
    );
    let other_price = event(
        "transaction.completed",
        &["pri_01zzzzzzzzzzzzzzzzzzzzzzzz"],
        "",
    );
    assert_eq!(
        licence_from_event(&other_price, &prices(), &key()),
        Ok(None)
    );
}

#[test]
fn broken_bodies_say_what_is_missing() {
    assert!(matches!(
        licence_from_event("nope", &prices(), &key()),
        Err(WebhookError::BadBody(_))
    ));
    let no_customer = r#"{"event_type":"transaction.completed","occurred_at":"2026-11-20T00:00:00Z","data":{"id":"txn_1","items":[{"price":{"id":"pri_01m7h6qqb0y4f9r6mftw05y772"}}]}}"#;
    let e = licence_from_event(no_customer, &prices(), &key()).unwrap_err();
    assert!(e.to_string().contains("customer"), "{e}");
}

#[test]
fn full_round_trip_signed_webhook_to_working_licence() {
    let body = event("transaction.completed", &[&prices().core], "");
    let header = sign(SECRET, NOW, body.as_bytes());
    verify_signature(&header, body.as_bytes(), SECRET, NOW + 5, 300).unwrap();
    let got = licence_from_event(&body, &prices(), &key())
        .unwrap()
        .unwrap();
    // The token is rejected by a different app key, accepted by the right one.
    let other = SigningKey::from_bytes(&[9u8; 32]).verifying_key();
    assert!(verify(&other, &got.token, APP_MAJOR).is_err());
    assert!(verify(&public(), &got.token, APP_MAJOR).is_ok());
}

/// The hosted licence service (services/licence, JavaScript) must issue tokens the app accepts.
/// This token was made by the Rust tool with the same key, and services/licence/test pins the
/// service's output to the same bytes, so the two stay in step.
#[test]
fn the_pinned_service_token_verifies() {
    let token = "eyJwcm9kdWN0IjoiZ29sZGVuIiwibWFqb3IiOjEsImlzc3VlZCI6IjIwMjYtMTEtMTgiLCJwbGFpZF91bnRpbCI6IjIwMjYtMTItMTgiLCJidXllciI6ImN0bV9maXh0dXJlIn0.E1ZYwMv0TqZ3izTtBwqX2gICVC_UUOBCjKMUFf6ns2qqjLf8kgmdmmZMlXamZv5uPFriqZ5CaGQpRuTGAIcFCg";
    let p = verify(&public(), token, APP_MAJOR).unwrap();
    assert_eq!(p.buyer, "ctm_fixture");
    assert_eq!(p.plaid_until, Some(day(2026, 12, 18)));
}
