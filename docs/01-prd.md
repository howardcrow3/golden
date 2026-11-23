# Golden: PRD, spec, design and architecture

Nov 15, 2026 · @Howard

A local AI app that reads your bank statements, credit card statements and paystubs, keeps your household P&L and balance sheet, projects your position month by month, and tells you when you and your spouse can retire. It runs on your own machine on the CeeVee agent harness, with local models by default, and looks like the High Turtle design system.

## Goal and principles

The goal is one honest answer to "when can we retire, and what do we change to get there", kept current from the documents the household already receives. Today that answer lives in spreadsheets that go stale the month after they are built.

**Who it is for.** Households in the US who want to plan retirement from their own records, often two earners. Every account, income line and retirement choice belongs to one person or to both. It is a Turtlebox product sold for Mac and Windows, licensed per household, with Howard's household as the first beta.

**Principles**

- **Local first.** Statements, the ledger, the plan and all AI processing stay on the machine. Nothing is sent to us. Hosted models and bank connections are optional and off by default.
- **Code does the math, the model reads and explains.** The model extracts transactions from documents, labels them and writes explanations. Every balance, total, projection and allocation is computed by deterministic code from the ledger. A number the model writes must trace to a ledger query, or it is flagged.
- **Nothing is trusted until it reconciles.** An imported statement is posted only when its opening balance plus its transactions equals its closing balance to the cent.
- **The person approves changes to the books.** New categorization rules, merged accounts and edits to past months are proposals the person accepts. The same approvals queue as CeeVee.
- **Same harness and delivery as CeeVee.** Licensing, trial, signed installers, updates, the local model runtime, agent loop, tool registry, policy engine and approvals come from CeeVee's crates and release pipeline, shared rather than copied.
- **Education, not advice.** Allocation suggestions are rules-based and explained. The app never places trades or moves money.

**Talk to it.** The chat is how you steer the app. You can change an assumption, correct how a transaction was read, give a standing instruction or ask "what if" in plain words, and see the effect on the plan before anything changes.

**Private by choice.** Privacy is the selling point, so it is a mode the person picks and can see, not a buried setting.

|  | Private mode (default) | Connected mode |
| --- | --- | --- |
| How data gets in | File uploads and the watched folder | Files, plus Plaid for the accounts the person links |
| What leaves the computer | Update checks and model downloads only | Plaid traffic for linked accounts, plus anything else the person turns on |
| AI | Local models only | Local by default; hosted models optional, in redaction mode |
| Price lookups, Gmail, MCP | Off; each can be turned on and is listed | Same |
| Shown in the app | A "Private" status in the sidebar foot, and a page listing every connection, which is empty | The same page listing each connection and what it sends |

Switching to Connected mode names the third parties involved before anything is linked. Switching back disconnects Plaid and deletes its tokens.

## Scope

v1 covers the full loop from documents to a retirement date, for US households only. Bank connections, tax filing and trading stay out.

| Area | v1 | Later (v1.x) | Out |
| --- | --- | --- | --- |
| Ingestion | PDF statements (text and scanned), CSV, OFX and QFX exports, paystub PDFs, a watched folder, drag and drop | Plaid connections, opt-in per account, outside Private mode; statements pulled from Gmail through the CeeVee Google connector | Screen-scraping bank sites with stored passwords |
| Books | Double-entry ledger, AI categorization with rules, transfer and duplicate detection, monthly close, P&L and balance sheet per person and household | Receipt matching, split transactions from email receipts | Business bookkeeping, invoicing |
| Accounts | Checking, savings, credit cards, brokerage, 401(k), IRA, Roth, HSA, mortgage, auto and student loans, home and car values entered by hand | Equity compensation (RSU, ESPP), crypto wallets by address | Bill pay |
| Projection | Monthly forecast 1 to 40 years out from trailing history, recurring items and planned events; what-ifs from chat or from the controls, compared against the plan | Seasonality learned per category beyond 24 months |  |
| Retirement | Federal and state income tax for all 50 states and DC, retirement month per person, Social Security claiming age per person, pensions, Monte Carlo success rate, earliest safe date, spending in retirement | Roth conversion and withdrawal-order optimizer | Estate planning |
| Investments | Current allocation across all accounts, target allocation from a glide path and risk answers, rebalancing suggestions, asset location hints | Tax-loss harvesting hints | Placing trades, individual stock picks |
| AI | Local models on the CeeVee harness, Assistant chat that answers, changes assumptions and runs what-ifs, background agents, approvals queue, activity log | Hosted models with a redaction mode |  |
| MCP | Local MCP server with read tools and per-client scopes | Draft tools (notes, scenarios) for MCP clients | Any tool that moves money |
| Platforms | macOS 13+ on Apple silicon, Windows 11 x64 (CeeVee targets) | Linux, if asked for | Web app, phone app |

## Requirements

