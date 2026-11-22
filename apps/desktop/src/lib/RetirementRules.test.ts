import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import { afterEach, describe, expect, it } from "vitest";
import Retirement from "./Retirement.svelte";
import type { Household } from "./api";
import { fromForm, toForm } from "./household";

const household: Household = {
  people: [{ name: "You", birthDate: "1981-06-15", retireAge: 62, ssAnnualCents: 3_800_000 }],
  streams: [{ label: "Salary", person: 0, annualCents: 18_000_000, startYear: null, endYear: null, contributionCents: 2_350_000 }],
  filing: "married_joint", state: "TX", spendingCents: 9_000_000, cashCents: 3_000_000,
  taxableCents: 15_000_000, traditionalCents: 60_000_000, rothCents: 12_000_000, picks: {},
  events: [{ label: "New roof", year: 2028, cents: 2_500_000 }],
  spendingRule: "fixed", withdrawalOrder: "standard", medicare: true,
};
const planResult = {
  successRate: 0.87, depletedYear: null, p10Cents: 1, p50Cents: 2, p90Cents: 3,
  years: [{ year: 2026, age: 45, balanceCents: 100, spendingCents: 1, taxesCents: 1 }, { year: 2027, age: 46, balanceCents: 200, spendingCents: 1, taxesCents: 1 }],
  realReturn: 0.04, volatility: 0.11, horizonAge: 95, ssClaimAge: 67,
};

describe("retirement spending rule and withdrawal order", () => {
  afterEach(cleanup);

  it("round-trips through the form, keeping planned events", () => {
    const back = fromForm(toForm(household));
    expect(back.events).toEqual(household.events);
    expect(back.spendingRule).toBe("fixed");
    expect(back.withdrawalOrder).toBe("standard");
    // A household saved before these existed gets the defaults.
    const old = { ...household, events: undefined, spendingRule: undefined, withdrawalOrder: undefined, medicare: undefined };
    const f = fromForm(toForm(old as unknown as Household));
    expect(f.events).toEqual([]);
    expect(f.spendingRule).toBe("fixed");
    expect(f.medicare).toBe(true);
  });

  it("saves the chosen rule and order, and does not drop planned events", async () => {
    const calls: [string, Record<string, unknown> | undefined][] = [];
    const invoke = async (cmd: string, args?: Record<string, unknown>) => {
      calls.push([cmd, args]);
      if (cmd === "household") return household;
      if (cmd === "assumptions") return [];
      if (cmd === "run_plan") return planResult;
      return undefined;
    };
    render(Retirement, { invoke });
    await fireEvent.change(await screen.findByLabelText("Spending rule"), { target: { value: "guardrails" } });
    await fireEvent.change(screen.getByLabelText("Withdrawal order"), { target: { value: "roth_first" } });
    await fireEvent.click(screen.getByRole("button", { name: "Save and run plan" }));
    await screen.findByTestId("success");
    const saved = (calls.find(([c]) => c === "save_household")![1] as { household: Household }).household;
    expect(saved.spendingRule).toBe("guardrails");
    expect(saved.withdrawalOrder).toBe("roth_first");
    expect(saved.events).toEqual(household.events);
  });
});
