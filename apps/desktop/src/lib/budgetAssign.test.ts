import { describe, expect, it } from "vitest";
import { leftToAssign } from "./budgetAssign";
import type { BudgetRow } from "./api";

const row = (category: string, budgetCents: number) => ({ category, budgetCents, spentCents: 0, remainingCents: budgetCents }) as BudgetRow;

describe("leftToAssign", () => {
  it("ignores the overall row and subtracts every category budget from income", () => {
    const r = leftToAssign(500_000, [row("Total", 400_000), row("Rent", 200_000), row("Food", 50_000)]);
    expect(r).toEqual({ assignedCents: 250_000, leftCents: 250_000 });
  });
  it("goes negative when more is assigned than comes in", () => {
    expect(leftToAssign(100_000, [row("Rent", 150_000)]).leftCents).toBe(-50_000);
  });
  it("is the whole income with no budgets", () => {
    expect(leftToAssign(100_000, []).leftCents).toBe(100_000);
  });
});
