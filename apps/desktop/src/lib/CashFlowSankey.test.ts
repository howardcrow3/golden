import { afterEach, describe, expect, it } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import CashFlowSankey from "./CashFlowSankey.svelte";
import type { QuickPlanResult, YearPoint } from "./api";

afterEach(cleanup);

const yr = (year: number, age: number, o: Partial<YearPoint>): YearPoint => ({ year, age, balanceCents: 0, spendingCents: 5_000_000, taxesCents: 1_000_000, earnedCents: 0, socialSecurityCents: 0, withdrawalsCents: 0, rmdCents: 0, medicareCents: 0, ...o });
const result = { successRate: 1, depletedYear: null, p10Cents: 0, p50Cents: 0, p90Cents: 0, realReturn: 0.03, volatility: 0.1, horizonAge: 95, ssClaimAge: 67,
  years: [yr(2030, 60, { earnedCents: 9_000_000 }), yr(2031, 61, { withdrawalsCents: 6_000_000 })] } as QuickPlanResult;

describe("CashFlowSankey", () => {
  it("shows the money in and out for the first year", () => {
    render(CashFlowSankey, { result });
    expect(screen.getByTestId("flow-year").textContent).toBe("2030");
    expect(screen.getByTestId("flow-in").textContent).toContain("Wages and business income");
    expect(screen.getByTestId("flow-out").textContent).toContain("Taxes");
  });
  it("moves to another year with the slider", async () => {
    render(CashFlowSankey, { result });
    await fireEvent.input(screen.getByLabelText("Year to show"), { target: { value: "1" } });
    expect(screen.getByTestId("flow-year").textContent).toBe("2031");
    expect(screen.getByTestId("flow-in").textContent).toContain("Withdrawals from savings");
  });
  it("stays hidden for results without the breakdown", () => {
    render(CashFlowSankey, { result: { ...result, years: [{ year: 1, age: 1, balanceCents: 0, spendingCents: 0, taxesCents: 0 }] } });
    expect(screen.queryByLabelText("Where the money goes")).toBeNull();
  });
});
