import type { TxnRow } from "./api";
import { filterTxns } from "./txnFilter";

export type WatchTotals = { thisMonthCents: number; lastMonthCents: number; count: number };

/** Money spent (never transfers) on lines that match `query`, this month and last month. */
export function watchTotals(rows: TxnRow[], query: string, today: Date): WatchTotals {
  const ym = (d: Date) => d.toISOString().slice(0, 7);
  const thisMonth = ym(today);
  const last = new Date(Date.UTC(today.getUTCFullYear(), today.getUTCMonth() - 1, 1));
  const lastMonth = ym(last);
  let thisMonthCents = 0;
  let lastMonthCents = 0;
  let count = 0;
  for (const r of filterTxns(rows, query)) {
    if (r.amountCents >= 0 || r.category === "Transfer") continue;
    const m = r.day.slice(0, 7);
    if (m === thisMonth) thisMonthCents += -r.amountCents;
    else if (m === lastMonth) lastMonthCents += -r.amountCents;
    else continue;
    count += 1;
  }
  return { thisMonthCents, lastMonthCents, count };
}
