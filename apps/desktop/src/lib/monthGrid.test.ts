import { describe, expect, it } from "vitest";
import { addMonths, calendarMonth, monthTitle } from "./monthGrid";

const rent = { date: "2026-11-28", payee: "Landlord", cents: -250_000 };
const pay = { date: "2026-11-30", payee: "Employer", cents: 500_000 };
const bonus = { date: "2026-11-30", payee: "Bonus", cents: 100_000 };
const later = { date: "2026-12-03", payee: "Gym", cents: -9_900 };

describe("calendarMonth", () => {
  it("lays November 2026 out in Sunday-first weeks with blank padding", () => {
    const weeks = calendarMonth([], 2026, 11);
    // Nov 1, 2026 is a Sunday; 30 days = 4 full weeks + 2 days.
    expect(weeks.length).toBe(5);
    expect(weeks.every((w) => w.length === 7)).toBe(true);
    expect(weeks[0][0].day).toBe(1);
    expect(weeks[4][1].day).toBe(30);
    expect(weeks[4][2].day).toBe(0);
  });

  it("pads the start of a month that begins midweek", () => {
    // Dec 1, 2026 is a Tuesday.
    const weeks = calendarMonth([], 2026, 12);
    expect(weeks[0].map((d) => d.day)).toEqual([0, 0, 1, 2, 3, 4, 5]);
    expect(weeks.flat().filter((d) => d.day > 0).length).toBe(31);
  });

  it("puts items on their day, sums the net, and leaves other months out", () => {
    const weeks = calendarMonth([rent, pay, bonus, later], 2026, 11).flat();
    const on = (d: number) => weeks.find((c) => c.day === d)!;
    expect(on(28).items.map((i) => i.payee)).toEqual(["Landlord"]);
    expect(on(30).items.length).toBe(2);
    expect(on(30).netCents).toBe(600_000);
    expect(on(28).netCents).toBe(-250_000);
    expect(weeks.flatMap((c) => c.items).some((i) => i.payee === "Gym")).toBe(false);
  });

  it("handles a leap February", () => {
    const days = calendarMonth([], 2028, 2).flat().filter((d) => d.day > 0);
    expect(days.length).toBe(29);
    expect(days[28].date).toBe("2028-02-29");
  });
});

describe("month helpers", () => {
  it("titles a month", () => {
    expect(monthTitle(2026, 11)).toBe("November 2026");
  });

  it("moves across year ends in both directions", () => {
    expect(addMonths(2026, 11, 1)).toEqual({ year: 2026, month: 12 });
    expect(addMonths(2026, 12, 1)).toEqual({ year: 2027, month: 1 });
    expect(addMonths(2026, 1, -1)).toEqual({ year: 2025, month: 12 });
    expect(addMonths(2026, 3, 0)).toEqual({ year: 2026, month: 3 });
  });
});
