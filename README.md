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
