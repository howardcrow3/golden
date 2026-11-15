# Architecture

A Tauri 2 desktop app with a Rust core, the same shape as CeeVee, so the AI runtime and agent harness can be shared rather than rebuilt.

## At a glance

&#91;embedded content: components and what calls what\]

The UI and the MCP server both go through the agent harness, which is the only path from any model to the ledger. The four feature crates use the local model for reading and labelling and keep everything in one encrypted file. The dashed box is the only traffic that can leave the machine, and all of it is off until you turn it on.

## Stack and repository layout

The recommendation is to lift CeeVee's generic crates into a shared `turtlebox-*` workspace first, then build Golden on them. Copying them would leave two harnesses to fix every time the grammar or model code changes.

**Stack (same as CeeVee):** Tauri 2, Rust core on Tokio, SvelteKit with the static adapter and Tailwind fed by the Turtlebox `tailwind.preset.js`, TypeScript bindings from Rust types with `tauri-specta`, SQLite through `sqlx`, llama.cpp through `llama-cpp-2`. Targets macOS 13+ arm64 and Windows 11 x64, signed and notarized the same way.

```
turtlebox-core/                # shared with CeeVee, extracted first
├─ turtlebox-ai/               # from ceevee-ai: llama.cpp worker, model catalog and download, grammars, tasks, eval
├─ turtlebox-agent/            # from ceevee-agent: loop, tool registry, risk levels, policy, approvals, memory, scheduler
└─ turtlebox-mcp/              # rmcp client and server wrapper over the tool registry

finance-app/
├─ apps/desktop/               # Tauri app: src-tauri + ui (SvelteKit)
├─ crates/
│  ├─ fin-core/                # domain types, money (i64 cents), dates, event bus, job runner
│  ├─ fin-db/                  # SQLCipher schema, migrations, repositories, FTS5, sqlite-vec
│  ├─ fin-ingest/              # file watcher, readers (PDF, OCR, CSV, OFX), layouts, reconciliation
│  ├─ fin-ledger/              # chart of accounts, posting rules, categorization, transfers, close, reports
│  ├─ fin-plan/                # projection engine, Monte Carlo, tax tables, Social Security, RMDs
│  ├─ fin-invest/              # holdings, fund table, allocation, glide path, rebalancing
│  ├─ fin-prices/              # optional price lookups by ticker
│  └─ fin-tools/               # registers every feature's tools with turtlebox-agent
├─ prompts/                    # versioned prompt + schema per AI task
├─ evals/                      # task evals and fixtures (anonymized statements)
└─ docs/
```

Crates depend downward only, and feature crates talk through the event bus, as in CeeVee. `fin-plan` and `fin-invest` are pure Rust with no I/O, so they test fast and could later compile to WebAssembly for a phone companion.

## AI layer and agent harness

The AI layer is CeeVee's, unchanged in shape: one llama.cpp context on a dedicated worker thread with a priority queue, grammar-constrained output for every task, and one `LlmProvider` trait for optional hosted models.

**Runtime.** llama.cpp with Metal on Apple silicon and Vulkan on Windows, CPU fallback. The model loads when an agent needs it and unloads after 10 idle minutes. Interactive work (Assistant, a document the person is watching) jumps ahead of background work (categorizing, close).

**Models** (from the shared catalog, SHA-256 verified, downloaded on first run):

| Use | Default | Size | License | Notes |
| --- | --- | --- | --- | --- |
| Extraction, categorizing, chat | Qwen3 4B Instruct, Q4\_K\_M | about 2.5 GB | Apache-2.0 | 8 to 15 GB RAM |
| Same, larger | Qwen3 8B Instruct, Q4\_K\_M | about 5 GB | Apache-2.0 | Offered at 16 GB RAM and up; recommended for extraction |
| Embeddings | bge-small-en-v1.5, Q8 | about 37 MB | MIT | Merchant memory, similar transactions, search |
| OCR | `ocrs` (Rust) | about 10 MB | MIT / Apache-2.0 | Scanned pages to text with positions |
| Hard scans (P1) | A small vision-language model on llama.cpp's multimodal path | 4 to 8 GB | Apache-2.0 required | Chosen by eval; license checked before it ships |

