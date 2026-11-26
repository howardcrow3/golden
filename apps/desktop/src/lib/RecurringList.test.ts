import { cleanup, render, screen, waitFor } from "@testing-library/svelte";
import { afterEach, describe, expect, it, vi } from "vitest";
import { fireEvent } from "@testing-library/svelte";
import RecurringList from "./RecurringList.svelte";

const rows = [
  { payee: "RENT PAYMENT", cadence: "monthly", typicalCents: -150000, timesSeen: 6, lastSeen: "2026-11-01", nextExpected: "2026-12-01", yearlyCents: -1800000 },
  { payee: "NETFLIX", cadence: "monthly", typicalCents: -1549, timesSeen: 6, lastSeen: "2026-11-14", nextExpected: "2026-12-14", yearlyCents: -18588 },
  { payee: "ACME PAYROLL", cadence: "monthly", typicalCents: 500000, timesSeen: 6, lastSeen: "2026-11-01", nextExpected: "2026-12-01", yearlyCents: 6000000 },
];

describe("RecurringList", () => {
  afterEach(cleanup);

  it("lists what repeats and totals only the money going out", async () => {
    const seen: unknown[] = [];
    render(RecurringList, {
      invoke: async (c: string, a?: unknown) => (c === "cancel_marks" ? [] : (seen.push([c, a]), rows)),
      today: new Date("2026-11-16T12:00:00Z"),
    });
    await waitFor(() => expect(screen.getAllByTestId("recurring-row")).toHaveLength(3));
    expect(seen).toEqual([["recurring", { from: "2025-11-16", to: "2026-11-16" }]]);
    // 18,000.00 + 185.88 = 18,185.88 out per year; the paycheck is not counted.
    expect(screen.getByTestId("recurring-total").textContent).toContain("18,186");
  });

  it("is hidden when nothing repeats or in a plain browser", async () => {
    render(RecurringList, { invoke: async () => [] });
    await new Promise((r) => setTimeout(r, 20));
    expect(screen.queryByTestId("recurring")).toBeNull();
    cleanup();
    render(RecurringList, {});
    await new Promise((r) => setTimeout(r, 20));
    expect(screen.queryByTestId("recurring")).toBeNull();
  });

  it("shows the yearly cost and what marking payments to cancel would save", async () => {
    const invoke = vi.fn(async (c: string) => (c === "cancel_marks" ? [] : c === "set_cancel_mark" ? ["NETFLIX"] : rows));
    render(RecurringList, { invoke, today: new Date("2026-11-16T12:00:00Z") });
    await waitFor(() => expect(screen.getAllByTestId("recurring-row")).toHaveLength(3));
    expect(screen.queryByTestId("recurring-savings")).toBeNull();
    expect(screen.queryByLabelText("Mark ACME PAYROLL to cancel")).toBeNull(); // income can't be cancelled
    await fireEvent.click(screen.getByLabelText("Mark NETFLIX to cancel"));
    await waitFor(() => expect(invoke).toHaveBeenCalledWith("set_cancel_mark", { payee: "NETFLIX", on: true }));
    await waitFor(() => expect(screen.getByTestId("recurring-savings").textContent).toContain("$186"));
  });

  it("remembers marks from earlier", async () => {
    const invoke = async (c: string) => (c === "cancel_marks" ? ["NETFLIX"] : rows);
    render(RecurringList, { invoke, today: new Date("2026-11-16T12:00:00Z") });
    await waitFor(() => expect(screen.getByTestId("recurring-savings")).toBeTruthy());
    expect((screen.getByLabelText("Mark NETFLIX to cancel") as HTMLInputElement).checked).toBe(true);
  });
});
