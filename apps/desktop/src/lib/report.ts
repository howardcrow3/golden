import type { BudgetRow, QuickPlanResult } from "./api";
import { formatCents, formatPercent } from "./money";

const esc = (s: string) =>
  s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");

/** One CSV cell: quoted when needed, and text that a spreadsheet would run as a formula is defused. */
export function csvCell(v: string | number): string {
  let s = String(v);
  if (typeof v === "string" && /^[=+\-@\t\r]/.test(s)) s = "'" + s;
  return /[",\n\r]/.test(s) ? `"${s.replace(/"/g, '""')}"` : s;
}

export function planCsv(r: QuickPlanResult): string {
  const lines = [["Year", "Age", "Balance", "Spending", "Taxes"].join(",")];
  for (const y of r.years) {
    lines.push(
      [y.year, y.age, y.balanceCents / 100, y.spendingCents / 100, y.taxesCents / 100]
        .map(csvCell)
        .join(","),
    );
  }
  return lines.join("\n") + "\n";
}

export function planJson(r: QuickPlanResult, title: string, when: string): string {
  return JSON.stringify({ title, generated: when, ...r }, null, 2);
}

/** A self-contained page: open it in any browser and print it to PDF. Nothing is loaded from outside. */
export function planHtml(r: QuickPlanResult, title: string, when: string): string {
  const money = (c: number) => esc(formatCents(c, { whole: true }));
  const rows = r.years
    .map(
      (y) =>
        `<tr><td>${y.year}</td><td>${y.age}</td><td>${money(y.balanceCents)}</td><td>${money(y.spendingCents)}</td><td>${money(y.taxesCents)}</td></tr>`,
    )
    .join("\n");
  const ends = r.depletedYear
    ? `<p>On the average path the money runs out in ${r.depletedYear}.</p>`
    : "";
  return `<!doctype html>
<html lang="en"><head><meta charset="utf-8"><title>${esc(title)}</title>
<style>
body{font:14px/1.5 -apple-system,Segoe UI,sans-serif;max-width:800px;margin:32px auto;color:#222;padding:0 16px}
h1{font-size:22px;margin:0}.sub{color:#666;margin:0 0 20px}
.big{font-size:32px;font-weight:600}
table{border-collapse:collapse;width:100%}th,td{text-align:right;padding:4px 8px;border-bottom:1px solid #ddd}
th:first-child,td:first-child{text-align:left}
.note{color:#666;font-size:12px;margin-top:24px}
@media print{body{margin:0}}
</style></head><body>
<h1>${esc(title)}</h1><p class="sub">Prepared ${esc(when)}</p>
<p class="big">${esc(formatPercent(r.successRate, 0))} chance the money lasts to ${r.horizonAge}</p>
<p>Middle outcome ${money(r.p50Cents)} at the end; a rough decade ${money(r.p10Cents)}, a good decade ${money(r.p90Cents)}.</p>
${ends}
<h2>Year by year</h2>
<table><thead><tr><th>Year</th><th>Age</th><th>Balance</th><th>Spending</th><th>Taxes</th></tr></thead>
<tbody>
${rows}
</tbody></table>
<p class="note">Assumes ${esc(formatPercent(r.realReturn))} real return, ${esc(formatPercent(r.volatility))} volatility, Social Security at ${r.ssClaimAge}. Amounts are in today's dollars. These are estimates, not advice.</p>
</body></html>
`;
}

/** Budget against spending for a period, one row per category, in plain dollars. */
export function budgetCsv(rows: BudgetRow[], period: string): string {
  const lines = [["Period", "Category", "Budget", "Spent", "Remaining"].join(",")];
  for (const r of rows) {
    lines.push([period, r.category, r.budgetCents / 100, r.spentCents / 100, r.remainingCents / 100].map(csvCell).join(","));
  }
  return lines.join("\n") + "\n";
}
