import { render, screen, waitFor } from "@testing-library/svelte";
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
});
