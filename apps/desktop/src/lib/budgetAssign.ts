import type { BudgetRow } from "./api";
import type { Upcoming } from "./upcoming";

/** What this month's income leaves unassigned once every category budget is taken out. Negative means over-assigned. */
export function leftToAssign(incomeCents: number, rows: BudgetRow[]): { assignedCents: number; leftCents: number } {
  const assignedCents = rows.filter((r) => r.category !== "Total").reduce((s, r) => s + r.budgetCents, 0);
  return { assignedCents, leftCents: incomeCents - assignedCents };
}

/**
 * Income for the month: what has arrived so far plus the recurring pay still expected before
 * `monthEnd` (YYYY-MM-DD). With no recurring pay this is just what has arrived.
 */
export function expectedIncome(receivedCents: number, upcomingItems: Upcoming[], monthEnd: string): number {
  const coming = upcomingItems.filter((i) => i.cents > 0 && i.date <= monthEnd).reduce((s, i) => s + i.cents, 0);
  return receivedCents + coming;
}
