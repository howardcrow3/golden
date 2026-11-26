import type { Upcoming } from "./upcoming";

export interface CalendarDay {
  /** YYYY-MM-DD */
  date: string;
  /** Day of the month; 0 for the blank cells before the 1st and after the last day. */
  day: number;
  items: Upcoming[];
  /** Net of the day's items: income minus bills. */
  netCents: number;
}

const iso = (t: number) => new Date(t).toISOString().slice(0, 10);

/**
 * One month as rows of seven days, Sunday first. Items dated in that month are placed on their
 * day; padding cells have `day` 0. `month` is 1-12.
 */
export function calendarMonth(items: Upcoming[], year: number, month: number): CalendarDay[][] {
  const first = Date.UTC(year, month - 1, 1);
  const daysInMonth = new Date(Date.UTC(year, month, 0)).getUTCDate();
  const lead = new Date(first).getUTCDay();
  const byDate = new Map<string, Upcoming[]>();
  for (const i of items) byDate.set(i.date, [...(byDate.get(i.date) ?? []), i]);

  const cells: CalendarDay[] = [];
  for (let k = 0; k < lead; k++) cells.push({ date: "", day: 0, items: [], netCents: 0 });
  for (let d = 1; d <= daysInMonth; d++) {
    const date = iso(Date.UTC(year, month - 1, d));
    const here = byDate.get(date) ?? [];
    cells.push({ date, day: d, items: here, netCents: here.reduce((s, i) => s + i.cents, 0) });
  }
  while (cells.length % 7 !== 0) cells.push({ date: "", day: 0, items: [], netCents: 0 });

  const weeks: CalendarDay[][] = [];
  for (let k = 0; k < cells.length; k += 7) weeks.push(cells.slice(k, k + 7));
  return weeks;
}

/** Month name and year for a heading. */
export function monthTitle(year: number, month: number): string {
  return new Date(Date.UTC(year, month - 1, 1)).toLocaleDateString("en-US", { month: "long", year: "numeric", timeZone: "UTC" });
}

/** The (year, month) a number of months after `year`/`month`. */
export function addMonths(year: number, month: number, delta: number): { year: number; month: number } {
  const t = year * 12 + (month - 1) + delta;
  return { year: Math.floor(t / 12), month: (t % 12) + 1 };
}
