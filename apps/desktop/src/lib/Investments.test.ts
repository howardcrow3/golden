import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import Investments from "./Investments.svelte";

afterEach(cleanup);

const checkup = {
  totalCents: 10_000_000,
  mix: [{ class: "stock", share: 0.7, target: 0.6 }, { class: "bond", share: 0.3, target: 0.35 }, { class: "cash", share: 0, target: 0.05 }, { class: "other", share: 0, target: 0 }],
  stockGap: 0.1, weightedFee: 0.0012, yearlyFeeCents: 12_000, feeCost20yCents: 250_000,
  flags: [{ name: "Employer stock", share: 0.08, reason: "One company's stock above 5% of the portfolio" }],
};

function mock(saved: unknown[] = []) {
  const calls: [string, unknown][] = [];
  const invoke = vi.fn(async (cmd: string, args?: unknown) => {
    calls.push([cmd, args]);
    if (cmd === "holdings") return saved;
    if (cmd === "investment_checkup") return checkup;
    return undefined;
  });
  return { invoke, calls };
}

describe("Investments", () => {
  it("loads saved holdings, saves edits and shows the check-up", async () => {
    const { invoke, calls } = mock([{ name: "Total market", valueCents: 7_000_000, class: "stock", expenseBp: 3, singleStock: false }]);
    render(Investments, { invoke });
    await waitFor(() => expect(screen.getAllByTestId("holding-row")).toHaveLength(1));
    await fireEvent.click(screen.getByText("Check my investments"));
    await waitFor(() => screen.getByTestId("inv-gap"));
    const save = calls.find(([c]) => c === "save_holdings")![1] as { holdings: unknown[] };
    expect(save.holdings[0]).toMatchObject({ name: "Total market", valueCents: 7_000_000, expenseBp: 3 });
    expect(calls.find(([c]) => c === "investment_checkup")![1]).toEqual({ risk: "moderate" });
    expect(screen.getByTestId("inv-gap").textContent).toContain("10% more in stocks");
    expect(screen.getByTestId("inv-fees").textContent).toContain("0.12%");
    expect(screen.getAllByTestId("inv-flag")).toHaveLength(1);
    expect(screen.getAllByTestId("inv-mix")).toHaveLength(3);
  });

  it("adds and removes rows, and rejects a bad value before saving", async () => {
    const { invoke, calls } = mock();
    render(Investments, { invoke });
    await fireEvent.click(screen.getByText("Add a holding"));
    expect(screen.getAllByTestId("holding-row")).toHaveLength(1);
    await fireEvent.input(screen.getByLabelText("Holding 1 value"), { target: { value: "lots" } });
    await fireEvent.click(screen.getByText("Check my investments"));
    expect((await screen.findByRole("alert")).textContent).toContain("dollar value");
    expect(calls.some(([c]) => c === "save_holdings")).toBe(false);
    await fireEvent.click(screen.getByText("Remove"));
    expect(screen.queryAllByTestId("holding-row")).toHaveLength(0);
  });

  it("shows an engine error such as an empty list", async () => {
    const invoke = vi.fn(async (cmd: string) => {
      if (cmd === "investment_checkup") throw new Error("Add at least one holding.");
      return cmd === "holdings" ? [] : undefined;
    });
    render(Investments, { invoke });
    await fireEvent.click(screen.getByText("Check my investments"));
    expect((await screen.findByRole("alert")).textContent).toContain("at least one");
  });

  it("adds holdings read from a positions file", async () => {
    const found = [{ name: "Total Stock Market Index", valueCents: 5_000_000, class: "stock", expenseBp: 3, singleStock: false }];
    const calls: [string, unknown][] = [];
    const invoke = vi.fn(async (cmd: string, args?: unknown) => {
      calls.push([cmd, args]);
      if (cmd === "holdings") return [];
      if (cmd === "parse_holdings") return found;
      return undefined;
    });
    render(Investments, { invoke });
    const file = new File(["Name,Value\nTotal Stock Market Index,50000\n"], "positions.csv", { type: "text/csv" });
    await fireEvent.change(screen.getByTestId("inv-import"), { target: { files: [file] } });
    await waitFor(() => expect(screen.getAllByTestId("holding-row")).toHaveLength(1));
    expect(screen.getByTestId("inv-imported").textContent).toContain("Added 1 holding from positions.csv");
    expect((calls.find(([c]) => c === "parse_holdings")![1] as { text: string }).text).toContain("Total Stock Market Index");
  });

  it("shows why a positions file could not be read", async () => {
    const invoke = vi.fn(async (cmd: string) => {
      if (cmd === "holdings") return [];
      if (cmd === "parse_holdings") throw new Error("No holdings with a value were found in that file.");
      return undefined;
    });
    render(Investments, { invoke });
    await fireEvent.change(screen.getByTestId("inv-import"), { target: { files: [new File(["x"], "bad.csv")] } });
    expect((await screen.findByRole("alert")).textContent).toContain("No holdings");
  });
});
