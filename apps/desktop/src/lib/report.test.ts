import { describe, expect, it } from "vitest";
import { csvCell, planCsv, planHtml, planJson } from "./report";
import type { QuickPlanResult } from "./api";

const r: QuickPlanResult = {
  successRate: 0.87, depletedYear: 2071, p10Cents: 1_000_000, p50Cents: 5_000_000, p90Cents: 9_000_000,
  years: [
    { year: 2026, age: 45, balanceCents: 123_456_00, spendingCents: 9_000_000, taxesCents: 1_500_050 },
    { year: 2027, age: 46, balanceCents: 130_000_00, spendingCents: 9_000_000, taxesCents: 1_600_000 },
  ],
  realReturn: 0.045, volatility: 0.12, horizonAge: 95, ssClaimAge: 67,
};

describe("report builders", () => {
  it("writes one CSV row per year in dollars", () => {
    const lines = planCsv(r).trim().split("\n");
    expect(lines[0]).toBe("Year,Age,Balance,Spending,Taxes");
    expect(lines).toHaveLength(3);
    expect(lines[1]).toBe("2026,45,123456,90000,15000.5");
  });
  it("defuses spreadsheet formulas and quotes commas", () => {
    expect(csvCell("=SUM(A1)")).toBe("'=SUM(A1)");
    expect(csvCell("a,b")).toBe('"a,b"');
    expect(csvCell('say "hi"')).toBe('"say ""hi"""');
    expect(csvCell(-5)).toBe("-5");
  });
  it("round-trips the data as JSON with a title", () => {
    const j = JSON.parse(planJson(r, "Plan", "Nov 22, 2026"));
    expect(j.title).toBe("Plan");
    expect(j.years).toHaveLength(2);
    expect(j.successRate).toBe(0.87);
  });
  it("builds a self-contained, escaped HTML page", () => {
    const h = planHtml(r, "Smith <plan>", "today");
    expect(h).toContain("<title>Smith &lt;plan&gt;</title>");
    expect(h).toContain("87%");
    expect(h).toContain("runs out in 2071");
    expect(h).toContain("<td>2027</td>");
    expect(h).not.toMatch(/https?:\/\//);
    expect(h).not.toContain("<script");
  });
});
