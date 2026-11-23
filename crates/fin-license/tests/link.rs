use fin_license::link::{token_from_link, token_from_links};

const T: &str = "eyJwcm9kdWN0IjoiZ29sZGVuIn0.c2ln-_X";

#[test]
fn an_activation_link_yields_its_token() {
    assert_eq!(
        token_from_link(&format!("golden://activate?key={T}")).as_deref(),
        Some(T)
    );
    assert_eq!(
        token_from_link(&format!("golden://activate/?key={T}")).as_deref(),
        Some(T)
    );
    assert_eq!(
        token_from_link(&format!("  golden://activate?x=1&key={T}#frag ")).as_deref(),
        Some(T)
    );
}

#[test]
fn anything_else_is_ignored() {
    for bad in [
        "",
        "golden://activate",
        "golden://activate?key=",
        "golden://open?key=a.b",
        "https://activate?key=a.b",
        "golden://activate?key=nodot",
        "golden://activate?key=a.b.c",
        "golden://activate?key=a.b%20c",
        "golden://activate?key=a.b<script>",
        "golden://activate?keys=a.b",
    ] {
        assert_eq!(token_from_link(bad), None, "{bad}");
    }
    let huge = format!(
        "golden://activate?key={}.{}",
        "a".repeat(3000),
        "b".repeat(3000)
    );
    assert_eq!(token_from_link(&huge), None);
}

#[test]
fn the_first_good_link_wins() {
    let links = vec![
        "golden://open".to_string(),
        format!("golden://activate?key={T}"),
    ];
    assert_eq!(token_from_links(&links).as_deref(), Some(T));
    assert_eq!(token_from_links::<&str>(&[]), None);
}

#[test]
fn a_real_issued_token_round_trips() {
    use fin_license::{issue, LicensePayload, APP_MAJOR};
    let key = ed25519_dalek::SigningKey::from_bytes(&[7u8; 32]);
    let token = issue(
        &key,
        &LicensePayload {
            product: "golden".into(),
            major: APP_MAJOR,
            issued: fin_core::NaiveDate::from_ymd_opt(2026, 11, 18).unwrap(),
            plaid_until: None,
            buyer: "ctm_x".into(),
        },
    );
    let link = format!("golden://activate?key={token}");
    assert_eq!(token_from_link(&link), Some(token));
}
