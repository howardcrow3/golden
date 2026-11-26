import { describe, expect, it } from "vitest";
import { daysLeftInMonth, monthEnd, safeToSpend } from "./spendToday";

describe("spendToday", () => {
  it("counts the days left including today", () => {
    expect(daysLeftInMonth("2026-11-26")).toBe(5);
    expect(daysLeftInMonth("2026-11-30")).toBe(1);
    expect(daysLeftInMonth("2026-12-01")).toBe(31);
    expect(daysLeftInMonth("2028-02-10")).toBe(20);
  });

  it("finds the month end, including leap years", () => {
    expect(monthEnd("2026-11-26")).toBe("2026-11-30");
    expect(monthEnd("2028-02-10")).toBe("2028-02-29");
    expect(monthEnd("2026-12-05")).toBe("2026-12-31");
  });

  it("spreads what is left, after bills, over the remaining days", () => {
    // $1,000 unspent, $250 of bills still due, 5 days left: $750 / 5 = $150 a day.
    const s = safeToSpend(100_000, 25_000, "2026-11-26");
    expect(s.leftCents).toBe(75_000);
    expect(s.perDayCents).toBe(15_000);
    expect(s.daysLeft).toBe(5);
    expect(s.over).toBe(false);
  });

  it("rounds the daily amount down to the cent", () => {
    expect(safeToSpend(10_000, 0, "2026-11-28").perDayCents).toBe(3_333);
  });

  it("goes to zero a day and flags it when bills outrun the budget", () => {
    const s = safeToSpend(10_000, 25_000, "2026-11-26");
    expect(s.leftCents).toBe(-15_000);
    expect(s.perDayCents).toBe(0);
    expect(s.over).toBe(true);
  });

  it("is zero, not over, when exactly spent", () => {
    const s = safeToSpend(25_000, 25_000, "2026-11-26");
    expect(s.perDayCents).toBe(0);
    expect(s.over).toBe(false);
  });
});
