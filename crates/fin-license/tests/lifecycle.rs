use ed25519_dalek::SigningKey;
use fin_core::NaiveDate;
use fin_license::*;
use rand::rngs::OsRng;

fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).unwrap()
}

#[test]
fn trial_to_licensed_lifecycle() {
    let sk = SigningKey::generate(&mut OsRng);
    let vk = sk.verifying_key();
    let first = d(2026, 11, 1);

    assert!(matches!(
        access(first, d(2026, 11, 2), None),
        Access::Trial { .. }
    ));
    let expired = access(first, d(2026, 12, 20), None);
    assert_eq!(expired, Access::Expired);
    assert!(!expired.can_edit());
    assert!(expired.can_export());

    let payload = LicensePayload {
        product: "golden".into(),
        major: APP_MAJOR,
        issued: d(2026, 12, 20),
        plaid_until: Some(d(2027, 12, 20)),
        buyer: "buyer@example.com".into(),
        core: None,
    };
    let token = issue(&sk, &payload);
    let got = verify(&vk, &token, APP_MAJOR).unwrap();
    assert_eq!(access(first, d(2026, 12, 21), Some(&got)), Access::Licensed);
    assert!(plaid_active(Some(&got), d(2027, 6, 1)));
    assert!(!plaid_active(Some(&got), d(2028, 1, 1)));
}

#[test]
fn tampered_or_foreign_tokens_rejected() {
    let sk = SigningKey::generate(&mut OsRng);
    let other = SigningKey::generate(&mut OsRng);
    let p = LicensePayload {
        product: "golden".into(),
        major: APP_MAJOR,
        issued: d(2026, 1, 1),
        plaid_until: None,
        buyer: "a".into(),
        core: None,
    };
    let token = issue(&sk, &p);
    assert!(verify(&other.verifying_key(), &token, APP_MAJOR).is_err());
    assert!(verify(&sk.verifying_key(), &token, APP_MAJOR + 1).is_err());
    assert!(verify(&sk.verifying_key(), "garbage", APP_MAJOR).is_err());
}
