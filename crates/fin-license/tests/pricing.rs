use fin_license::pricing::{check_price_id, Pricing, PricingInfo};

#[test]
fn standard_prices_match_the_data_file() {
    let p = Pricing::standard();
    assert_eq!(p.currency, "USD");
    assert_eq!(p.core_cents, 3999); // $39.99 once per major version
    assert_eq!(p.plaid_cents, 500); // $5 a month
    assert_eq!(p.plaid_interval, "month");
}

#[test]
fn real_looking_price_ids_pass() {
    assert!(check_price_id("pri_01m7h6qqb0y4f9r6mftw05y772").is_ok());
    assert!(check_price_id("  pri_01m7h71ez1v1pen447723fmeje \n").is_ok());
}

#[test]
fn a_product_id_is_refused_with_the_fix() {
    let e = check_price_id("pro_01m7h6qqb0y4f9r6mftw05y772").unwrap_err();
    assert!(e.contains("product ID") && e.contains("pri_"), "{e}");
}

#[test]
fn junk_is_refused() {
    for bad in [
        "",
        "pri_",
        "pri_SHORT",
        "pri_has space 1234567",
        "txn_01m7h6qqb0y4",
        "sk_live_abc",
    ] {
        assert!(check_price_id(bad).is_err(), "{bad:?}");
    }
}

#[test]
fn info_keeps_good_ids_and_reports_bad_ones() {
    let ok = PricingInfo::from_ids(
        Some("pri_01m7h6qqb0y4f9r6mftw05y772"),
        Some("pri_01m7h71ez1v1pen447723fmeje"),
    );
    assert_eq!(
        ok.core_price_id.as_deref(),
        Some("pri_01m7h6qqb0y4f9r6mftw05y772")
    );
    assert_eq!(
        ok.plaid_price_id.as_deref(),
        Some("pri_01m7h71ez1v1pen447723fmeje")
    );
    assert!(ok.warning.is_none());

    let mixed = PricingInfo::from_ids(
        Some("pro_01m7h6qqb0y4f9r6mftw05y772"),
        Some("pri_01m7h71ez1v1pen447723fmeje"),
    );
    assert!(mixed.core_price_id.is_none());
    assert!(mixed.plaid_price_id.is_some());
    assert!(mixed.warning.unwrap().starts_with("Core price:"));
}

#[test]
fn missing_ids_are_not_an_error() {
    for none in [None, Some(""), Some("   ")] {
        let i = PricingInfo::from_ids(none, none);
        assert!(i.core_price_id.is_none() && i.plaid_price_id.is_none() && i.warning.is_none());
    }
}

#[test]
fn serializes_flat_camel_case_for_the_ui() {
    let v = serde_json::to_value(PricingInfo::from_ids(None, None)).unwrap();
    assert_eq!(v["coreCents"], 3999);
    assert_eq!(v["plaidInterval"], "month");
    assert!(v["corePriceId"].is_null());
}
