# Visual design

The app uses the High Turtle (Turtlebox) design system as it stands in `howardcrow3/highturtle`, `turtlebox/design-system/`: warm neutrals, one gold accent, Inter only, square corners, rules instead of shadows.

## Design rules

The system was written for a marketing site, so Golden keeps every rule and adds the pieces a data-dense app needs, built from the same tokens and proposed back upstream, as CeeVee's `docs/DESIGN.md` does.

**Kept as written**

- Role tokens only (`paper`, `surface`, `line`, `ink`, `body`, `border-control`, `accent`, `on-ink`, `on-accent`, `danger`, `focus-ring`), never hex. Light and dark both first class.
- Inter only: Light 300 for headings and body, Regular and Medium for UI. Labels and buttons 11px uppercase with 0.3em tracking.
- Square corners, 1px `line` rules, `surface` plates. No shadows, gradients or rounded cards.
- 8px spacing scale. 300ms ease-out motion, removed under reduced motion. 2px focus ring at 4px offset.
- Gold is never text and never carries meaning alone. No icon set, no emoji, no imagery. Voice per `voice.md`: plain, specific, no exclamation marks.

**Added for Golden**

- **Figures.** All money in Inter with tabular numerals (`font-variant-numeric: tabular-nums`) so columns align. Large figures (net worth, success rate) use the `page-title` style at Light 300; table figures use `body-sm`. Negative amounts show a minus sign, never red alone, never parentheses.
- **Gains and losses.** The system has one added hue, `danger`, for errors. A loss is not an error, so it does not use `danger`. Direction is shown with a sign and a word ("down $1,240 from October"). `danger` is kept for failures: a statement that does not reconcile, a plan below the success threshold.
- **Tables.** Row height 40px, a 1px `line` rule between rows, figures right aligned, header in the `label` style. The selected row is a `surface` plate with a 2px `ink` rule on the left.
- **Status.** CeeVee's status chip: a round dot plus a literal word. Reconciled (filled `accent` dot), Off by $x (`danger` dot plus the amount), Needs a look (hollow `border-control` dot), Locked (filled `ink` dot).
- **Numbers link to their source.** Any figure can be clicked to open the transactions or document lines behind it. A dotted `line` underline marks it, turning `accent` on hover, the same as the system's link underline.

## Chart language

Charts use the warm phase ramp from `sites/highturtle.css` for series and keep gold for the one thing that matters on each chart. That gives a rich palette without adding a hue the system does not have.

| Chart role | Token | Light | Dark | Rule |
| --- | --- | --- | --- | --- |
| Actual history line | `ink` | #1c1c1c | #f5f0e7 | 2px solid |
| Projection line | `ink` | #1c1c1c | #f5f0e7 | 2px, dashed 4 4, starts at the last closed month |
| Projection range (10th to 90th percentile) | `s2` band | #d9d2c5 | #34312d | Filled band, no edge |
| Inner range (25th to 75th) | `s3` band | #c8b99a | #46413a | Filled band over the outer one |
| The one highlight (retirement month, today, target) | `accent` | #d4a853 | #d4a853 | A 1px vertical rule or a filled dot, always with a text label beside it |
| Category series, up to 6 | `s1` to `s6` | ramp | ramp | Ordered by size, largest darkest; a 7th and later group as Other in `s1` |
| Gridlines | `line` | #d9d2c5 | #2e2e2e | 1px, horizontal only |
| Axis labels and ticks | `body` | #3a3530 | #c8b99a | `fine` style, tabular numbers |
| Below threshold, failed runs | `danger` | #8f3b2a | #e58e80 | Only for a plan under the success target or a failure, with words |

**Chart kinds by screen**

