import type { Recurring } from "./api";

export interface Upcoming {
  date: string;
  payee: string;
  cents: number;
}

const STEP_DAYS: Record<string, number> = {
  weekly: 7,
  "every two weeks": 14,
  monthly: 30,
  quarterly: 91,
  yearly: 365,
};

const DAY = 86_400_000;
const iso = (t: number) => new Date(t).toISOString().slice(0, 10);
const parse = (s: string) => Date.parse(`${s}T00:00:00Z`);

/**
 * Dated payments and income expected over the next `weeks` weeks, soonest first, from what
 * repeats in the statements. A payment already overdue by up to a step is shown on `today`.
 */
export function upcoming(rows: Recurring[], today: string, weeks = 13): Upcoming[] {
  const start = parse(today);
  const end = start + weeks * 7 * DAY;
  const out: Upcoming[] = [];
  for (const r of rows) {
    const step = STEP_DAYS[r.cadence];
    let t = parse(r.nextExpected);
    if (!step || Number.isNaN(t) || r.typicalCents === 0) continue;
    // Late by less than one step: assume it is about to land.
    if (t < start && start - t < step * DAY) t = start;
    while (t < start) t += step * DAY;
    for (; t <= end; t += step * DAY) out.push({ date: iso(t), payee: r.payee, cents: r.typicalCents });
  }
  return out.sort((a, b) => a.date.localeCompare(b.date) || a.payee.localeCompare(b.payee));
}

/** The biggest dated items first-in-time order is kept; `limit` keeps the largest by size. */
export function biggest(items: Upcoming[], limit: number): Upcoming[] {
  const keep = new Set([...items].sort((a, b) => Math.abs(b.cents) - Math.abs(a.cents)).slice(0, limit));
  return items.filter((i) => keep.has(i));
}
