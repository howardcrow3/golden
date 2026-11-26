import { describe, expect, it } from "vitest";
import { expectedIncome, leftToAssign } from "./budgetAssign";
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

describe("expectedIncome", () => {
  const item = (date: string, cents: number) => ({ date, payee: "x", cents });
  it("adds pay still to come this month to what has arrived", () => {
    expect(expectedIncome(0, [item("2026-03-14", 200_000), item("2026-03-28", 200_000)], "2026-03-31")).toBe(400_000);
  });
  it("ignores bills and pay after the month ends", () => {
    expect(expectedIncome(100_000, [item("2026-03-10", -50_000), item("2026-04-02", 200_000)], "2026-03-31")).toBe(100_000);
  });
  it("is what has arrived when nothing recurs", () => {
    expect(expectedIncome(123_000, [], "2026-03-31")).toBe(123_000);
  });
});