**Providers.** Ollama, LM Studio, any OpenAI-compatible endpoint, and Anthropic, as in CeeVee, all off by default. A hosted provider runs in **redaction mode**: merchant names and amounts in the prompt are replaced with tokens, and the reply is mapped back locally. Extraction never goes to a hosted model unless the person turns that on for one document.

**Harness.** `turtlebox-agent` gives each agent the same loop: build a compact prompt from the agent's system prompt, working state and latest observation; generate under a grammar that allows only an allowed tool call or a final answer; check the call against the policy; run the tool in Rust with a timeout; record the step; repeat to the step limit. Golden adds:

- **Number guard** in place of CeeVee's fact guard (Specification, AI tasks).
- **No act level.** Policy modes are only `off`, `ask` and `auto` for propose-level actions (rules, new accounts, locking a month). Nothing in the registry moves money, so there is nothing to automate at act level.
- **Ledger-backed working state.** Tools return IDs and summaries, never whole statements, so a run stays inside a 4B model's context.
- **Eval suite** per task on anonymized fixture statements from 15 institutions, gating each release on extraction accuracy, reconciliation rate and categorization accuracy.

## Chat, steering and what-ifs

Chat runs on the same harness as every other agent; what-ifs are a thin layer over the plan engine, so they cost nothing to throw away.

