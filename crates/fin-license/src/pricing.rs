//! What Golden costs, in one place. The amounts live in `data/pricing.json` so the UI copy,
//! the docs and the tests all read the same numbers; change them there. The Paddle price IDs
//! are supplied at build time (they are not in the repo).
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pricing {
    pub currency: String,
    /// One-time price of the core app, per major version, in cents.
    pub core_cents: i64,
    /// Price of the optional Plaid add-on per `plaid_interval`, in cents.
    pub plaid_cents: i64,
    /// "month" or "year".
    pub plaid_interval: String,
}

impl Pricing {
    pub fn standard() -> Pricing {
        serde_json::from_str(include_str!("../data/pricing.json")).expect("bundled pricing")
    }
}

/// Pricing plus the Paddle price IDs checkout needs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PricingInfo {
    #[serde(flatten)]
    pub pricing: Pricing,
    pub core_price_id: Option<String>,
    pub plaid_price_id: Option<String>,
    /// Hosted checkout links; None until a checkout page is configured, which keeps the Buy
    /// buttons off so nobody can pay before licences can be delivered.
    pub core_checkout_url: Option<String>,
    pub plaid_checkout_url: Option<String>,
    /// Set when a configured ID was rejected, in words a person can act on.
    pub warning: Option<String>,
}

/// A Paddle price ID looks like `pri_` plus lowercase letters and digits. A product ID (`pro_`)
/// is a different thing and cannot be bought, so it is refused with a pointer to the right one.
pub fn check_price_id(id: &str) -> Result<&str, String> {
    let id = id.trim();
    if let Some(rest) = id.strip_prefix("pri_") {
        if rest.len() >= 10
            && rest
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        {
            return Ok(id);
        }
        return Err(format!("{id:?} doesn't look like a Paddle price ID."));
    }
    if id.starts_with("pro_") {
        return Err(format!(
            "{id} is a Paddle product ID. Checkout needs the price ID (it starts with pri_): open the product in Paddle and copy the ID next to its price."
        ));
    }
    Err(format!(
        "{id:?} isn't a Paddle price ID (they start with pri_)."
    ))
}

/// Golden's store page on turtlebox.studio. Its Buy button opens Paddle's checkout, and the billing
/// service emails the licence after payment. The app opens this page when no other checkout page is set.
pub const STORE_URL: &str = "https://turtlebox.studio/golden/#buy";

/// The short name sent to the checkout page so one page can serve every Turtlebox app.
pub const APP_SLUG: &str = "golden";

/// Link to the hosted checkout page for one price. `base` must be an https URL; `app` is a short
/// lowercase slug the page passes to Paddle as custom data. Nothing else is accepted, so the
/// link can't carry anything the page doesn't expect.
pub fn checkout_url(base: &str, price_id: &str, app: &str) -> Result<String, String> {
    let base = base.trim();
    let rest = base
        .strip_prefix("https://")
        .ok_or("the checkout page address must start with https://")?;
    if rest.is_empty() || base.contains(['?', '#', ' ', '"', '<', '>']) {
        return Err("the checkout page address is not a plain https address".into());
    }
    let price = check_price_id(price_id)?;
    if app.is_empty()
        || app.len() > 32
        || !app
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    {
        return Err("the app name must be 1 to 32 lowercase letters, digits or dashes".into());
    }
    Ok(format!("{base}?price={price}&app={app}"))
}

impl PricingInfo {
    /// Combine the standard prices with whatever IDs the build was given.
    pub fn from_ids(
        core: Option<&str>,
        plaid: Option<&str>,
        checkout_base: Option<&str>,
    ) -> PricingInfo {
        let mut warnings = vec![];
        let mut take = |label: &str, raw: Option<&str>| -> Option<String> {
            let raw = raw.map(str::trim).filter(|s| !s.is_empty())?;
            match check_price_id(raw) {
                Ok(id) => Some(id.to_string()),
                Err(e) => {
                    warnings.push(format!("{label}: {e}"));
                    None
                }
            }
        };
        let core_price_id = take("Core price", core);
        let plaid_price_id = take("Plaid price", plaid);
        let base = checkout_base.map(str::trim).filter(|b| !b.is_empty());
        let mut url_for = |label: &str, id: &Option<String>| -> Option<String> {
            let (base, id) = (base?, id.as_deref()?);
            match checkout_url(base, id, APP_SLUG) {
                Ok(u) => Some(u),
                Err(e) => {
                    warnings.push(format!("{label}: {e}"));
                    None
                }
            }
        };
        let core_checkout_url = if base.is_some() {
            url_for("Core checkout", &core_price_id)
        } else {
            Some(STORE_URL.to_string())
        };
        let plaid_checkout_url = url_for("Plaid checkout", &plaid_price_id);
        PricingInfo {
            pricing: Pricing::standard(),
            core_price_id,
            plaid_price_id,
            core_checkout_url,
            plaid_checkout_url,
            warning: if warnings.is_empty() {
                None
            } else {
                Some(warnings.join(" "))
            },
        }
    }
}