- Net worth: area under the actual line in `s2`, the projection dashed with its range bands, a gold rule at today.
- Cash flow: income and spending as side-by-side bars per month, savings as a line above them; a 12-month strip of savings rate under it.
- Spending by category: a horizontal bar per category against its trailing average (a short `ink` tick), sorted by amount. No pie charts.
- Balance sheet: one stacked bar per month in the six tax-treatment groups, liabilities below zero.
- Retirement: a fan chart of household net worth by age, the two retirement months as gold rules labelled with each name, Social Security start marked on the axis.
- Allocation: two 100% bars, Current over Target, in the asset-class ramp, with the difference in points written at the end of each segment.

Every chart has a title that states the finding ("Net worth up $18,400 since June"), a source line naming the months it covers, and keyboard access to each point. Hover shows the exact figures in a `surface` plate with a `line` border.

## App shell and screen layouts

The shell is CeeVee's: a fixed sidebar, a content column on a 12-column grid, and an Assistant panel that slides in from the right. Minimum window 1100 × 720; the content column caps at 1280px and centres.

**Shell**

- **Sidebar, 232px, `surface` plate, 1px `line` rule on its right.** The turtle mark and app name at the top in the `wordmark` style. The household switch under it: three text tabs (Howard, Spouse, Both) with a gold underline on the active one. Nav in two groups split by a rule: Overview, Cash flow, Balance sheet, Plan, Retirement, Investments; then Transactions, Documents, Approvals, Activity, Settings. Counts sit right aligned in `fine` ("Approvals 3"). The active item is an `ink` fill with `on-ink` text, as the system's primary button.
- **Top bar, 64px, on `paper`.** Page title in `page-title` at 40px, the period picker on the right (Month, Quarter, Year, Trailing 12), and an "Ask" button that opens the Assistant.
- **Assistant panel, 400px.** A `surface` plate with a `line` rule on its left. Answers set figures in tabular numbers and underline each one as a source link. Tool steps show as quiet `fine` lines ("Read October P&L") that expand.

**Screens**

