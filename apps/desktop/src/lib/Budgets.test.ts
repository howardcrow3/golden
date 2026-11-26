import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import Budgets from "./Budgets.svelte";
import { budgetCsv } from "./report";

afterEach(cleanup);

const food = { category: "Food", budgetCents: 40_000, spentCents: 12_550, remainingCents: 27_450 };
const today = new Date("2026-11-26T12:00:00Z");

describe("budget export", () => {
  it("writes plain dollars with a header and defuses formulas", () => {
    const csv = budgetCsv([food, { ...food, category: "=bad", spentCents: 0, remainingCents: 40_000 }], "month of 2026-11-26");
    const lines = csv.trim().split("\n");
    expect(lines[0]).toBe("Period,Category,Budget,Spent,Remaining");
    expect(lines[1]).toBe("month of 2026-11-26,Food,400,125.5,274.5");
    expect(lines[2]).toContain("'=bad");
  });

  it("saves the CSV from the button", async () => {
    const invoke = vi.fn(async (cmd: string, args?: unknown) =>
      cmd === "budget_status" ? [food] : cmd === "category_choices" ? ["Food"] : cmd === "save_report" ? `/x/${(args as { stem: string }).stem}.csv` : undefined,
    );
    render(Budgets, { grain: "month", today, invoke });
    await fireEvent.click(await screen.findByText("Export budget as CSV"));
    await waitFor(() => expect(screen.getByTestId("budget-saved").textContent).toContain(".csv"));
    const call = invoke.mock.calls.find((c) => c[0] === "save_report")!;
    expect((call[1] as { kind: string; content: string }).kind).toBe("csv");
    expect((call[1] as { content: string }).content).toContain("Food,400,125.5,274.5");
  });
});
