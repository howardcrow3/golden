# Golden

A private, local-first desktop app for household finance and retirement planning (macOS and Windows).

Plan documents are in [`docs/`](docs/README.md).

## Layout

| Path | What it is |
| --- | --- |
| `crates/fin-core` | Money in integer cents, week/month/quarter/year grains, people and age rules |
| `crates/fin-plan` | Federal and state tax, retirement projection, Monte Carlo, assumptions with options, what-if scenarios |
| `crates/fin-db` | SQLite ledger (optional SQLCipher encryption), idempotent imports, totals at any grain |
| `crates/fin-ingest` | CSV, OFX and text-PDF statement readers, statement library naming |
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

Working today: balance sheet with starting balances, paystub reading, monthly projection, budgets, backup/restore, savings allocation suggestions, a read-only assistant (MCP) server, statement import (CSV/OFX/text PDF, with a balance check against the closing balance the statement prints) with auto-filing into a statement library, cash flow at any grain, recategorize with remembered rules, CSV export, household and income streams, assumptions with options and likelihoods, Monte Carlo retirement plan, rule-based what-if chat, encrypted local database with the key in the OS keychain, trial and licence lock.

Known gaps:

- Five states (AR, MD, MN, NY, VT) are flagged approximate; see the notes in `crates/fin-plan/data/states_2026.json`.
- Licence activation is on: `apps/desktop/src-tauri/license_public_key.txt` holds the public key, and keys are issued by the highturtle billing Lambda on AWS; see [signing and licensing setup](docs/signing-and-licensing-setup.md).
- Installers are unsigned (no Apple notarization or Windows certificate yet); the same doc lists the steps.
- The built-in model (llama.cpp, downloaded in the app) drives the what-if chat, transaction categories, statement names, plain-English explanations and suggested assumptions, each with a rules fallback. It has only been exercised with fake engines so far; Windows runs on CPU.
- PDF statements work when the PDF has a text layer and the lines carry running balances or minus/CR markers; scanned PDFs need OCR, which is not built in yet, and a PDF whose layout cannot be read reliably is refused rather than guessed. Only generic layouts have been tried, not real bank PDFs.