P0 ships in v1, P1 ships in v1 if time allows, P2 is v1.x. IDs are referenced from the Specification tab.

| ID | As a household member, I want to | Requirement | Priority |
| --- | --- | --- | --- |
| IN-1 | drop a month of statements in and be done | Accept PDF, CSV, OFX and QFX by drag and drop or a watched folder; detect institution, account and period; import without setup per bank | P0 |
| IN-2 | trust that nothing was missed | Post a statement only when opening balance + transactions = closing balance; show unreconciled statements with the gap and the lines in doubt | P0 |
| IN-3 | import scanned or photographed statements | OCR the page, then extract; the same reconciliation check applies | P1 |
| IN-4 | have paystubs split correctly | Extract gross pay, each tax, pre-tax and post-tax deductions, employer match, and net pay; post net pay against the matching deposit | P0 |
| IN-5 | never import the same thing twice | Detect duplicate files and overlapping statement periods; merge, never double-count | P0 |
| IN-6 | skip the monthly downloads if I am willing to connect | Link accounts through Plaid in Connected mode; transactions, balances and holdings sync daily into the same ledger, still checked against statements when they arrive; unlink and delete tokens in one step | P2 |
| IN-7 | stop sorting and renaming statement files myself | The app picks up statements from Downloads or any folder I choose, reads each one, renames it to a clear name ("2026-10 Chase Checking 4821 statement.pdf") and files it in my statement library by year and statement type. The library folder has a default and I can move it. Every move can be undone | P0 |
| BK-1 | see where money goes without tagging every line | AI categorizes each transaction with a confidence; low-confidence lines go to a review list; accepted corrections become proposed rules | P0 |
| BK-2 | not count transfers and card payments as spending | Pair transfers between own accounts and card payments to card statements automatically | P0 |
| BK-3 | see a P&L each month | Income, taxes, spending by category, savings, savings rate; per person and household; month, quarter, year, trailing 12 | P0 |
| BK-4 | see a balance sheet | Assets and liabilities by account and by type (cash, taxable, tax-deferred, tax-free, property, debt); net worth over time | P0 |
| BK-5 | close a month | A close step checks every account has a reconciled statement, lists anomalies, writes a short month summary, then locks the month | P1 |
| BK-6 | look at money by week, month, quarter or year | One Week / Month / Quarter / Year switch in the top bar of every money screen (cash flow, budgets, transactions, projections), remembered per screen; budgets can be set in any of those units and are shown in whichever one is selected | P0 |
| PR-1 | see where we will be each month | Monthly projection of cash, each account balance and net worth, from trailing history, recurring items and planned events | P0 |
| PR-2 | plan known changes | Add events: raise, bonus, new loan, payoff, home purchase, college, one-off costs; each with a date and amount | P0 |
| PR-3 | ask "what if" and see the answer next to our plan | Ask a what-if in chat ("what if I retire at 60 and we sell the house in 2035?") or change any control; the app runs it as a temporary scenario on top of the plan and shows the difference in success rate, retirement dates, net worth and monthly cash, without changing the plan | P0 |
| PR-4 | keep the what-ifs worth keeping | Save a what-if as a named scenario, stack what-ifs on each other, compare up to three scenarios side by side, and make one the plan in a single step with undo | P0 |
| PR-5 | see and change every number the projection rests on | Every projection parameter (raises, bonus, inflation, returns, spending growth, retirement spending, healthcare cost, tax rates, Social Security, home value growth and the rest) holds several values, each with its source and a likelihood. The app picks the most likely from the household's own history; the person can pick another, add their own, or ask in chat, and see the effect at once. A what-if is simply a different pick on one or more parameters | P0 |
| PR-6 | see lumpy income as it really arrives | Every income source is a stream with its own pattern: salary per pay date, bonuses in their month, RSU and option vests on their schedule, ESPP purchases, dividends by holding and pay date, interest, tax refunds, irregular side income. History tags each payment to its stream; projections place each stream on its own dates with its own tax, instead of an even monthly average | P0 |
| PR-7 | have every income I and my household get, at once | Any number of concurrent streams per person and jointly: several jobs, self-employment and 1099 work, rental properties, pensions and annuities, Social Security, investment income, trust distributions, royalties, alimony and others. Each starts and stops on its own dates (a job ending at retirement, a pension starting at 65), and each is projected and taxed on its own | P0 |
| PR-8 | have the age rules use our real birthdays | Date of birth is captured for each adult and dependent at setup. It drives Social Security (full retirement age, claiming at 62 to 70, spousal and survivor benefits), Medicare at 65, penalty-free withdrawals at 59½, catch-up contributions, required minimum distributions, and dependent credits, each on the exact month the person reaches the age | P0 |
| RT-1 | choose when each of us retires | A retirement month per person; Social Security claiming age per person; spending in retirement as today's spending or a set amount | P0 |
| RT-2 | know if the plan works | Monte Carlo success rate, the median and 10th percentile net worth path, and the earliest month each person can retire at a chosen success rate | P0 |
| RT-3 | understand what moves the answer | Show the effect of each lever: retire a year later, spend 10% less, save 5% more, claim Social Security later | P1 |
| IV-1 | see our real allocation | Allocation across every investment account by asset class, from holdings on statements and fund lookups | P0 |
| IV-2 | get a suggested allocation | Target allocation from horizon and a short risk questionnaire, a glide path to and through retirement, rebalancing amounts per account, and asset location hints | P0 |
| AI-1 | ask questions in plain words | Assistant chat over the ledger and the plan, answering with numbers from tools and links to the transactions behind them | P0 |
| AI-4 | redirect the AI and change assumptions by talking to it | Chat can change any assumption (returns, inflation, raises, retirement spending, tax rate), add or edit planned events, set standing instructions ("count Amazon as household, not mine") and correct the AI's reading of the books. Each change shows its effect on the plan and applies on one click, with a history and undo | P0 |
| AI-2 | run everything on my own computer | Local models by default; hosted models optional, off by default, with a redaction mode | P0 |
| AI-3 | see what the AI did | Activity log of every agent run and tool call; approvals queue for every change to the books | P0 |
| MC-1 | use my data from other AI tools | Local MCP server, off by default, read tools only in v1, scope per client, every call logged | P0 |
| SC-1 | keep the data safe | Encrypted database, unlock with the OS (Touch ID or Windows Hello) or a passcode, no telemetry by default, delete everything in one step | P0 |
| SC-2 | know my data is private | Private mode by default; a Connections page that lists every outside service in use and what it sends; a visible Private status | P0 |

