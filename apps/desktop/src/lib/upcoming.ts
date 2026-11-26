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

/** Whole calendar months per step for the cadences that follow the calendar. */
const STEP_MONTHS: Record<string, number> = { monthly: 1, quarterly: 3, yearly: 12 };

const DAY = 86_400_000;
const iso = (t: number) => new Date(t).toISOString().slice(0, 10);
const parse = (s: string) => Date.parse(`${s}T00:00:00Z`);

/** `t` moved `n` calendar months on, keeping day `anchor` (or the month's last day when it is shorter). */
function addCalendarMonths(t: number, n: number, anchor: number): number {
  const d = new Date(t);
  const m = d.getUTCFullYear() * 12 + d.getUTCMonth() + n;
  const y = Math.floor(m / 12);
  const mo = m % 12;
  const last = new Date(Date.UTC(y, mo + 1, 0)).getUTCDate();
  return Date.UTC(y, mo, Math.min(anchor, last));
}

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
    const months = STEP_MONTHS[r.cadence];
    if (months) {
      // Monthly, quarterly and yearly payments keep their day of the month (a 30-day step drifts).
      const anchor = new Date(parse(r.nextExpected)).getUTCDate();
      const next = (x: number) => addCalendarMonths(x, months, anchor);
      while (t < start) t = next(t);
      for (; t <= end; t = next(t)) out.push({ date: iso(t), payee: r.payee, cents: r.typicalCents });
      continue;
    }
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
