import { describe, expect, it } from "vitest";
import { biggest, upcoming } from "./upcoming";
import type { Recurring } from "./api";

const r = (payee: string, cadence: string, cents: number, next: string): Recurring => ({
  payee, cadence, typicalCents: cents, timesSeen: 6, lastSeen: "2026-01-01", nextExpected: next, yearlyCents: cents,
});

describe("upcoming", () => {
  it("repeats a weekly payment through the window", () => {
    const u = upcoming([r("Gym", "weekly", -1000, "2026-03-02")], "2026-03-01", 4);
    expect(u.map((x) => x.date)).toEqual(["2026-03-02", "2026-03-09", "2026-03-16", "2026-03-23"]);
  });

  it("orders items from different payees by date", () => {
    const u = upcoming([r("Rent", "monthly", -150000, "2026-03-20"), r("Pay", "every two weeks", 200000, "2026-03-05")], "2026-03-01", 4);
    expect(u.map((x) => x.payee)).toEqual(["Pay", "Pay", "Rent"]);
  });

  it("shows a slightly late payment today and skips a long-stale one's past dates", () => {
    expect(upcoming([r("Bill", "monthly", -5000, "2026-02-27")], "2026-03-01", 1)[0].date).toBe("2026-03-01");
    const stale = upcoming([r("Old", "weekly", -100, "2025-01-01")], "2026-03-01", 1);
    expect(stale.every((x) => x.date >= "2026-03-01")).toBe(true);
  });

  it("ignores unknown cadences, bad dates and zero amounts", () => {
    expect(upcoming([r("A", "sometimes", -1, "2026-03-02"), r("B", "weekly", -1, "nope"), r("C", "weekly", 0, "2026-03-02")], "2026-03-01")).toEqual([]);
  });

  it("keeps the largest items, in date order", () => {
    const u = upcoming([r("Big", "monthly", -90000, "2026-03-10"), r("Small", "weekly", -100, "2026-03-02")], "2026-03-01", 2);
    const b = biggest(u, 1);
    expect(b).toHaveLength(1);
    expect(b[0].payee).toBe("Big");
  });
});
