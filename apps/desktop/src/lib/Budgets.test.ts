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

  it("turns rollover on for a category and shows what carried in", async () => {
    const rolled = { ...food, rolloverCents: 5_000, rolls: true, remainingCents: 32_450 };
    const invoke = vi.fn(async (cmd: string) => (cmd === "budget_status" ? [rolled] : cmd === "category_choices" ? ["Food"] : undefined));
    render(Budgets, { grain: "month", today, invoke });
    expect((await screen.findByText(/rolled over/)).textContent).toContain("$50");
    await fireEvent.click(screen.getByLabelText("Roll over Food"));
    await waitFor(() => expect(invoke).toHaveBeenCalledWith("set_budget_rollover", { category: "Food", on: false }));
  });

  it("offers no rollover in the year view", async () => {
    const invoke = vi.fn(async (cmd: string) => (cmd === "budget_status" ? [food] : cmd === "category_choices" ? ["Food"] : undefined));
    render(Budgets, { grain: "year", today, invoke });
    await screen.findByTestId("budget-row");
    expect(screen.queryByLabelText("Roll over Food")).toBeNull();
  });

  it("shows what is left of this month's income after the category budgets", async () => {
    const rent = { category: "Rent", budgetCents: 150_000, spentCents: 0, remainingCents: 150_000 };
    const invoke = vi.fn(async (cmd: string) => (cmd === "budget_status" ? [food, rent] : cmd === "categories" ? [{ category: "Salary", cents: 500_000 }] : cmd === "category_choices" ? ["Food"] : undefined));
    render(Budgets, { grain: "month", today, invoke });
    const note = await screen.findByTestId("left-to-assign");
    // $5,000 in, $400 + $1,500 assigned.
    expect(note.textContent).toContain("$3,100");
  });

  it("warns when the budgets are more than the income", async () => {
    const invoke = vi.fn(async (cmd: string) => (cmd === "budget_status" ? [food] : cmd === "categories" ? [{ category: "Salary", cents: 10_000 }] : cmd === "category_choices" ? ["Food"] : undefined));
    render(Budgets, { grain: "month", today, invoke });
    expect((await screen.findByTestId("left-to-assign")).textContent).toContain("$300 more");
  });
});