| Screen | Layout, top to bottom |
| --- | --- |
| Overview | A row of four figure tiles (Net worth, This month's savings, Savings rate, Retirement success rate); the net worth chart across 12 columns; two half-width plates: Cash flow this month and Needs review (documents and approvals, as a list) |
| Cash flow | Period summary line; the income and spending bar chart; the spending-by-category bars against average on 8 columns, the selected category's transactions on 4 |
| Balance sheet | Net worth figure and change; the stacked monthly bar; the account table grouped by tax treatment with a subtotal row per group |
| Plan | The projection chart with events as gold-labelled ticks on the time axis; the events table under it; a scenario switch in the top bar |
| Retirement | Two person cards side by side (name, retirement month stepper, claiming age stepper, earliest safe month); the success rate as a large figure with its threshold; the fan chart; the lever table (lever, new success rate, change) |
| Investments | Current over Target allocation bars; the glide path chart; the rebalancing table per account with buy and sell amounts |
| Documents | A drop zone across the top with a 1px dashed `border-control` frame; the documents table; selecting one opens a split view, the PDF page on the left and its extracted lines on the right with the reconciliation sum at the foot |
| Transactions | A filter row; the table with category as an inline select; a review strip at the top when lines are under the confidence threshold |
| Approvals and Activity | CeeVee's screens, unchanged apart from the finance proposal kinds |

**Empty and first-run states** use the voice rules: "No statements yet. Drop a PDF or CSV here, or choose a folder to watch." First run asks for the household members and birth months, then the first statements, then shows the Overview filling in as documents reconcile.

## Chat panel, what-if cards and privacy status

The chat panel sits beside every screen, so you can see a what-if land on the chart you are looking at.

- **Panel.** 400px, docked right, a `surface` plate with a `line` rule on its left; open by default on Plan and Retirement, one click elsewhere. A context line at the top in `fine` names what the chat sees ("Retirement, Base plan, Both"). The input is a `border-control` field with a primary Send button; suggested prompts sit above it as plain text links, never chips with icons.
- **Messages.** Your messages are `body` text on `paper`, right-aligned under a `label` "You". Answers are `body` text, left-aligned, figures in tabular numbers with the dotted source underline. Agent steps show as quiet `fine` lines that expand.
- **What-if card.** A `surface` plate with a 2px `accent` rule on top, the only gold edge in the app, so a hypothetical never reads as the real plan. First the overrides in plain words, each line editable; then a three-column table (Measure, Plan, What-if) with the difference written out ("+2 points", "9 months earlier"); then the actions Save as scenario, Compare and Make this the plan. While the simulation runs, the table shows the deterministic numbers and a `fine` line "Running 2,000 market paths".
- **On the chart.** While a what-if is open, the screen's main chart draws the what-if as a dashed `ink` line over the plan's solid line, and a `label` "What-if" with a gold dot sits in the chart title. Closing the card removes both.
- **Change card.** Same plate without the gold rule: old value struck through in `body`, new value in `ink`, the effect on plan success, and Apply and Not now. After Apply the card collapses to one line, "Inflation set to 3.0%. Undo".
- **Compare view.** Up to three scenarios as columns, the plan first, rows for each measure, the best value in a row marked with a filled gold dot beside it, never by colour alone.
- **Privacy status.** The sidebar foot reads "Private. Nothing leaves this computer" with a hollow `ink` dot, or "Connected. 2 services" with a filled dot, linking to the Connections page. That page is a plain table: service, what it sends, since, and a Disconnect button.
- **Assumptions panel.** Opens from the Assumptions button on Plan and Retirement, and from any change card. One row per parameter, grouped as in the Specification: name, then its options as a row of text buttons with the value and a `fine` source line under each ("3.1%, your paystubs"). The active option is an `ink` fill; the app's likeliest pick carries a small filled gold dot and the word "Likeliest", so you can see when you have moved off it. A pin marks a pick the app will not change. "Add a value" opens a field. A sensitivity strip at the top lists the five parameters that move plan success most, each as a horizontal bar from its low to its high option, in `s3` with the active value as an `ink` tick.
- **Time switch.** A segmented control of four text buttons, Week, Month, Quarter, Year, at the right of every money screen's header. Square, 1px `border-control`, the active one an `ink` fill with `on-ink` text. Keyboard: W, M, Q, Y. No dropdown, so the current grain is always visible.
- **Income by stream.** Lumpy income is drawn as stacked bars, one segment per stream in ramp order (salary `s6`, bonus `s4`, RSU vests `s3`, dividends and interest `s2`), so the bonus month and vest months stand out by height. Projected bars sit right of the today rule as outlines in the same ramp. A lump over twice the median bar gets a `fine` label above it ("Bonus $18.4k"). The legend is a row of swatches and names under the plate title, not a box.
- **Spread versus dated.** In Week and Month views of the future, dated items (pay, vests, the April true-up) are solid; spending spread across the period is a 45° hatch in the same color, with "spread" in the legend.
- **Budgets.** One row per category: name, a bar of spent against budget for the period on screen (`ink` up to 100%, `danger` past it), and the budget in the grain it was set ("$150 / week") in `fine` text when that differs from the grain on screen.

## Mock-ups

The Overview, Retirement and Plan-with-chat screens are drawn at full size, in the real tokens, light and dark, on a design canvas: [Golden screens](https://claude.ai/artifact/2Mbh7hSkqnVbFCZbGrkR9q). The figures on them are samples for layout only.

- **Overview:** four figure tiles, the net worth chart with 24 months actual and 36 projected, October cash flow, and the Needs a look list with status dots.
- **Retirement:** a card per person with month and claiming-age steppers, the 87% plan success figure, the fan chart with both retirement dates in gold, and the lever table.
- **Plan with chat:** the Assistant panel open with a what-if ("retire at 60 and sell the house in 2035") as a gold-ruled card, the what-if drawn dashed over the plan, a change card for 3% inflation, and the Private status in the sidebar foot.

**One conflict to settle.** The `highturtle` repo's `main` branch (updated Nov 11) sets everything in Inter. The Turtlebox Design System artifact and CeeVee's `docs/DESIGN.md` still set headings in Cormorant Garamond. The mock-ups follow the repo and use Inter only.
