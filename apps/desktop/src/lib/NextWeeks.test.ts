import { cleanup, render, screen, waitFor } from "@testing-library/svelte";
import { afterEach, describe, expect, it } from "vitest";
import NextWeeks from "./NextWeeks.svelte";

const row = (payee: string, cadence: string, typicalCents: number, nextExpected: string) => ({
  payee, cadence, typicalCents, timesSeen: 6, lastSeen: "2026-02-01", nextExpected, yearlyCents: typicalCents * 12,
});

describe("NextWeeks", () => {
  afterEach(cleanup);

  it("lists dated items with in and out totals", async () => {
    const invoke = async () => [row("Pay", "monthly", 400000, "2026-03-15"), row("Rent", "monthly", -150000, "2026-03-03")];
    render(NextWeeks, { invoke, today: new Date("2026-03-01T12:00:00Z") });
    await waitFor(() => screen.getByTestId("next-weeks"));
    const rows = screen.getAllByTestId("next-weeks-row");
    expect(rows[0].textContent).toContain("Rent");
    expect(screen.getByTestId("next-weeks-total").textContent).toContain("$12,000");
    expect(screen.getByTestId("next-weeks-total").textContent).toContain("$4,500");
  });

  it("shows nothing when nothing repeats or the call fails", async () => {
    const a = render(NextWeeks, { invoke: async () => [] });
    await new Promise((r) => setTimeout(r, 20));
    expect(a.container.querySelector("section")).toBeNull();
    cleanup();
    const b = render(NextWeeks, { invoke: async () => { throw new Error("x"); } });
    await new Promise((r) => setTimeout(r, 20));
    expect(b.container.querySelector("section")).toBeNull();
  });
});
