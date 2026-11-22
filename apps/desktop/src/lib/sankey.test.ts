import { describe, expect, it } from "vitest";
import type { YearPoint } from "./api";
import { layoutFlow, yearFlow } from "./sankey";

const y = (o: Partial<YearPoint> = {}): YearPoint => ({
  year: 2040, age: 64, balanceCents: 0, spendingCents: 8_000_000, taxesCents: 2_000_000,
  earnedCents: 12_000_000, socialSecurityCents: 0, withdrawalsCents: 0, rmdCents: 0, medicareCents: 0, ...o,
});

describe("yearFlow", () => {
  it("splits a working year into spending, tax and savings", () => {
    const f = yearFlow(y());
    expect(f.sources.map((s) => s.key)).toEqual(["earned"]);
    expect(f.sinks.map((s) => [s.key, s.cents])).toEqual([["living", 8_000_000], ["taxes", 2_000_000], ["saved", 2_000_000]]);
    expect(f.total).toBe(12_000_000);
  });
  it("shows withdrawals and Social Security in retirement, with no savings", () => {
    const f = yearFlow(y({ earnedCents: 0, socialSecurityCents: 3_000_000, withdrawalsCents: 7_000_000, rmdCents: 0, medicareCents: 500_000 }));
    expect(f.sources.map((s) => s.key)).toEqual(["ss", "withdrawals"]);
    expect(f.sinks.find((s) => s.key === "medicare")?.cents).toBe(500_000);
    expect(f.sinks.find((s) => s.key === "living")?.cents).toBe(7_500_000);
    expect(f.sinks.some((s) => s.key === "saved")).toBe(false);
  });
  it("copes with an older result that has no source fields", () => {
    const f = yearFlow({ year: 2030, age: 50, balanceCents: 0, spendingCents: 100, taxesCents: 0 });
    expect(f.sources).toEqual([]);
  });
});

describe("layoutFlow", () => {
  it("makes a ribbon for every source and use, whose sizes add back up", () => {
    const f = yearFlow(y({ socialSecurityCents: 1_000_000 }));
    const l = layoutFlow(f, 400, 200);
    expect(l.left).toHaveLength(2);
    expect(l.right).toHaveLength(3);
    expect(l.ribbons).toHaveLength(6);
    const total = l.ribbons.reduce((s, r) => s + r.cents, 0);
    expect(Math.abs(total - 13_000_000)).toBeLessThan(10);
    expect(l.ribbons.every((r) => r.path.startsWith("M"))).toBe(true);
  });
  it("keeps boxes inside the height", () => {
    const l = layoutFlow(yearFlow(y()), 400, 200);
    for (const b of [...l.left, ...l.right]) expect(b.y + b.h).toBeLessThanOrEqual(200.01);
  });
  it("handles an empty year", () => {
    const l = layoutFlow(yearFlow({ year: 1, age: 1, balanceCents: 0, spendingCents: 0, taxesCents: 0 }), 400, 200);
    expect(l.ribbons).toEqual([]);
  });
});
