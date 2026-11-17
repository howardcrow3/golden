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

`.github/workflows/app.yml` already signs and notarizes the macOS build on pushes to `main` when the secrets below exist. With none set, it builds unsigned as before. Pull-request builds are never signed.

Needs an Apple Developer Program membership ($99/year).

1. In Xcode (Settings > Accounts > Manage Certificates) or at developer.apple.com, create a **Developer ID Application** certificate. In Keychain Access, export it with its private key as `cert.p12` and choose a password.
2. Base64-encode it: `base64 -i cert.p12 | pbcopy`.
3. In the repository, open Settings > Secrets and variables > Actions and add:
   - `APPLE_CERTIFICATE`: the base64 text.
   - `APPLE_CERTIFICATE_PASSWORD`: the `.p12` password.
   - `APPLE_SIGNING_IDENTITY` (optional): for example `Developer ID Application: Your Name (TEAMID)`. Tauri finds it from the certificate if omitted.
4. For notarization, add **one** of:
   - Apple ID: `APPLE_ID` (your Apple ID email), `APPLE_PASSWORD` (an app-specific password from appleid.apple.com), `APPLE_TEAM_ID`.
   - App Store Connect API key: `APPLE_API_ISSUER`, `APPLE_API_KEY` (key ID), `APPLE_API_KEY_P8` (the full text of the `.p8` file).
5. Merge anything to `main`. The macOS job's "Configure Apple signing and notarization" step prints which mode it used. Verify the download with `spctl -a -vv Golden.app` and `xcrun stapler validate Golden.dmg`.

Never paste these values into chat or commit them.

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

## 6. Checkout (shared by all Turtlebox apps)

- `site/checkout/` is a static page that opens Paddle's overlay checkout for any price: `https://<site>/checkout/?price=pri_...&app=<app-slug>`. It uses Paddle's public client-side token (in `checkout.js`) and passes the app slug to Paddle as custom data so the licence service knows which app a sale is for. Other Turtlebox apps just link to it with their own price ID and slug.
- `.github/workflows/pages.yml` publishes `site/` to GitHub Pages. One-time: repository Settings > Pages > Source = GitHub Actions.
- In Paddle, add the Pages domain (for example `howardcrow3.github.io`) to approved domains for checkout.
- Set the repository **variable** `GOLDEN_CHECKOUT_URL` to the page address (for example `https://howardcrow3.github.io/golden/checkout/`). Until it is set, the app's Buy buttons stay off.
- `SALES_OPEN` in `site/checkout/checkout.js` is `false`: the page refuses to open checkout, so nobody can pay before keys can be delivered. Flip it to `true` in the same change that deploys the licence service.
- Still to build: the licence service that receives Paddle's webhook (`fin-license::paddle` already verifies it and issues the key), emails the buyer, and needs the licence signing key and Paddle webhook secret as its own secrets.

## Checklist

- [ ] Licence keypair generated, secret stored offline, public key committed
- [ ] Apple secrets added and notarization verified on a nightly build
- [ ] Windows signing configured and verified
- [ ] Tagged release workflow added
- [ ] Pages enabled, domain approved in Paddle, `GOLDEN_CHECKOUT_URL` set
- [ ] Licence service (webhook + email) deployed, then `SALES_OPEN = true`
