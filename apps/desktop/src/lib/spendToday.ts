export interface SafeToSpend {
  /** Budget left this month minus bills still to come; negative when over. */
  leftCents: number;
  /** What is left spread over the days remaining, including today (never below zero). */
  perDayCents: number;
  daysLeft: number;
  billsCents: number;
  over: boolean;
}

/** Days from `today` to the end of its month, counting today. `today` is YYYY-MM-DD. */
export function daysLeftInMonth(today: string): number {
  const [y, m, d] = today.split("-").map(Number);
  const last = new Date(Date.UTC(y, m, 0)).getUTCDate();
  return last - d + 1;
}

/** The last day of `today`'s month as YYYY-MM-DD. */
export function monthEnd(today: string): string {
  const [y, m] = today.split("-").map(Number);
  return new Date(Date.UTC(y, m, 0)).toISOString().slice(0, 10);
}

/**
 * Safe to spend: the overall monthly budget still unspent, less the bills that are due before the
 * month ends (`billsCents`, a positive amount), spread across the days left.
 */
export function safeToSpend(budgetRemainingCents: number, billsCents: number, today: string): SafeToSpend {
  const daysLeft = daysLeftInMonth(today);
  const leftCents = budgetRemainingCents - billsCents;
  return {
    leftCents,
    perDayCents: leftCents > 0 ? Math.floor(leftCents / daysLeft) : 0,
    daysLeft,
    billsCents,
    over: leftCents < 0,
  };
}
