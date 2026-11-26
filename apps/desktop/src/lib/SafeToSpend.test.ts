import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen, waitFor } from "@testing-library/svelte";
import SafeToSpend from "./SafeToSpend.svelte";

afterEach(cleanup);

const today = new Date("2026-11-26T12:00:00Z");
const total = { category: "Total", budgetCents: 300_000, spentCents: 200_000, remainingCents: 100_000 };
const rent = { payee: "Landlord", cadence: "monthly", typicalCents: -25_000, timesSeen: 6, lastSeen: "2026-10-28", nextExpected: "2026-11-28", yearlyCents: -300_000 };
const pay = { payee: "Employer", cadence: "monthly", typicalCents: 500_000, timesSeen: 6, lastSeen: "2026-10-31", nextExpected: "2026-11-30", yearlyCents: 6_000_000 };
const nextMonth = { ...rent, payee: "Gym", typicalCents: -9_900, nextExpected: "2026-12-03" };

function mock(rows: unknown[], recurring: unknown[]) {
  return vi.fn(async (cmd: string) => (cmd === "budget_status" ? rows : cmd === "recurring" ? recurring : undefined));
}

describe("SafeToSpend", () => {
  it("shows a daily amount after bills due this month, ignoring income and later months", async () => {
    render(SafeToSpend, { invoke: mock([total], [rent, pay, nextMonth]), today });
    await waitFor(() => screen.getByTestId("safe-to-spend-today"));
    // $1,000 left - $250 rent = $750 over 5 days = $150.
    expect(screen.getByTestId("safe-to-spend-today").textContent).toContain("$150");
    expect(screen.getByTestId("safe-to-spend").textContent).toContain("5 days");
  });

  it("says so when bills are more than what is left", async () => {
    const tight = { ...total, remainingCents: 10_000 };
    render(SafeToSpend, { invoke: mock([tight], [rent]), today });
    await waitFor(() => screen.getByTestId("safe-to-spend-over"));
    expect(screen.getByTestId("safe-to-spend-over").textContent).toContain("$150");
  });

  it("points to Budgets when there is no overall budget", async () => {
    const category = { category: "Food", budgetCents: 50_000, spentCents: 0, remainingCents: 50_000 };
    render(SafeToSpend, { invoke: mock([category], []), today });
    await waitFor(() => screen.getByTestId("safe-to-spend-hint"));
    expect(screen.getByTestId("safe-to-spend-hint").textContent).toContain("overall monthly budget");
  });

  it("shows nothing when the data can't be read", async () => {
    const invoke = vi.fn(async () => {
      throw new Error("locked");
    });
    render(SafeToSpend, { invoke, today });
    await new Promise((r) => setTimeout(r, 20));
    expect(screen.queryByTestId("safe-to-spend")).toBeNull();
    expect(screen.queryByTestId("safe-to-spend-hint")).toBeNull();
  });
});
