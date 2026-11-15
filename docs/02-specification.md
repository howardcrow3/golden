# Specification

How each part of the app behaves, what the AI does and does not do, and the data it keeps. Every number the app shows is computed by code from the ledger; the model reads documents, labels things and explains.

## Screens and behaviour

Twelve screens in a left sidebar, the same shell as CeeVee. The household switch (Howard, spouse, both) sits at the top and filters every screen.

| Screen | What it shows | What you do there | Reqs |
| --- | --- | --- | --- |
| Overview | Net worth today and its 24-month line with the projection; this month's P&L so far; retirement readiness for each person; items waiting for review | Open anything that needs attention | BK-3, BK-4, RT-2 |
| Documents | Every imported file with institution, account, period, status (Reconciled, Off by $x, Needs a look, Duplicate) | Drop files, open a statement beside its extracted lines, fix a line, re-run extraction | IN-1 to IN-5 |
| Transactions | All transactions, searchable, with category, person, account, confidence and source page | Recategorize, split, mark as transfer, bulk edit, accept proposed rules | BK-1, BK-2 |
| Cash flow | Monthly P&L: income, taxes, spending by category, savings and savings rate, with variance against the trailing average | Change the period, drill from a number to its transactions | BK-3 |
| Balance sheet | Assets and liabilities by account and by tax treatment; net worth over time | Add manual assets (home, car) and their values; close a month | BK-4, BK-5 |
| Plan | Monthly projection of cash, accounts and net worth; planned events on a timeline | Add or edit events; save scenarios, compare up to three side by side, make one the plan | PR-1 to PR-3 |
| Retirement | Retirement month and claiming age per person; success rate; net worth fan chart; earliest safe date; lever table | Move the retirement dates, set spending in retirement, run the simulation | RT-1 to RT-3 |
| Investments | Current allocation by asset class and by account; target allocation and glide path; rebalancing amounts | Answer the risk questions, choose a glide path, mark a holding as off-limits | IV-1, IV-2 |
| Assistant | Chat over the ledger and the plan, as a panel on every screen; what-if and change cards; every number links to its source | Ask questions, run what-ifs, change assumptions, give standing instructions, redirect a running agent | AI-1 |
| Approvals | Proposed rules, merges, edits to closed months, from any agent | Accept, edit or dismiss; accept low-risk kinds in a batch | AI-3 |
| Activity | Every agent run with its steps, tool calls, time and tokens; every MCP call | Read what happened and why | AI-3, MC-1 |
| Settings | Household members, accounts, Private or Connected mode and the Connections page, Plaid links, models, providers, MCP clients and scopes, standing instructions, assumption history, security, backups, licence | Change any of it | AI-2, MC-1, SC-1 |

The app lives in the macOS menu bar and the Windows tray like CeeVee, so the watched folder and month-end agents run with the window closed. The tray menu shows documents waiting and approvals waiting.

## Chat, steering and what-ifs

The Assistant panel is open on every screen and knows which screen, period, person and scenario you are looking at. Whatever you type falls into one of four kinds, and each has a fixed way of taking effect. (AI-1, AI-4, PR-3, PR-4)

| Kind | Example | What happens | Takes effect |
| --- | --- | --- | --- |
| Question | "Why was October spending high?" | Answer from read tools, every figure linked to its source | Nothing changes |
| What-if | "What if I retire at 60 and we sell the house in 2035?" | A temporary scenario on top of the current plan is built and run; a what-if card shows the result next to the plan | Nothing changes until you save it or make it the plan |
| Change | "Use 3% inflation", "Assume my raises stop in 2030", "Spending in retirement should be $8,000 a month" | A change card shows the old and new value and the effect on the plan | One click on Apply; listed in the assumption history with undo |
| Instruction | "Count Amazon as household, not mine", "Ignore the 2025 bonus in trends", "Be more conservative with returns" | Stored as a standing instruction or a rule, shown on the card in plain words | One click on Keep; editable in Settings > Instructions |

**What-ifs.**

- A what-if is a list of overrides on the base plan: assumption values, retirement month or claiming age per person, added or removed events, spending changes, allocation changes. The model turns the sentence into that list with `draft-overrides`, under a schema and grammar; code validates each value (dates in range, amounts positive, ages 62 to 70 for Social Security).
- The card restates the overrides in plain words before the result, so a misread shows at once ("Howard retires Jan 2031 at 60; house sold Jun 2035 for $900,000 in today's dollars, $200,000 mortgage paid off"). Any line can be edited on the card.
- The engine runs the deterministic projection at once and the 2,000-run simulation in the background, about 2 seconds. The card shows plan success, earliest safe month per person, net worth at retirement and at 90, and monthly cash in the first retirement year, each against the base plan with the difference.
- Follow-ups stack: "and if we also spend 10% less?" adds to the open what-if. "Start over" clears it.
- Card actions: **Save as scenario** (named, listed on the Plan screen), **Compare** (opens the side-by-side view, up to 3 scenarios), **Make this the plan** (copies the overrides into the base plan as one change with undo).
- The same what-if can be built without chat: moving a stepper on the Retirement screen or editing an event opens the same card.

