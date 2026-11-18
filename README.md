# Golden

A private, local-first desktop app for household finance and retirement planning (macOS and Windows).

Plan documents are in [`docs/`](docs/README.md).

## Layout

| Path | What it is |
| --- | --- |
| `crates/fin-core` | Money in integer cents, week/month/quarter/year grains, people and age rules |
| `crates/fin-plan` | Federal and state tax, retirement projection, Monte Carlo, assumptions with options, what-if scenarios |
| `crates/fin-db` | SQLite ledger (optional SQLCipher encryption), idempotent imports, totals at any grain |
| `crates/fin-ingest` | CSV and OFX statement readers, statement library naming |
| `crates/fin-license` | 14-day trial and offline Ed25519-signed licences (plus Plaid add-on expiry) |
| `crates/fin-app` | Service layer behind the Tauri commands: import, filing, recategorize, export, household, what-if chat |
| `crates/fin-ledger` | Merchant normalization, categorization rules, transfer matching |
| `apps/desktop` | Tauri 2 shell and Svelte UI |

## Develop

```sh
cargo test --workspace            # all Rust tests (unit, property and integration)
cargo clippy --workspace --all-targets -- -D warnings
cd apps/desktop && npm ci && npm test && npm run tauri dev
```

## CI

- **CI** runs format, lint and every test on Linux, macOS and Windows for each pull request.
- **App** type-checks and tests the UI, builds the desktop app on macOS (Apple silicon) and Windows, and on every push to `main` publishes the installers to the rolling [`nightly`](../../releases/tag/nightly) release. Builds are unsigned until signing certificates are added.

## Status

Working today: balance sheet with starting balances, paystub reading, monthly projection, budgets, backup/restore, savings allocation suggestions, a read-only assistant (MCP) server, statement import (CSV/OFX) with auto-filing into a statement library, cash flow at any grain, recategorize with remembered rules, CSV export, household and income streams, assumptions with options and likelihoods, Monte Carlo retirement plan, rule-based what-if chat, encrypted local database with the key in the OS keychain, trial and licence lock.

Known gaps:

- Six states (AR, CT, MD, MN, NY, VT) are flagged approximate; see the notes in `crates/fin-plan/data/states_2026.json`.
- Licence activation is on: `apps/desktop/src-tauri/license_public_key.txt` holds the public key, and keys are issued by the highturtle billing Lambda on AWS; see [signing and licensing setup](docs/signing-and-licensing-setup.md).
- Installers are unsigned (no Apple notarization or Windows certificate yet); the same doc lists the steps.
- The what-if chat is rule-based by default. A language-model interpreter exists (`fin-app/src/model.rs`: validated answers only, no balances or birth dates sent, falls back to the rules) but no model is wired in yet; a `Completer` for a local model or CeeVee is the remaining piece.
