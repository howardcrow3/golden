//! Trial and licence logic: a 14-day trial, a one-time licence per major version, and an
//! optional yearly Plaid add-on. Licences are Ed25519-signed tokens checked offline, so
//! Private mode never needs to phone home.
use base64::engine::general_purpose::URL_SAFE_NO_PAD as B64;
use base64::Engine;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier};
use fin_core::NaiveDate;
use serde::{Deserialize, Serialize};

pub mod link;
pub mod paddle;
pub mod pricing;

pub use ed25519_dalek::VerifyingKey;

pub const TRIAL_DAYS: i64 = 14;
/// The major version of the app this build belongs to; a licence covers one major version.
pub const APP_MAJOR: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LicensePayload {
    pub product: String,
    pub major: u32,
    pub issued: NaiveDate,
    /// Last day the Plaid add-on is paid through, if the buyer chose Plaid.
    pub plaid_until: Option<NaiveDate>,
    /// Opaque buyer reference (not an email), for support lookups.
    pub buyer: String,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum LicenseError {
    #[error("licence is malformed")]
    Malformed,
    #[error("licence signature is not valid")]
    BadSignature,
    #[error("licence is for a different product")]
    WrongProduct,
    #[error("licence covers version {0}, this is version {1}")]
    WrongMajor(u32, u32),
}

/// Parse a base64 public key (as printed by `golden-license keygen`).
pub fn parse_public_key(b64: &str) -> Result<VerifyingKey, LicenseError> {
    let bytes: [u8; 32] = B64
        .decode(b64.trim())
        .map_err(|_| LicenseError::Malformed)?
        .try_into()
        .map_err(|_| LicenseError::Malformed)?;
    VerifyingKey::from_bytes(&bytes).map_err(|_| LicenseError::Malformed)
}

/// Parse a base64 secret key (seller side only).
pub fn parse_signing_key(b64: &str) -> Result<SigningKey, LicenseError> {
    let bytes: [u8; 32] = B64
        .decode(b64.trim())
        .map_err(|_| LicenseError::Malformed)?
        .try_into()
        .map_err(|_| LicenseError::Malformed)?;
    Ok(SigningKey::from_bytes(&bytes))
}

pub fn encode_key(bytes: &[u8; 32]) -> String {
    B64.encode(bytes)
}

/// Create a signed licence token (used by the store backend and tests, never shipped in the app).
pub fn issue(key: &SigningKey, payload: &LicensePayload) -> String {
    let body = serde_json::to_vec(payload).expect("serializable");
    let sig = key.sign(&body);
    format!("{}.{}", B64.encode(&body), B64.encode(sig.to_bytes()))
}

/// Verify a token against the app's public key and this build's major version.
pub fn verify(
    public: &VerifyingKey,
    token: &str,
    major: u32,
) -> Result<LicensePayload, LicenseError> {
    let (body_b64, sig_b64) = token
        .trim()
        .split_once('.')
        .ok_or(LicenseError::Malformed)?;
    let body = B64.decode(body_b64).map_err(|_| LicenseError::Malformed)?;
    let sig_bytes: [u8; 64] = B64
        .decode(sig_b64)
        .map_err(|_| LicenseError::Malformed)?
        .try_into()
        .map_err(|_| LicenseError::Malformed)?;
    public
        .verify(&body, &Signature::from_bytes(&sig_bytes))
        .map_err(|_| LicenseError::BadSignature)?;
    let p: LicensePayload = serde_json::from_slice(&body).map_err(|_| LicenseError::Malformed)?;
    if p.product != "golden" {
        return Err(LicenseError::WrongProduct);
    }
    if p.major != major {
        return Err(LicenseError::WrongMajor(p.major, major));
    }
    Ok(p)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Access {
    /// Full use; `days_left` counts down during the trial.
    Trial {
        days_left: i64,
    },
    Licensed,
    /// Trial over and no valid licence: read-only, with export always allowed.
    Expired,
}

impl Access {
    pub fn can_edit(&self) -> bool {
        !matches!(self, Access::Expired)
    }
    /// Your data is yours: export works in every state.
    pub fn can_export(&self) -> bool {
        true
    }
}

/// Decide what the person may do today. `first_run` is when the trial started.
pub fn access(first_run: NaiveDate, today: NaiveDate, license: Option<&LicensePayload>) -> Access {
    if license.is_some() {
        return Access::Licensed;
    }
    let used = (today - first_run).num_days().max(0);
    if used < TRIAL_DAYS {
        Access::Trial {
            days_left: TRIAL_DAYS - used,
        }
    } else {
        Access::Expired
    }
}

/// Whether the optional bank-connection add-on is active today.
pub fn plaid_active(license: Option<&LicensePayload>, today: NaiveDate) -> bool {
    license
        .and_then(|l| l.plaid_until)
        .is_some_and(|until| today <= until)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::OsRng;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }
    fn payload(major: u32, plaid: Option<NaiveDate>) -> LicensePayload {
        LicensePayload {
            product: "golden".into(),
            major,
            issued: d(2026, 11, 20),
            plaid_until: plaid,
            buyer: "b-123".into(),
        }
    }
    fn keys() -> (SigningKey, VerifyingKey) {
        let sk = SigningKey::generate(&mut OsRng);
        let vk = sk.verifying_key();
        (sk, vk)
    }

    #[test]
    fn keys_round_trip_through_text() {
        let sk = SigningKey::generate(&mut OsRng);
        let pk_text = encode_key(&sk.verifying_key().to_bytes());
        let sk_text = encode_key(&sk.to_bytes());
        assert_eq!(parse_public_key(&pk_text).unwrap(), sk.verifying_key());
        assert_eq!(
            parse_signing_key(&sk_text).unwrap().to_bytes(),
            sk.to_bytes()
        );
        assert!(parse_public_key("nope").is_err());
        assert!(parse_public_key("").is_err());
    }

    #[test]
    fn issued_license_verifies() {
        let (sk, vk) = keys();
        let tok = issue(&sk, &payload(1, None));
        assert_eq!(verify(&vk, &tok, 1).unwrap(), payload(1, None));
        assert_eq!(
            verify(&vk, &format!("  {tok}\n"), 1).unwrap().buyer,
            "b-123"
        );
    }

    #[test]
    fn tampering_is_detected() {
        let (sk, vk) = keys();
        let tok = issue(&sk, &payload(1, None));
        let other = issue(&sk, &payload(1, Some(d(2099, 1, 1))));
        let forged = format!(
            "{}.{}",
            other.split('.').next().unwrap(),
            tok.split('.').nth(1).unwrap()
        );
        assert_eq!(verify(&vk, &forged, 1), Err(LicenseError::BadSignature));
    }

    #[test]
    fn someone_elses_key_is_rejected() {
        let (sk, _) = keys();
        let (_, other_vk) = keys();
        assert_eq!(
            verify(&other_vk, &issue(&sk, &payload(1, None)), 1),
            Err(LicenseError::BadSignature)
        );
    }

    #[test]
    fn garbage_is_malformed() {
        let (_, vk) = keys();
        for t in ["", "abc", "a.b", "!!.!!"] {
            assert_eq!(verify(&vk, t, 1), Err(LicenseError::Malformed), "{t}");
        }
    }

    #[test]
    fn licence_is_per_major_version() {
        let (sk, vk) = keys();
        assert_eq!(
            verify(&vk, &issue(&sk, &payload(1, None)), 2),
            Err(LicenseError::WrongMajor(1, 2))
        );
    }

    #[test]
    fn other_products_are_rejected() {
        let (sk, vk) = keys();
        let mut p = payload(1, None);
        p.product = "ceevee".into();
        assert_eq!(
            verify(&vk, &issue(&sk, &p), 1),
            Err(LicenseError::WrongProduct)
        );
    }

    #[test]
    fn trial_counts_down_then_expires() {
        let start = d(2026, 11, 15);
        assert_eq!(access(start, start, None), Access::Trial { days_left: 14 });
        assert_eq!(
            access(start, d(2026, 11, 28), None),
            Access::Trial { days_left: 1 }
        );
        assert_eq!(access(start, d(2026, 11, 29), None), Access::Expired);
        assert_eq!(access(start, d(2030, 1, 1), None), Access::Expired);
    }

    #[test]
    fn clock_set_backwards_does_not_extend_the_trial_past_14_days() {
        let start = d(2026, 11, 15);
        assert_eq!(
            access(start, d(2026, 11, 10), None),
            Access::Trial { days_left: 14 }
        );
    }

    #[test]
    fn licence_always_grants_access() {
        let p = payload(1, None);
        assert_eq!(
            access(d(2020, 1, 1), d(2030, 1, 1), Some(&p)),
            Access::Licensed
        );
    }

    #[test]
    fn expired_is_read_only_but_exportable() {
        let a = access(d(2026, 1, 1), d(2026, 12, 1), None);
        assert!(!a.can_edit());
        assert!(a.can_export());
        assert!(Access::Trial { days_left: 3 }.can_edit());
    }

    #[test]
    fn plaid_addon_is_independent_of_the_core_licence() {
        let none = payload(1, None);
        let paid = payload(1, Some(d(2027, 11, 20)));
        assert!(!plaid_active(None, d(2026, 12, 1)));
        assert!(!plaid_active(Some(&none), d(2026, 12, 1)));
        assert!(plaid_active(Some(&paid), d(2027, 11, 20)));
        assert!(!plaid_active(Some(&paid), d(2027, 11, 21)));
        assert_eq!(
            access(d(2020, 1, 1), d(2028, 1, 1), Some(&paid)),
            Access::Licensed
        ); // lapsed Plaid never locks the app
    }
}
