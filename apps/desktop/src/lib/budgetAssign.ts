import type { BudgetRow } from "./api";

/** What this month's income leaves unassigned once every category budget is taken out. Negative means over-assigned. */
export function leftToAssign(incomeCents: number, rows: BudgetRow[]): { assignedCents: number; leftCents: number } {
  const assignedCents = rows.filter((r) => r.category !== "Total").reduce((s, r) => s + r.budgetCents, 0);
  return { assignedCents, leftCents: incomeCents - assignedCents };
}
