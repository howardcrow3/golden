//! Seller-side tool. Never shipped in the app.
//!   golden-license keygen
//!   golden-license issue <secret-key-b64> <buyer-ref> <issued YYYY-MM-DD> [plaid-until YYYY-MM-DD]
use fin_core::NaiveDate;
use fin_license::{encode_key, issue, parse_signing_key, LicensePayload, APP_MAJOR};
use rand::rngs::OsRng;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("keygen") => {
            let sk = ed25519_dalek::SigningKey::generate(&mut OsRng);
            println!(
                "SECRET (keep private, never commit): {}",
                encode_key(&sk.to_bytes())
            );
            println!(
                "PUBLIC (goes in apps/desktop/src-tauri/license_public_key.txt): {}",
                encode_key(&sk.verifying_key().to_bytes())
            );
        }
        Some("issue") if args.len() >= 4 => {
            let key = parse_signing_key(&args[1]).expect("secret key");
            let day =
                |s: &str| NaiveDate::parse_from_str(s, "%Y-%m-%d").expect("date as YYYY-MM-DD");
            let payload = LicensePayload {
                product: "golden".into(),
                major: APP_MAJOR,
                issued: day(&args[3]),
                plaid_until: args.get(4).map(|s| day(s)),
                buyer: args[2].clone(),
            };
            println!("{}", issue(&key, &payload));
        }
        _ => {
            eprintln!(
                "usage: golden-license keygen | issue <secret> <buyer> <issued> [plaid-until]"
            );
            std::process::exit(2);
        }
    }
}