- **Routing.** Each message first goes through `classify-message` (question, what-if, change, instruction). Questions run the Assistant's free loop. What-ifs and changes run a short fixed workflow: `draft-overrides`, validate in code, run the engine, return a card. A 4B model handles this reliably because each step is narrow.
- **Scenario overlay.** The plan is a `HouseholdState`. A scenario is a parent pointer plus a list of typed overrides (`PickOption`, `AddOption`, `SetRetirement`, `SetClaimAge`, `AddEvent`, `RemoveEvent`, `ScaleSpending`, `SetAllocation`, `MoveState`). The engine resolves the chain into a state, runs it, and caches the result by input hash in `sim_run`. An open what-if is an unsaved scenario; saving gives it a name; "Make this the plan" folds its overrides into the base as one `assumption_change` batch.
- **Changes.** `assumption.propose` returns a card; Apply writes the new value and a history row in one transaction and re-runs the plan. Undo writes the reverse change.
- **Instructions.** Stored in `instruction`, rendered into each agent's system prompt under "The person's standing instructions", after the fixed rules, so they can steer tone, caution and categorization but cannot lift a policy or turn off the number guard. Rule-like instructions become rules.
- **Redirecting a run.** A message sent while an agent runs is queued to that `AgentRun` as an observation marked as from the person; the loop reads it at the next step. "Stop" cancels the run through the job runner.
- **Live updates.** Engine results stream to the UI over Tauri events, so the chart shows the deterministic what-if at once and redraws when the simulation finishes.
- **Parameters with options.** `fin-plan` reads no bare numbers: every input is a `Parameter { key, options: Vec<ParamOption>, active, pinned }`. A pure `score_options` function ranks options from ledger statistics and bundled reference data; the active option resolves to a value (or a per-year series) when the `HouseholdState` is built. A what-if is a list of picks, so it can be stored, stacked and compared at no cost. The sensitivity view runs 2 × N deterministic projections (each parameter's low and high option) in parallel, well under a second for the 40 v1 parameters; uncertainty mode samples an option per Monte Carlo run by likelihood.
- **Lumpy income as dated events.** `fin-plan` produces a `Vec<CashEvent { date, stream, gross, withheld, kind: Dated | Spread }>` before anything is summed. Each `IncomeStream` has a `Cadence` (interval days, days of month, months of year, or irregular) fitted by `fin-ledger` from posted history, and a schedule generator that places future payments on dates. Supplemental withholding and the April true-up are events too. The Monte Carlo sums events into months, so its step stays monthly.
- **Any grain from one store.** Ledger lines and cash events carry a day. `fin-ledger` exposes `totals(range, grain, group_by)` where `Grain` is Week (ISO), Month, Quarter or Year, done as a SQL group-by on a precomputed `period_key` per grain, indexed. Budgets convert between grains by day count in `fin-core`, not in the UI. Nothing is stored per grain, so the switch is instant and never disagrees with itself.

## Plaid, licensing and distribution

**Plaid (v1.x).** Plaid needs a server to exchange the public token for an access token, and that access token must stay secret. Two ways to keep it off our servers as much as possible:

| Option | How | Trade-off |
| --- | --- | --- |
| Relay (recommended) | A small Turtlebox relay on AWS (the highturtle account CeeVee uses) does the token exchange and each sync call, encrypts results with the household's public key, and stores the access token encrypted under a key only the app holds | Data passes through the relay in transit, never stored readable; the relay also meters Plaid's per-account cost for the yearly Connected fee |
| Bring your own keys | The person uses their own Plaid developer account and the app talks to Plaid directly | Nothing passes through Turtlebox, but setup is too hard for most buyers |

The app keeps the encrypted token reference in the keychain; unlinking revokes it at Plaid and deletes it from the relay.

**Licensing and updates.** CeeVee's `ceevee-license` crate and highturtle endpoints, unchanged in shape: a 14-day trial from first run, then an Ed25519-signed licence per major version, activated on up to 2 computers, checked offline. Minor and patch updates of that version are included; a new major version is a new purchase, and the old one keeps working. Signed installers (NSIS on Windows with Azure Trusted Signing, notarized DMG on macOS arm64), Tauri updater channels with staged rollout, and signed model, tax-table and layout-template manifests, all as in CeeVee's release pipeline. License checks and update checks are the only calls Private mode makes besides model downloads.

## Ingestion pipeline and calculation engines

**Statement library.** `fin-ingest` gets a `library` module. Folder watching uses the `notify` crate (FSEvents on macOS, ReadDirectoryChangesW on Windows) with a settle delay so half-downloaded files are skipped. After a document is classified and extracted, a pure `file_name(doc_meta, template)` builds the name and `library_path(doc_meta)` the year and type folder; the move is a rename on the same volume and copy-then-delete across volumes, written to `library_file` first so a crash can be resumed or undone. The library root sits behind Tauri's file-system scope, so the app can touch only watched folders and the library. Moving the library is one background job with progress.

Ingestion is a job on CeeVee's job runner; the engines are plain Rust libraries called by tools and by the UI.

**Ingestion (`fin-ingest`).**

- `notify` watches the chosen folder; drag and drop sends files through the same path. Each file becomes a `Job` with the seven named steps from the Specification, resumable and cancellable.
- Readers: `pdf-extract` with `lopdf` fallback for text PDFs, keeping x and y positions so columns survive; `ocrs` for scans; `csv` and an OFX/QFX parser in code.
- Layout templates: after the first good import from an institution, the column positions and header patterns are saved as data. Later statements from it are parsed by template, and the model only classifies and checks. Templates ship as signed data updates, like CeeVee's ATS adapters.
- Reconciliation is pure code, in integer cents, and is the gate to posting.

**Ledger (`fin-ledger`).** Double-entry postings in SQLite with a check constraint that every entry sums to zero. Reports are SQL views over postings, cached per month and invalidated when a month changes. Closing a month writes a snapshot of every balance.

**Projection and simulation (`fin-plan`).**

- Deterministic monthly engine over a `HouseholdState` (accounts, people, events, assumptions). One step is a pure function, so a month can be tested alone.
- Monte Carlo: 2,000 runs on `rayon` across cores, block bootstrap of historical monthly returns and inflation bundled with the app as data, seeded so the same inputs give the same answer.
- Tax tables (federal brackets, FICA, IRS contribution limits, RMD divisors, IRMAA tiers, and a rules file per state and DC) and Social Security rules ship as signed, versioned data files, updated each January and when a state changes its law, without an app release.
- Results are cached in `sim_run` by a hash of the inputs, so moving a retirement month back to where it was is instant.

**Allocation (`fin-invest`).** A bundled fund table (symbol to asset-class mix and expense ratio) maps holdings to classes. Glide path, target and rebalancing are pure functions. Price lookups by ticker are the only network calls this crate makes, and they are optional.

## Security and privacy

Financial data raises the bar above CeeVee in three places: the database is encrypted, the app locks, and every byte that leaves the machine is listed.

| Area | Decision |
| --- | --- |
| Database | SQLCipher (BSD-style license) with AES-256; the key is random, held in the macOS Keychain or Windows Credential Manager, and released by Touch ID, Windows Hello or a passcode |
| Original documents | Stored in the app-data folder, encrypted with a key derived from the database key; the source folder is left untouched unless the person asks to move files in |
| App lock | Locks after 15 idle minutes and on sleep; background agents keep running with the key held in memory only while unlocked, otherwise they wait |
| Network | Private mode: model and update CDN and licence checks only. Connected mode adds what the person turns on: the Plaid relay, price lookups (tickers only), hosted providers, Gmail. Every one is listed on the Connections page. Nothing else. No telemetry; crash reports opt-in and scrubbed of amounts and names |
| Model prompts | Account numbers cut to last four and SSNs dropped before text reaches any model; hosted providers only in redaction mode |
| MCP server | Off by default; localhost only; per-client token and scope; every call logged; the Settings screen says plainly that a client's model may be hosted |
| Untrusted content | Document text is wrapped as data; a document cannot raise a policy mode or reach a propose-level tool without the person |
| Backups | Nightly encrypted local backup with 14-day retention; export is one encrypted file plus the originals |
| Delete | "Delete everything" removes the database, originals, backups, keychain entries and downloaded models in one step |
| Tauri | Capabilities limited to needed commands, strict CSP, no remote content in the webview, as in CeeVee |

## Testing, order of work and risks

**Testing.** CeeVee's setup plus three finance-specific suites.

- Reconciliation fixtures: anonymized real statements from 15 institutions, each with its expected lines; every release must reconcile all of them.
- Ledger property tests (`proptest`): every entry sums to zero, the balance sheet equals the sum of postings, and P&L savings equals the change in net worth less market movement.
- Engine golden tests: fixed households with hand-checked projections, and Monte Carlo runs with a fixed seed compared against stored results.
- AI evals per task, as in `ceevee-ai/evals/`, run against the real default model, gating releases on the PRD targets.

**Order of work (about 22 weeks).**

1. **Weeks 1 to 3, foundation.** Extract `turtlebox-ai`, `turtlebox-agent` and `turtlebox-mcp` from CeeVee and point CeeVee at them; Golden skeleton, shell, SQLCipher, job runner, tray mode, signing.
2. **Weeks 4 to 8, ingestion.** Readers, OCR, classify and extract tasks, layout templates, reconciliation, Documents screen with the split view. Import 24 months of the household's statements as the test bed.
3. **Weeks 9 to 12, books.** Chart of accounts, posting rules, paystub posting, categorization, transfers, Transactions, Cash flow and Balance sheet screens, monthly close.
4. **Weeks 13 to 18, planning.** Projection engine with federal and state tax, events, scenario overlays, Monte Carlo, Retirement and Plan screens, holdings and allocation, Investments screen.
5. **Weeks 19 to 22, assistant and release.** Assistant panel with what-if and change cards, standing instructions, number guard, MCP server and scopes, Private mode and Connections page, licensing and trial, eval gates, beta on Howard's machines, then paid launch. Plaid relay follows in v1.x.

**Risks.**

| Risk | Mitigation |
| --- | --- |
| A 4B model misreads statement tables | Positions kept from the PDF, layout templates per institution, reconciliation as a hard gate, 8B offered, hosted model per document as a last resort |
| Scanned statements read badly | OCR plus the reconciliation gate; show the page beside the lines so a fix takes seconds |
| Retirement answer trusted more than it should be | Show the range, not one number; assumptions panel on every result; back-test the projection on the household's own history |
| Allocation suggestions read as advice | Rules shown with every number, "suggested" wording, footer; legal review before any sale |
| Extracting the shared crates slows CeeVee | Do it in week 1 behind CeeVee's existing tests; no behaviour change |
| Plaid bills every month per linked account, but the app is a one-time purchase | Decided: choosing Plaid adds an annual subscription, sold as an upsell on top of the one-time price, metered at the relay; Private mode stays a one-time purchase |
| State tax rules change every year across 51 jurisdictions | Rules as signed data files with golden tests per state, updated outside app releases; a licensed tax-data source considered if upkeep runs high |
| MCP clients send data to hosted models | Off by default, Summary scope by default, plain warning, call log |
