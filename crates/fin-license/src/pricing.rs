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

impl PricingInfo {
    /// Combine the standard prices with whatever IDs the build was given.
    pub fn from_ids(core: Option<&str>, plaid: Option<&str>) -> PricingInfo {
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
        PricingInfo {
            pricing: Pricing::standard(),
            core_price_id,
            plaid_price_id,
            warning: if warnings.is_empty() {
                None
            } else {
                Some(warnings.join(" "))
            },
        }
    }
}
