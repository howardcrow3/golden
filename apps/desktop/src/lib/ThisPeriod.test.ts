import { afterEach, describe, expect, it } from "vitest";
import { cleanup, render, screen } from "@testing-library/svelte";
import ThisPeriod from "./ThisPeriod.svelte";

afterEach(cleanup);

const row = (period: string, income: number, spending: number, count = 3) => ({
  period,
  incomeCents: income,
  spendingCents: spending,
  netCents: income - spending,
  count,
});
const today = new Date("2026-11-15T00:00:00Z");

describe("ThisPeriod", () => {
  it("shows the latest period with activity and the share of income kept", async () => {
    const invoke = async () => [row("2026-10", 500_000, 400_000), row("2026-11", 0, 0, 0)];
    render(ThisPeriod, { grain: "month", today, invoke });
    expect((await screen.findByTestId("tp-income")).textContent).toBe("$5,000");
    expect(screen.getByTestId("tp-spending").textContent).toBe("$4,000");
    expect(screen.getByTestId("tp-net").textContent).toBe("$1,000");
    expect(screen.getByTestId("tp-rate").textContent).toBe("You kept 20% of what came in.");
  });
  it("says so when spending beat income", async () => {
    const invoke = async () => [row("2026-11", 100_000, 150_000)];
    render(ThisPeriod, { grain: "month", today, invoke });
    expect((await screen.findByTestId("tp-rate")).textContent).toBe("You spent 50% more than came in.");
  });
  it("shows nothing before any statement is imported or outside the app", async () => {
    render(ThisPeriod, { grain: "month", today, invoke: async () => [] });
    await new Promise((r) => setTimeout(r, 10));
    expect(screen.queryByTestId("this-period")).toBeNull();
    cleanup();
    render(ThisPeriod, { grain: "month", today, invoke: async () => { throw new Error("no"); } });
    await new Promise((r) => setTimeout(r, 10));
    expect(screen.queryByTestId("this-period")).toBeNull();
  });
  it("omits the rate when there was no income", async () => {
    render(ThisPeriod, { grain: "week", today, invoke: async () => [row("2026-W46", 0, 5_000)] });
    await screen.findByTestId("this-period");
    expect(screen.queryByTestId("tp-rate")).toBeNull();
  });
});
