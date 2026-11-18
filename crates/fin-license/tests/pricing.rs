use fin_license::pricing::{check_price_id, checkout_url, Pricing, PricingInfo};

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
        None,
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
        None,
    );
    assert!(mixed.core_price_id.is_none());
    assert!(mixed.plaid_price_id.is_some());
    assert!(mixed.warning.unwrap().starts_with("Core price:"));
}

#[test]
fn missing_ids_are_not_an_error() {
    for none in [None, Some(""), Some("   ")] {
        let i = PricingInfo::from_ids(none, none, none);
        assert!(i.core_price_id.is_none() && i.plaid_price_id.is_none() && i.warning.is_none());
    }
}

#[test]
fn serializes_flat_camel_case_for_the_ui() {
    let v = serde_json::to_value(PricingInfo::from_ids(None, None, None)).unwrap();
    assert_eq!(v["coreCents"], 3999);
    assert_eq!(v["plaidInterval"], "month");
    assert!(v["corePriceId"].is_null());
    assert_eq!(v["coreCheckoutUrl"], "https://turtlebox.studio/golden/#buy");
    assert!(v["plaidCheckoutUrl"].is_null());
}

const BASE: &str = "https://turtlebox.example/checkout/";
const CORE: &str = "pri_01m7h6qqb0y4f9r6mftw05y772";
const PLAID: &str = "pri_01m7h71ez1v1pen447723fmeje";

#[test]
fn checkout_links_carry_only_the_price_and_app() {
    assert_eq!(
        checkout_url(BASE, CORE, "golden").unwrap(),
        "https://turtlebox.example/checkout/?price=pri_01m7h6qqb0y4f9r6mftw05y772&app=golden"
    );
}

#[test]
fn checkout_links_refuse_odd_input() {
    assert!(checkout_url("http://turtlebox.example/c/", CORE, "golden").is_err());
    assert!(checkout_url("https://", CORE, "golden").is_err());
    assert!(checkout_url("https://x.example/c?evil=1", CORE, "golden").is_err());
    assert!(checkout_url("https://x.example/c/#frag", CORE, "golden").is_err());
    assert!(checkout_url(BASE, "pro_01m7h6qqb0y4f9r6mftw05y772", "golden").is_err());
    assert!(checkout_url(BASE, CORE, "Golden App").is_err());
    assert!(checkout_url(BASE, CORE, "").is_err());
    assert!(checkout_url(BASE, CORE, &"a".repeat(33)).is_err());
}

#[test]
fn without_a_checkout_page_buy_opens_the_store_page() {
    use fin_license::pricing::STORE_URL;
    assert_eq!(STORE_URL, "https://turtlebox.studio/golden/#buy");
    // Core always has a place to buy; the add-on has no page yet, so no link.
    for base in [None, Some("  ")] {
        let i = PricingInfo::from_ids(Some(CORE), Some(PLAID), base);
        assert_eq!(i.core_checkout_url.as_deref(), Some(STORE_URL));
        assert!(i.plaid_checkout_url.is_none());
    }
    // A build with no price IDs at all still opens the store.
    assert_eq!(
        PricingInfo::from_ids(None, None, None)
            .core_checkout_url
            .as_deref(),
        Some(STORE_URL)
    );
}

#[test]
fn with_a_checkout_page_both_links_appear_and_a_missing_id_has_none() {
    let i = PricingInfo::from_ids(Some(CORE), Some(PLAID), Some(BASE));
    assert!(i.core_checkout_url.unwrap().contains(CORE));
    assert!(i.plaid_checkout_url.unwrap().contains(PLAID));
    let i = PricingInfo::from_ids(Some(CORE), None, Some(BASE));
    assert!(i.core_checkout_url.is_some() && i.plaid_checkout_url.is_none());
}

#[test]
fn a_bad_checkout_address_is_reported() {
    let i = PricingInfo::from_ids(Some(CORE), None, Some("http://insecure.example/"));
    assert!(i.core_checkout_url.is_none());
    assert!(i.warning.unwrap().contains("https://"));
}
