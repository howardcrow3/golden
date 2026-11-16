# Signing and licence setup (one-time, needs a person)

CI builds installers on every merge to `main`, but two things need accounts and secrets that only a person can create. Until they are done, nightly builds are unsigned and licence activation is off.

## 1. Licence keys (about 2 minutes)

```sh
cargo run -p fin-license --features tool --bin golden-license -- keygen
```

- Keep the **secret** key offline (password manager or hardware key). Anyone holding it can mint licences.
- Put the **public** key in `apps/desktop/src-tauri/license_public_key.txt` and commit it. The app then accepts licences signed with the secret key.
- Issue a licence for a customer with `golden-license issue` (see `--help`). Licences are verified offline, so no server is needed.
- Losing the secret key means no new licences can be issued for that public key; shipping a new public key invalidates old licences, so back it up.

## 2. macOS signing and notarization

Needs an Apple Developer Program membership.

1. Create a "Developer ID Application" certificate and export it as `.p12`.
2. Base64-encode it: `base64 -i cert.p12 | pbcopy`.
3. Add these GitHub Actions secrets: `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`, `APPLE_SIGNING_IDENTITY` (for example `Developer ID Application: Name (TEAMID)`), `APPLE_ID`, `APPLE_PASSWORD` (an app-specific password), `APPLE_TEAM_ID`.
4. Pass them as `env` to the `tauri-action` step in `.github/workflows/app.yml`. Tauri signs and notarizes when they are present.

## 3. Windows signing

Pick one:

- **Azure Trusted Signing** (cheapest, no hardware token): configure it and set `bundle.windows.signCommand` in `tauri.conf.json`.
- **EV or OV code-signing certificate**: export a `.pfx`, store it and its password as secrets, import it in the workflow, and set `bundle.windows.certificateThumbprint`.

An unsigned installer shows a SmartScreen warning; an EV certificate removes it immediately, others build reputation over time.

## 4. Release channel

Today every merge refreshes the rolling `nightly` release. Before selling, add a tagged `v1.x` release job that only runs on version tags, so customers never receive a nightly.

## 5. Prices and Paddle

- Prices: `crates/fin-license/data/pricing.json` (core one-time and Plaid per month, in cents). The Licence screen and tests read it, so change it there only. Currently $39.99 and $5 a month.
- Paddle price IDs are repository secrets `GOLDEN_PRICE_ID` (core) and `GOLDEN_PLAID_PRICE_ID` (Plaid). CI passes them into the app build. They must be **price** IDs (`pri_...`), not product IDs (`pro_...`); the app refuses a product ID and says so on the Licence screen.
- Changing a price in Paddle creates a new price ID: update the secret too, or checkout would charge the old amount.

## Checklist

- [ ] Licence keypair generated, secret stored offline, public key committed
- [ ] Apple secrets added and notarization verified on a nightly build
- [ ] Windows signing configured and verified
- [ ] Tagged release workflow added
- [ ] Paddle checkout and licence-issuing webhook deployed
