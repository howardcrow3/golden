import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import { afterEach, describe, expect, it } from "vitest";
import MonthClose from "./MonthClose.svelte";

const report = (over = {}) => ({
  month: "2026-10",
  closed: false,
  finished: true,
  incomeCents: 500000,
  spendingCents: 100000,
  netCents: 400000,
  count: 7,
  issues: [{ kind: "uncategorized", text: "1 transaction still need a category." }],
  summary: "October 2026: $5,000 came in, $1,000 went out, and you kept $4,000.",
  ...over,
});

describe("MonthClose", () => {
  afterEach(cleanup);

  it("shows the summary and issues, then closes and reopens", async () => {
    const calls: string[] = [];
    const invoke = async (cmd: string) => {
      calls.push(cmd);
      if (cmd === "close_month") return report({ closed: true });
      if (cmd === "reopen_month") return report();
      return report();
    };
    render(MonthClose, { invoke, month: "2026-10" });
    expect(await screen.findByText(/you kept \$4,000/)).toBeTruthy();
    expect(screen.getByText(/still need a category/)).toBeTruthy();
    await fireEvent.click(screen.getByRole("button", { name: "Close this month" }));
    await waitFor(() => expect(screen.getByText("Closed")).toBeTruthy());
    await fireEvent.click(screen.getByRole("button", { name: "Reopen this month" }));
    await waitFor(() => expect(screen.getByText("Open")).toBeTruthy());
    expect(calls).toContain("close_month");
    expect(calls).toContain("reopen_month");
  });

  it("offers no close for an unfinished month and shows errors", async () => {
    const invoke = async (cmd: string) => {
      if (cmd === "close_month") throw new Error("That month isn't over yet.");
      return report({ finished: false, issues: [] });
    };
    render(MonthClose, { invoke, month: "2026-11" });
    await screen.findByTestId("month-close");
    expect(screen.queryByRole("button", { name: "Close this month" })).toBeNull();
  });

  it("is hidden with no data or in a plain browser", async () => {
    render(MonthClose, { invoke: async () => report({ count: 0, issues: [] }) });
    await new Promise((r) => setTimeout(r, 20));
    expect(screen.queryByTestId("month-close")).toBeNull();
    cleanup();
    render(MonthClose, {});
    await new Promise((r) => setTimeout(r, 20));
    expect(screen.queryByTestId("month-close")).toBeNull();
  });
});
