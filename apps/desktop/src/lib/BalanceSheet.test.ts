import { fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import { afterEach, describe, expect, it } from "vitest";
import { cleanup } from "@testing-library/svelte";
import BalanceSheet from "./BalanceSheet.svelte";

const sheet = {
  assets: [{ name: "Checking", kind: "checking", cents: 200000 }],
  debts: [{ name: "Visa", kind: "credit card", cents: 30000 }],
  assetsCents: 200000,
  debtsCents: 30000,
  netWorthCents: 170000,
};

const accountsRows = [
  { id: 1, name: "Checking", kind: "checking", balanceCents: 200000, openingCents: 0 },
  { id: 2, name: "Visa", kind: "credit", balanceCents: -30000, openingCents: 0 },
];

const history = [
  { month: "2026-09", netWorthCents: 100000 },
  { month: "2026-10", netWorthCents: 150000 },
  { month: "2026-11", netWorthCents: 170000 },
];

function fake(calls: [string, unknown][]) {
  return async (cmd: string, args?: unknown) => {
    calls.push([cmd, args]);
    if (cmd === "accounts") return accountsRows;
    if (cmd === "set_opening_balance") return null;
    if (cmd === "net_worth_history") return history;
    return sheet;
  };
}

describe("BalanceSheet", () => {
  afterEach(cleanup);
  it("shows net worth, what you have and what you owe", async () => {
    render(BalanceSheet, { invoke: async () => sheet });
    await waitFor(() => expect(screen.getByTestId("net-worth")).toBeTruthy());
    expect(screen.getByTestId("net-worth").textContent).toContain("1,700");
    expect(screen.getAllByTestId("asset-row")).toHaveLength(1);
    expect(screen.getAllByTestId("debt-row")[0].textContent).toContain("Visa");
  });

  it("says nothing is owed when there are no debts", async () => {
    render(BalanceSheet, { invoke: async () => ({ ...sheet, debts: [], debtsCents: 0, netWorthCents: 200000 }) });
    await waitFor(() => expect(screen.getByText("Nothing owed.")).toBeTruthy());
  });

  it("is hidden without accounts", async () => {
    render(BalanceSheet, { invoke: async () => ({ ...sheet, assets: [], debts: [] }) });
    await new Promise((r) => setTimeout(r, 20));
    expect(screen.queryByTestId("balance-sheet")).toBeNull();
  });

  it("is hidden in a plain browser", async () => {
    render(BalanceSheet, {});
    await new Promise((r) => setTimeout(r, 20));
    expect(screen.queryByTestId("balance-sheet")).toBeNull();
  });

  it("saves only the starting balances that changed", async () => {
    const calls: [string, unknown][] = [];
    render(BalanceSheet, { invoke: fake(calls) });
    await waitFor(() => screen.getByText("Set starting balances"));
    await fireEvent.click(screen.getByText("Set starting balances"));
    await fireEvent.input(screen.getByLabelText("Starting balance for Checking"), { target: { value: "10000" } });
    await fireEvent.click(screen.getByText("Save starting balances"));
    await waitFor(() => expect(screen.queryByTestId("opening-editor")).toBeNull());
    const sets = calls.filter(([c]) => c === "set_opening_balance");
    expect(sets).toEqual([["set_opening_balance", { accountId: 1, cents: 1000000 }]]);
  });

  it("explains a bad starting balance and keeps the editor open", async () => {
    const calls: [string, unknown][] = [];
    render(BalanceSheet, { invoke: fake(calls) });
    await waitFor(() => screen.getByText("Set starting balances"));
    await fireEvent.click(screen.getByText("Set starting balances"));
    await fireEvent.input(screen.getByLabelText("Starting balance for Visa"), { target: { value: "lots" } });
    await fireEvent.click(screen.getByText("Save starting balances"));
    await waitFor(() => expect(screen.getByRole("alert").textContent).toContain("Visa"));
    expect(screen.getByTestId("opening-editor")).toBeTruthy();
    expect(calls.some(([c]) => c === "set_opening_balance")).toBe(false);
  });

  it("draws a month-by-month trend when there is history", async () => {
    render(BalanceSheet, { invoke: fake([]) });
    await waitFor(() => expect(screen.getByTestId("net-worth-trend")).toBeTruthy());
    expect(screen.getByTestId("net-worth-trend").getAttribute("aria-label")).toContain("2026-09 to 2026-11");
  });
});
