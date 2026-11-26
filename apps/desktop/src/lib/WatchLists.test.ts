import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import WatchLists from "./WatchLists.svelte";
import { watchTotals } from "./watchTotals";

afterEach(cleanup);

const today = new Date("2026-11-26T12:00:00Z");
const t = (id: number, day: string, payee: string, amountCents: number, category: string | null = "Food", tags: string[] = []) => ({ id, day, payee, memo: "", amountCents, category, account: "Checking", note: "", tags, splits: [] });
const rows = [
  t(1, "2026-11-03", "Corner Cafe", -450),
  t(2, "2026-11-10", "Starbucks", -600),
  t(3, "2026-10-20", "Corner Cafe", -300),
  t(4, "2026-09-02", "Corner Cafe", -999),
  t(5, "2026-11-12", "Cafe refund", 200),
  t(6, "2026-11-13", "Cafe transfer", -5000, "Transfer"),
  t(7, "2026-11-14", "Airline", -40000, "Travel", ["trip"]),
];

describe("watchTotals", () => {
  it("adds up spending that matches every word, this month and last, ignoring income, transfers and older months", () => {
    expect(watchTotals(rows as never, "cafe", today)).toEqual({ thisMonthCents: 450, lastMonthCents: 300, count: 2 });
  });
  it("finds lines by tag", () => {
    expect(watchTotals(rows as never, "trip", today).thisMonthCents).toBe(40000);
  });
});

describe("WatchLists", () => {
  it("shows each list with its totals", async () => {
    const invoke = vi.fn(async (c: string) => (c === "watch_lists" ? [{ name: "Trip", query: "trip" }] : c === "transactions" ? rows : undefined));
    render(WatchLists, { invoke, today });
    const row = await screen.findByTestId("watch-row");
    expect(row.textContent).toContain("Trip");
    expect(row.textContent).toContain("$400 this month");
  });

  it("adds and removes lists", async () => {
    const invoke = vi.fn(async (c: string, a?: unknown) => (c === "watch_lists" ? [] : c === "transactions" ? rows : c === "save_watch_lists" ? (a as { lists: unknown[] }).lists : undefined));
    render(WatchLists, { invoke, today });
    await fireEvent.input(await screen.findByLabelText("Watch list name"), { target: { value: "Coffee" } });
    await fireEvent.input(screen.getByLabelText("Watch list search"), { target: { value: "cafe" } });
    await fireEvent.click(screen.getByText("Add"));
    await waitFor(() => expect(invoke).toHaveBeenCalledWith("save_watch_lists", { lists: [{ name: "Coffee", query: "cafe" }] }));
    await screen.findByTestId("watch-row");
    await fireEvent.click(screen.getByLabelText("Remove Coffee"));
    await waitFor(() => expect(invoke).toHaveBeenCalledWith("save_watch_lists", { lists: [] }));
  });

  it("shows the reason when a list is refused", async () => {
    const invoke = async (c: string) => {
      if (c === "save_watch_lists") throw new Error("Give each watch list a name and something to look for.");
      return c === "watch_lists" ? [] : c === "transactions" ? rows : undefined;
    };
    render(WatchLists, { invoke, today });
    await fireEvent.click(await screen.findByText("Add"));
    await screen.findByText(/name and something/);
  });
});