**Changes and instructions.**

- Assumptions live in one table with a history row per change: who changed it (you, or the Assistant on your click), when, old value, new value and the chat message it came from. Undo restores the old value and re-runs the plan.
- An instruction that can be a rule ("Amazon is household") becomes a categorization rule. One that steers the AI ("be more conservative") is stored as a standing instruction, added to every agent's prompt as user preference, and never overrides policy or the number guard.
- Changes to closed months, account merges and rules that touch more than 50 transactions still go through Approvals, with the chat card linking there.
- The Assistant may suggest a what-if or a change, but only you apply it. Text in documents cannot create one.

**Redirecting the AI mid-task.** While an agent is working (an import, a close), the panel shows its steps. Typing "stop" cancels the run; "skip the Amex statement" or "treat that $4,000 as a transfer" is passed to the running agent as a new instruction for that run, and the step it changes is shown.

## Ingestion

Every file goes through the same seven steps, run by the Ingest agent as a fixed workflow. The model makes a decision inside a step; code decides the order and checks the result.

1. **Receive.** Drag and drop, the watched folder, or (v1.x) a Gmail attachment. Hash the file; an identical hash is a duplicate and stops here. Store the original, encrypted, in the document store.
2. **Read.** CSV, OFX and QFX are parsed by code. PDFs with a text layer are read with positions kept, so table rows survive. Scanned pages go to OCR first.
3. **Classify.** `classify-document` returns kind (bank statement, card statement, brokerage or retirement statement, loan statement, paystub, W-2, other), institution, last four digits of the account, and period.
4. **Match the account.** Code matches institution and last four to a known account. A new account becomes a proposal: "New account: Chase Sapphire ending 4417. Add it?"
5. **Extract.** `extract-statement` or `extract-paystub` returns the lines under a JSON schema and grammar, so output always parses. For known institutions, a saved layout template (learned from the first good import) extracts by position and the model only checks it.
6. **Reconcile.** Code checks opening balance + sum of lines = closing balance to the cent, and that the period does not overlap a posted statement. Brokerage statements check holdings × price = market value within $1. Paystubs check gross − taxes − deductions = net. A failure sets the status to "Off by $x" and shows the lines most likely wrong (low confidence, or a total that is off by a single line's amount). Nothing posts until it reconciles or the person accepts the difference.
7. **Post.** Lines become ledger entries. Categorization, transfer pairing and duplicate checks run next (Ledger section).

**Statement library (IN-7).** Bank file names are useless ("Statement\_20261031\_8f3a.pdf", "download (3).pdf"), so the app files every document it reads into a library the person can browse outside the app.

- **Where it looks.** Downloads is watched from setup, after the person agrees, plus any other folders they add. Only files that look like financial documents are picked up: PDF, CSV, OFX and QFX that the classifier reads as a statement, paystub or tax form from a known kind of issuer. Everything else in Downloads is left alone. Drag and drop works too.
- **Where it files.** The library defaults to Documents/Golden/Statements on Mac and Windows and can be moved from Settings; moving it moves the files and updates every link. Inside, folders are by year, then by type: Bank, Credit cards, Brokerage, Retirement, Paystubs, Tax forms, Loans and mortgage, Insurance, Other. The year is the statement period's end (a Dec 15 to Jan 14 card statement goes in the January year); tax forms go by tax year.
- **How it names.** After OCR and extraction, the name comes from what is inside the document, not the old name: `2026-10 Chase Checking 4821 statement.pdf`, `2026-Q3 Fidelity 401k 2290 statement.pdf`, `2026-10-31 Acme paystub Howard.pdf`, `2025 W-2 Acme Howard.pdf`. Only the last four digits of an account appear. The pattern can be edited in Settings; clashes get a counter, and a second copy of the same file (same hash) is not filed twice.
- **Move or copy.** Files are moved by default, so Downloads stays clean; copy is a setting. The original name and location are kept in the database, and Activity can undo any filing, one file or a whole batch.
- **When it is unsure.** A document whose institution, account, period or type the AI is not confident about stays where it was and goes to Needs a look with the proposed name and folder; one click accepts or edits it. A document that posts but does not reconcile is still filed, marked in the app.
- **Privacy.** The library is ordinary files in the person's folder, outside the encrypted database, so they can open them in Preview or Explorer. If the library sits in a synced folder (iCloud Drive, OneDrive, Dropbox), the app says so on the Connections page and in Private mode offers to keep files only inside the encrypted store instead.

| Document | Fields extracted |
| --- | --- |
| Bank and card statement | Account, period, opening and closing balance, each line's date, description, amount, running balance where printed; card statements add payment due, minimum, APR, interest charged |
| Brokerage, 401(k), IRA, HSA statement | Period, holdings (symbol or fund name, quantity, price, value, cost basis where printed), contributions by source (employee, employer), dividends, fees |
| Loan and mortgage statement | Principal balance, rate, payment split into principal, interest and escrow |
| Paystub | Pay date, period, employer, person, gross by type (salary, bonus, RSU), each tax (federal, state, Social Security, Medicare), pre-tax deductions (401(k), HSA, FSA, medical), post-tax deductions (Roth 401(k), ESPP), employer match, net pay, year-to-date figures |
| W-2 (P1) | Box 1 to 17, to check the year's paystubs add up |

Sensitive identifiers are reduced on read: full account numbers keep only the last four digits, and Social Security numbers are dropped before any text reaches the model.

**Plaid (v1.x, Connected mode only).** A linked account syncs transactions, balances and holdings daily through Plaid. Synced lines post as provisional; when the month's statement arrives, reconciliation matches them line by line and the statement becomes the record. An account can be linked, paused or unlinked at any time; unlinking deletes its access token. In Private mode the Plaid option is hidden behind the mode switch.

## Ledger, P&L and balance sheet

The ledger is double-entry, so the P&L and balance sheet are two views of the same entries and always agree. Every entry links back to the document page and line it came from.

**Chart of accounts.** Asset accounts (cash, taxable investments, tax-deferred, tax-free, HSA, property), liability accounts (cards, mortgage, loans), income categories (salary, bonus, interest, dividends, other) and expense categories (about 40 in two levels: Housing > Mortgage interest, Food > Groceries). Each account and entry carries an owner: one person or joint.

**Posting rules.**

- A card purchase debits an expense category and credits the card liability. A card payment is a transfer between checking and the card, not spending.
- A paystub posts as one entry: gross salary as income; each tax as a tax expense; 401(k), HSA and Roth contributions as transfers into those accounts; insurance premiums as expenses; net pay into the deposit account. The matching bank deposit is linked, not posted twice.
- Employer match posts as income into the retirement account.
- Mortgage payments split into principal (reduces the liability), interest (expense) and escrow (tax and insurance expenses).
- Investment value changes post monthly as unrealized gain or loss, kept out of the spending P&L and shown as its own line.

**Categorization.** In order: the person's rules, then merchant memory (the last category used for this cleaned merchant name), then `categorize-transaction` on the local model with the embedding of similar past transactions as examples. Confidence under 0.8 sends the line to the review list. When the person fixes the same merchant twice, the Books agent proposes a rule.

**Transfer pairing.** Code looks for equal and opposite amounts across own accounts within 5 days. Ambiguous pairs go to the model with both descriptions, then to review.

**Reports.**

- P&L: income, taxes, spending by category, savings (income − taxes − spending) and savings rate, per person and household, for any month, quarter, year or trailing 12 months, with variance against the trailing 12-month average.
- Balance sheet: every account's balance at month end, grouped by type and by tax treatment, plus net worth and its change. Investment accounts use statement values; between statements, holdings × latest price.

**Monthly close.** The Close agent runs on the 5th of each month (or when the last statement for a month reconciles). It checks every active account has a reconciled statement for the month, lists anomalies (a new recurring charge, a category 50% over its average, a missing paycheck), writes a five-sentence month summary with every number from a ledger query, and proposes locking the month. Edits to a locked month are proposals.

## Projection and retirement engine

One Rust engine runs month by month from the last closed month to age 100 of the younger spouse. The deterministic run gives the Plan screen; 2,000 randomized runs give the Retirement screen. A full simulation takes under 2 seconds on an M1.

**Baseline from history.**

- Recurring items (salary, mortgage, subscriptions, insurance) are detected from at least 3 regular occurrences and carried forward at their latest amount and cadence.
- Variable spending per category uses the trailing 12-month median, with a monthly seasonality factor once 24 months of history exist (December gifts, summer travel). Outliers above 3 times the median are left out and shown as one-offs.
- Income grows at a raise rate per person (default: their own average from paystub history, capped at 5%). Spending grows with inflation (default 2.5%).

**Planned events.** Dated changes the person enters or accepts from the Assistant: a raise, a bonus, a new loan, a payoff, a home purchase or sale, college costs, a car every N years, a one-off cost. Each is a row with a start, an end, an amount and an inflation flag.

**Monthly step.** Income in, taxes out (federal brackets, FICA, and the household's state income tax, all indexed), contributions to each account at current rates up to IRS limits, spending out, debt payments on their amortization, returns on each account by its allocation, and surplus or shortfall to cash. A shortfall is drawn from accounts in a set order: cash, taxable, tax-deferred, tax-free.

**Taxes (US only).** Federal: ordinary brackets, long-term capital gains and qualified dividend brackets, the standard deduction, FICA and the Medicare surtax, taxation of Social Security benefits, and IRMAA surcharges on Medicare premiums. State: all 50 states and DC, each with its brackets or flat rate, standard deduction or exemptions, and its treatment of Social Security, pensions, IRA and 401(k) withdrawals and capital gains; local income taxes (New York City, Ohio and Pennsylvania cities and others) as an optional flat rate. The state comes from the household's address and paystubs; a move is a planned event, so "what if we move to Tennessee when we retire?" is an ordinary what-if. Tax years are paid in the projection in April, with withholding during the year from paystubs. The app projects tax for planning only and does not prepare returns.

**Retirement inputs per person.** Date of birth; retirement month; Social Security benefit at 62, 67 and 70 from ssa.gov and the claiming age; pension amount and start; part-time income after retirement. Household inputs: spending in retirement (today's spending, a percent of it, or a set amount), healthcare before Medicare at 65 (default $1,100 a month per person, inflated at 5%), required minimum distributions from 73 or 75 depending on birth year.

**Birthdays and age rules.** Setup asks each adult's and each dependent's date of birth; it is stored encrypted like everything else and never sent anywhere, even in Connected mode. A pure `age_events` function turns each birthday into dated plan events, in the month each age is reached (PR-8):

| Age | Rule the plan applies |
| --- | --- |
| 50 | 401(k), 403(b) and IRA catch-up contributions allowed |
| 55 | HSA catch-up; the rule of 55 for a 401(k) at an employer left that year |
| 59½ | Penalty-free withdrawals from IRAs and 401(k)s |
| 60 to 63 | Larger 401(k) catch-up limit |
| 62 to 70 | Social Security claiming window; full retirement age from birth year (67 for 1960 and later); benefit scaled by months early or late; spousal benefit up to 50% of the other's at full retirement age; survivor benefit |
| 65 | Medicare starts; pre-65 healthcare cost ends; IRMAA applies from income two years earlier |
| 73 or 75 | Required minimum distributions start (73 for birth years 1951 to 1959, 75 for 1960 and later) |
| Dependents under 17, 19 or 24 | Child tax credit, other dependent credit and student rules end on the right birthday |

Social Security treats a person born on the 1st as reaching an age the month before, and the engine follows that rule. Changing a birthday reruns the plan like any other change.

**Monte Carlo.** Monthly returns per asset class are drawn from a model fit to historic data (block bootstrap of 1926 onward US stock, bond and cash returns, so bad years cluster as they did). Inflation is drawn alongside. A run fails if spending cannot be met before the younger spouse turns 95.

**Outputs.**

- Success rate, and median, 10th and 90th percentile net worth by month.
- Earliest safe retirement month for each person at the chosen success rate (default 85%), found by searching retirement months with the other person's date held fixed.
- Lever table: the change in success rate from retiring 12 months later, spending 10% less, saving 5% more of pay, claiming Social Security at 70.
- Safe monthly spending in retirement at the chosen success rate.

Assumptions (returns, inflation, raise rate, tax rate) sit on one Assumptions panel with their sources, and every result shows the assumptions it used.

## Parameters and options

Every number the projection uses is a parameter with a list of options, not a single value. One option is active; the others are kept so a what-if is only a different pick. (PR-5)

**An option** has a value (or a value per year, such as raises of 4% to 2030 then 2%), a label, a source and a likelihood from 0 to 1. Sources, in the order the app trusts them:

1. **Your history.** Computed from the ledger: your raise from paystubs year over year, your spending growth, your bonus as a share of salary, your home's value trend.
2. **Reference values.** Bundled, cited and dated: CPI forecasts, long-run market returns, Social Security COLA history, healthcare cost trends.
3. **Presets.** Conservative, middle and optimistic around the first two.
4. **Yours.** Any value you type on the Assumptions panel or say in chat.

**Picking the likeliest.** Code scores each option: options from your own history score by how many years of data back them and how steady they are; reference values score by their source's own range; presets score lower; your own entries are not scored, they simply win once chosen. The highest score is active unless you have pinned a pick. At each month close the Plan agent re-scores, and if a better-supported option would change the plan success by 2 points or more, it proposes the switch ("Your raises averaged 3.1% over 4 years, not 2%. Use 3.1%?").

**Example: annual raise for Howard.**

| Option | Value | Source | Likelihood | Active |
| --- | --- | --- | --- | --- |
| Your history | 3.1% | 4 years of paystubs, 2.6% to 3.8% | 0.55 | Yes |
| Middle | 2.5% | Reference wage growth | 0.25 |  |
| Conservative | 1% | Preset | 0.10 |  |
| Optimistic | 4% | Preset | 0.10 |  |

**Changing a pick in chat.** "What if my raises are only 1%?" picks the Conservative option inside a what-if. "Assume 1.5% raises" adds a new option of yours and offers to make it active. "Show me raises from 1% to 4%" runs every option and shows a range. "Go back to what you think is likely" unpins it. Each pick takes effect the same way as any change (Chat, steering and what-ifs).

**Seeing what matters.** The Assumptions panel lists every parameter with its active option, and a sensitivity view runs each parameter's lowest and highest option with the others held, then sorts parameters by how far they move plan success. That tells you which assumptions deserve your attention.

**Uncertainty mode (P1).** Instead of fixing one option, the Monte Carlo can draw each run's option by likelihood, so the success rate reflects doubt about raises and inflation as well as market luck. Off by default, with a toggle on the Retirement screen.

**Parameters in v1** (each with options as above):

| Group | Parameters |
| --- | --- |
| Income | Raise per person, bonus as % of salary, years to next promotion, part-time income after retirement, pension amount and start |
| Spending | Spending growth before retirement, spending in retirement (% of today or amount), spending change at 75 and 85, healthcare before Medicare, long-term care reserve, one-off events |
| Markets | Return and volatility per asset class, inflation, interest on cash, mortgage and loan rates for new debt |
| Savings | Contribution rate per account, employer match, catch-up contributions from 50 |
| Taxes | State of residence by year, effective local rate, tax law after 2026 (current law or older brackets) |
| Social Security | Claiming age per person, benefit at 62, 67 and 70, cut to benefits from 2034 (none, 20%), COLA |
| Property | Home value growth, sale date and price, rent after a sale, car replacement cycle |
| Plan | Life expectancy per person, plan success target, withdrawal order |

## Lumpy income and time grain

Many households get a large share of their income in a few big payments: a bonus in March, RSU vests each quarter, dividends that bunch at quarter end, a tax refund in April. The app keeps every payment on the date it arrived, and projects each one on the date it is expected, so neither history nor the forecast smooths it away. (PR-6, BK-6)

**Income streams.** Every income line belongs to a stream, and each stream has a kind, an owner (person or joint), a cadence and a tax treatment.

| Kind | Typical cadence | How the amount is projected | Tax treatment |
| --- | --- | --- | --- |
| Salary | Weekly, biweekly, semimonthly or monthly | Current gross per pay date, grown by the raise parameter on the raise month | Regular withholding from the W-4 |
| Bonus | Yearly or twice yearly, on a known month | A share of salary (a parameter with options, e.g. 0%, 10%, 15%, last year's), paid on the usual month | Supplemental: 22% federal flat (37% above $1M), state supplemental rate |
| RSU vests | Quarterly or monthly, from the grant schedule | Shares vesting times a share price (a parameter: today's price, flat, or grown at the market return) | Ordinary income at vest, supplemental withholding, shares sold to cover |
| Options and ESPP | On exercise or purchase dates | Entered as planned events; ESPP on its purchase dates from the contribution rate | Per type: ISO, NSO, qualifying or not |
| Dividends | Quarterly or monthly per holding | Holding value times trailing yield, paid in the months the holding paid last year | Qualified or ordinary; reinvested or paid as cash per account |
| Interest | Monthly | Cash balance times the rate parameter | Ordinary |
| Tax refund or payment | Once a year, usually April | The year's true-up: tax computed minus tax withheld and paid | Not income; a cash event |
| Side and other income | Irregular | Trailing 12-month total placed in the months it came last year, or as planned events | Self-employment tax where it applies |
| Second and later jobs | Each on its own pay schedule | Per employer, like salary, with its own raise and end date | Withholding per W-4; the true-up catches under-withholding across jobs |
| Self-employment and 1099 | Irregular or per invoice | Trailing 12 months placed as it came, or a set amount per month | Self-employment tax, quarterly estimated payments, QBI deduction |
| Rental property | Monthly rent per property | Rent grown by a parameter, minus expenses and a vacancy option | Schedule E: net of expenses and depreciation; passive loss limits |
| Pension and annuity | Monthly from a start date | Fixed amount with a COLA option; survivor percentage after a death | Ordinary; most states tax it, some exempt it |
| Social Security | Monthly from the claiming month | From the SSA estimates, birthday and claiming age, with a COLA option | Up to 85% taxable federally; most states exempt it |
| Other | Per item | Trust distributions, royalties, alimony, unemployment, gifts and inheritances, as planned events or trailing history | Per item |

**Many streams at once.** A household can have any number of streams, and so can each person (two jobs and a rental, say). Each stream has its own start and end dates, its own growth parameter with options, and its own tax treatment, and streams can be tied to events (salary ends at the retirement month; a pension and Social Security start on their months). Income screens group streams by person or by kind, and the chat can add, end or change a stream ("my consulting stops next June"). (PR-7)

**Finding the cadence in history.** When a statement or paystub posts, `fin-ledger` groups income lines by payer and memo into a stream, then fits a cadence from the dates: fixed interval (7, 14 or about 30.4 days), fixed days of the month (the 15th and last), fixed months (March bonus; Feb, May, Aug, Nov vests), or irregular. Paystubs split one deposit into salary, bonus and RSU lines with their withholding. A new stream, or a stream that breaks its cadence, goes to Needs a look with the AI's reading for the person to confirm.

**Projecting it.** The plan engine produces a list of dated cash events, not a single monthly amount. Each stream places its payments on their expected dates, with withholding computed per payment (so a March bonus shows the 22% federal hold at the time it lands). Tax is settled for the year with an April true-up event, and quarterly estimated payments are added when withholding falls short of the safe harbor. Spending is projected monthly from trends and planned events and spread evenly across the days of the month, and shown as spread, not as dated. The Monte Carlo runs still step monthly; each month sums the events that fall in it.

**Time grain.** Every money screen (Overview, Cash flow, Budgets, Accounts, Plan) has one Week / Month / Quarter / Year switch in the top bar. The ledger stores each line by day, so any grain is a group-by, not a separate data set.

- Weeks are ISO weeks (Monday start); quarters and years are calendar by default, with an optional fiscal year start month.
- The choice is remembered per screen. Charts keep the same time span when the grain changes, so switching from Month to Week on a 12-month view shows 52 bars.
- Retirement stays in years; the plan's year table can open any year into months or weeks.
- Week and Month views of the future show dated income as dated and spread spending with a hatch so the two are not confused. A 13-week cash view on Overview lists the next big inflows and outflows by date.
- The chat can change the grain like any other control ("show me this by week").

**Budgets in any grain.** A budget line has a category, an amount and the grain it was set in ("$150 a week on groceries", "$9,000 a year on travel"). Shown in another grain, it converts by days (a weekly budget is amount times days in the period divided by 7), so a February month gets less than a March month. Spending is compared to the budget for the period on screen, and lumpy categories (insurance, tuition, travel) can be set to compare year-to-date instead of per period.

**Lumpy income and the AI.** The Assistant answers in the grain the person is looking at, and calls out lumps ("October was high because of the RSU vest; without it, saving was 14%"). The number guard applies as everywhere else.

## Investment allocation suggestions

The app suggests a household target allocation and the trades to reach it, computed by rules the person can read. It never places a trade.

**Current allocation.** Holdings from statements are mapped to asset classes: US stocks, international stocks, US bonds, TIPS, cash, real estate, other. Funds are looked up by ticker in a bundled fund table (updated with the app), so a target-date fund is split into its parts. Unknown holdings ask the person once.

**Target allocation.**

1. A six-question risk questionnaire gives a risk score from 1 to 5. The questions ask about loss tolerance, other income, and how the person reacted in 2020 and 2022.
2. The years to the earlier spouse's retirement set the starting point on a glide path: stocks = 110 − age, adjusted ±10 points by risk score, with a floor of 30% and a ceiling of 90%.
3. The glide path keeps stepping down to 5 years after retirement, then holds. The person can pick a flatter or steeper path.
4. Inside stocks, 70% US and 30% international by default. Inside bonds, TIPS take a share that grows to 30% at retirement.
5. The Monte Carlo run uses this target, so the success rate on the Retirement screen reflects it.

**Rebalancing.** When any asset class is more than 5 points from target, the app shows the amounts per account to buy and sell, preferring moves inside tax-advantaged accounts to avoid taxable gains, and new contributions before sales.

**Asset location hints.** Bonds and REITs in tax-deferred accounts, broad stock index funds in taxable, highest-growth holdings in Roth. Shown as suggestions with the estimated yearly tax difference.

**Wording.** Results say "suggested" and show the rule behind each number. A footer on the Investments screen says the suggestions are educational, rules-based and not personal investment advice. If the app is ever sold, this feature needs a legal review first (PRD open questions).

## AI tasks and agents

The AI follows CeeVee's rule for small local models: workflows first, free agents second. Background agents are fixed workflows; only the Assistant runs a free tool-calling loop, with a short tool list and a 12-step limit.

**Tasks.** Each task is a versioned prompt file, a JSON schema for its output and a grammar compiled from that schema, as in `ceevee-ai/prompts/`.

| Task | Input | Output |
| --- | --- | --- |
| `classify-document` | First two pages of text | Kind, institution, account last four, period, person |
| `extract-statement` | Statement text with positions | Balances and lines, each with a confidence |
| `extract-holdings` | Brokerage or retirement statement text | Holdings, contributions by source, fees |
| `extract-paystub` | Paystub text | Gross, taxes, deductions, match, net, year to date |
| `clean-merchant` | Raw description | Merchant name, location, kind of business |
| `categorize-transaction` | Cleaned line, 5 similar past lines with their categories | Category, owner, confidence |
| `pair-transfer` | Two candidate lines | Same transfer or not, with a reason |
| `explain-variance` | Category totals this month and trailing average | Two sentences naming the transactions behind the change |
| `summarize-month` | Close report from the ledger | Five-sentence month summary |
| `draft-event` | A chat request ("we'll buy a car in 2028 for 40k") | A planned event row to approve |
| `explain-result` | Simulation output and lever table | A plain explanation of the retirement answer |

Two tasks serve the chat: `classify-message` sorts a message into question, what-if, change or instruction, and `draft-overrides` turns a what-if or change into a validated list of overrides (assumption key and value, person and retirement month, event added or removed).

**Number guard.** CeeVee's fact guard, applied to money. After any task that writes text, code finds every amount, percent and date in the output and checks each against the tool results of that run. Anything that does not match is flagged and the text is not shown until it is fixed or regenerated.

**Agents.**

| Agent | Kind | Trigger | What it does | Tools |
| --- | --- | --- | --- | --- |
| Assistant | Free loop | Person opens chat | Answers questions, builds scenarios, drafts events, explains results | All read tools; draft tools; \`whatif.run\`, \`scenario.save\`; \`assumption.propose\`, \`instruction.propose\` and other propose tools, applied on your click |
| Ingest agent | Workflow | New file | Runs the seven ingestion steps | `docs.*`, `ingest.*`, `account.propose` |
| Books agent | Workflow | After posting | Categorizes, pairs transfers, flags duplicates, proposes rules | `ledger.read`, `ledger.categorize`, `rule.propose` |
| Close agent | Workflow | 5th of the month | Checks reconciliation, lists anomalies, writes the summary, proposes the lock | `ledger.read`, `report.*`, `close.propose` |
| Plan agent | Workflow | Month closed, or assumptions changed | Re-runs the projection and simulation, notes changes in the success rate over 3 points | `plan.run`, `plan.read` |

**Risk levels** follow CeeVee: read runs freely; draft is visible and reversible (a category on an open month, a scenario); propose needs the person (rules, new accounts, edits to closed months, planned events); act does not exist in this app because nothing moves money. Never available at any level: deleting documents or accounts, changing security or MCP settings, sending data off the machine.

**Untrusted content.** Statement and paystub text is wrapped as data and marked untrusted, as CeeVee treats postings and email. An instruction inside a document is never followed.

## MCP server

The app exposes its own tool registry as a local MCP server, the way CeeVee does with `rmcp`. It is off by default, read-only in v1, and every call is logged on the Activity screen.

**Transport.** stdio for clients that launch it (Claude Desktop, Claude Code), and streamable HTTP on 127.0.0.1 only, with a per-client bearer token, for clients that connect. No remote access.

**Clients and scopes.** Each client is added in Settings with a name, a token and a scope. Data returned to an MCP client goes to whatever model that client uses, which may be a hosted one, so the scope screen says so plainly.

| Scope | What the client can see |
| --- | --- |
| Summary | Totals only: net worth, P&L by category, account balances by type, retirement success rate and dates. No merchant names, no account numbers |
| Detail | Summary plus transactions, holdings and scenarios |
| Draft (v1.x) | Detail plus creating scenarios and notes, which land as proposals |

**Tools (v1).**

| Tool | Returns | Scope |
| --- | --- | --- |
| `get_net_worth(as_of?)` | Assets, liabilities, net worth, by type | Summary |
| `get_pnl(period, person?)` | Income, taxes, spending by category, savings rate | Summary |
| `get_balance_sheet(as_of?)` | Balances by account type and tax treatment | Summary |
| `get_projection(months, scenario?)` | Monthly cash, net worth and account balances | Summary |
| `get_retirement_plan(scenario?)` | Dates, success rate, percentile paths, levers | Summary |
| `get_allocation()` | Current and target allocation, rebalancing amounts | Summary |
| `list_transactions(filter)` | Matching transactions, 200 at most per call | Detail |
| `get_holdings(account?)` | Holdings with values | Detail |
| `list_scenarios()` | Saved scenarios and their results | Detail |
| `run_scenario(changes)` | A what-if result, not saved | Detail |

The app is also an MCP client, as CeeVee is, for the optional Gmail connector that pulls statement attachments (v1.x).

## Data model

One encrypted SQLite file. Money is stored as integer cents; dates as ISO dates; AI output as JSON with a `*_with` column naming the prompt version, as in CeeVee.

| Table | Holds | Key columns |
| --- | --- | --- |
| `person` | Household members | name, role (adult or dependent), date of birth, retirement month, SS estimates, claiming age |
| `institution` | Banks, issuers, brokers, employers | name, layout template |
| `account` | Every account | institution, owner, type, tax treatment, last four, open and close dates |
| `document` | Each imported file | hash, kind, account, period, status, encrypted blob path, extracted JSON |
| `statement` | A reconciled statement | document, account, period, opening, closing, reconciled at |
| `entry` | A ledger entry (one transaction) | date, description, merchant, document line, owner, month locked |
| `posting` | One side of an entry | entry, account or category, amount in cents |
| `category` | Income and expense categories | parent, name, kind |
| `rule` | Categorization rules | match (merchant, text, amount range, account), category, owner, source |
| `merchant` | Cleaned merchant names | name, default category, embedding |
| `paystub` | Parsed paystub | person, employer, pay date, lines JSON, entry |
| `holding` | Holdings at a statement date | account, symbol, asset class mix, quantity, price, value, cost basis |
| `price` | Prices by symbol and date | symbol, date, close |
| `fund` | Bundled fund table | symbol, asset class mix, expense ratio |
| `month_close` | Close status per month | month, checks JSON, summary, locked at |
| `event` | Planned events | name, owner, start, end, amount, inflation flag, scenario |
| `scenario` | Saved what-ifs | name, overrides JSON, last result JSON |
| `parameter` | Every projection parameter (Parameters and options) | key, group, unit, person, active option, pinned |
| `risk_profile` | Questionnaire answers and score | person, answers, score, glide path |
| `sim_run` | Simulation results | scenario, inputs hash, success rate, percentiles JSON |
| `agent_run`, `agent_step`, `proposal`, `policy`, `agent_memory` | The CeeVee harness tables, unchanged |  |
| `chat_thread`, `chat_message`, `chat_summary` | Assistant conversations |  |
| `mcp_client`, `mcp_call` | MCP clients, scopes, call log | client, scope, token hash; tool, arguments, result size, time |

Tables added for chat, what-ifs, privacy and licensing:

| Table | Holds | Key columns |
| --- | --- | --- |
| `assumption_change` | History of every assumption change | parameter, old option, new option, source (control, chat message or Plan agent), applied at, undone at |
| `instruction` | Standing instructions to the AI | text, scope (all agents or one), created from message, active |
| `whatif` | Open and recent what-ifs | base scenario, overrides JSON, chat thread, result JSON, saved as scenario |
| `connection` | Every outside service in use | kind (Plaid, price lookups, hosted model, Gmail, MCP client), status, what it sends, linked at |
| `plaid_item` | Linked Plaid items | institution, accounts, token reference in the keychain, last sync, status |
| `license` | CeeVee's licence table, unchanged |  |
| parameter\_option | The options for each parameter | parameter, label, value or per-year values JSON, source kind, source detail, likelihood, scored at, created by |
| income\_stream | Each source of income and how it arrives | kind, owner, payer, cadence (interval, days of month or months), tax treatment, amount parameter, starts, ends, linked event (such as the retirement month), confirmed |
| budget | Budget lines in the grain they were set | category, amount, grain (week, month, quarter, year), compare (per period or year to date), starts, ends |
| grant | Equity grants and their vesting schedules | owner, type (RSU, ISO, NSO, ESPP), symbol, shares, vest dates and shares, strike, sell-to-cover |
| library\_file | Where each document was found and where it was filed | document, file hash, original path and name, library path, doc type, year, moved or copied, filed at, undone at |
| watch\_source | Folders the app watches for new documents | path, enabled, move or copy, added at |

`scenario` gains `parent` (scenarios stack on the base plan or on another scenario) and `is_plan` (exactly one row is the plan).

FTS5 covers transaction descriptions and document text. `sqlite-vec` holds merchant and transaction embeddings in the same file.
