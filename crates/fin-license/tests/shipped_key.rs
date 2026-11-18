//! The public key committed for the app must parse, or activation would silently stay off.
use fin_license::parse_public_key;

#[test]
fn the_committed_public_key_is_a_valid_ed25519_key() {
    let text = include_str!("../../../apps/desktop/src-tauri/license_public_key.txt");
    assert!(
        parse_public_key(text).is_ok(),
        "license_public_key.txt must hold the base64url public key"
    );
}