## Success measures and release plan

v1 is done when the household's last 24 months are in the app, reconciled, and the retirement answer is trusted enough to act on.

| Measure | Target |
| --- | --- |
| Statements that reconcile on first import, text PDFs from the top 10 US banks and card issuers | 95% |
| Transactions categorized correctly without a correction (after the first month of rules) | 90% |
| Time to import and review one month for the household | under 10 minutes |
| Transfers and card payments counted as spending | 0 |
| Numbers in AI answers that do not trace to a ledger query | 0 (number guard) |
| Projection error, 12 months ahead, on net worth back-tested against the household's own history | within 5% |
| Data sent off the machine with default settings | only price lookups by ticker and update checks |

The build runs in five phases, about 22 weeks, detailed on the Architecture tab:

1. Foundation and shared harness (weeks 1 to 3).
2. Ingestion and reconciliation (weeks 4 to 8).
3. Ledger, P&L, balance sheet and close (weeks 9 to 12).
4. Projection, retirement and allocation engines (weeks 13 to 18).
5. Assistant, MCP server, polish and beta on Howard's machines (weeks 19 to 22).

## Open questions

The plan assumes the first answer to each of these. Each one changes scope if the answer differs.

- [ ] **Product or personal tool?** Sold through Turtlebox, like CeeVee. Decided 2026-11-14: a product for Mac and Windows. That brings licensing, support and a legal review of the allocation feature into v1.
- [ ] **Bank connections.** Decided 2026-11-14: file uploads in v1, Plaid in v1.x as an opt-in. Open: who holds the Plaid access tokens (see Architecture, Plaid).
- [ ] **Desktop like CeeVee?** Decided: macOS and Windows. A desktop app only; no web or phone version.
- [ ] **Taxes.** Decided 2026-11-14: US only, with federal and state income tax modelled for every state and DC, including a planned move between states.
- [ ] **Social Security.** Assumed: you each enter your benefit estimates from ssa.gov. Import of the SSA statement PDF is possible later.
- [ ] **Name.** Decided 2026-11-15: the app is called Golden. Still to do: check the name and domain are free, and trademark search before launch.
- [ ] **Price and trial.** Decided 2026-11-14: a 14-day free trial, then a one-time payment per major version (updates within that version included). Open: how many computers one licence covers (2 assumed). Plaid bills per linked account every month, so a household that turns on Plaid pays an annual subscription. Decided 2026-11-15: the buyer chooses between a local, private setup (the core one-time price only) and Plaid connectivity (the core one-time price plus the annual subscription on top). The Plaid subscription is positioned as an upsell: the core product is complete and private on its own, and connectivity is an optional paid extra offered where it saves the monthly downloads. Starting prices, decided 2026-11-15: $25 one-time for the core and $20 a year for Plaid on top. Revised 2026-11-16 (Howard, may change again): $39.99 one-time for the core and $5 a month for Plaid on top; the amounts live in `crates/fin-license/data/pricing.json`. Revised 2026-11-23 (Howard): Plaid on top is $9.97 a month or $100 a year. Open: whether to revisit these after the beta.
- [ ] **Legal review.** Before launch, counsel checks that rules-based allocation suggestions in software qualify as education, not personal investment advice, and reviews the privacy policy.
