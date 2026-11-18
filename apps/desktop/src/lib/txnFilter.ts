import type { TxnRow } from "./api";

/** Lines whose payee, memo, category, account or amount contain every word typed. */
export function filterTxns(rows: TxnRow[], query: string): TxnRow[] {
  const words = query.toLowerCase().split(/\s+/).filter(Boolean);
  if (words.length === 0) return rows;
  return rows.filter((r) => {
    const hay = [r.payee, r.memo, r.category ?? "", r.account, (Math.abs(r.amountCents) / 100).toFixed(2), r.day].join(" ").toLowerCase();
    return words.every((w) => hay.includes(w));
  });
}

/** Money in, money out and the net of the lines shown. */
export function sumTxns(rows: TxnRow[]): { inCents: number; outCents: number; netCents: number } {
  let inCents = 0;
  let outCents = 0;
  for (const r of rows) {
    if (r.amountCents >= 0) inCents += r.amountCents;
    else outCents += -r.amountCents;
  }
  return { inCents, outCents, netCents: inCents - outCents };
}
