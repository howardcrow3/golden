import type { TxnRow } from "./api";

/** Lines whose payee, memo, note, tag, category, account or amount contain every word typed. */
export function filterTxns(rows: TxnRow[], query: string): TxnRow[] {
  const words = query.toLowerCase().split(/\s+/).filter(Boolean);
  if (words.length === 0) return rows;
  return rows.filter((r) => {
    const hay = [r.payee, r.memo, r.category ?? "", r.account, r.note ?? "", (r.tags ?? []).join(" "), (Math.abs(r.amountCents) / 100).toFixed(2), r.day].join(" ").toLowerCase();
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

export type Lane = { category: string | null; rows: TxnRow[]; totalCents: number };

/** Groups lines into board lanes: uncategorised first, then categories by size of spend. */
export function groupByCategory(rows: TxnRow[]): Lane[] {
  const lanes = new Map<string | null, Lane>();
  for (const r of rows) {
    const key = r.category ?? null;
    const lane = lanes.get(key) ?? { category: key, rows: [], totalCents: 0 };
    lane.rows.push(r);
    lane.totalCents += r.amountCents;
    lanes.set(key, lane);
  }
  return [...lanes.values()].sort((a, b) => (a.category === null ? -1 : b.category === null ? 1 : Math.abs(b.totalCents) - Math.abs(a.totalCents)));
}
