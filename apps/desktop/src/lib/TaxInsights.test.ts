import { cleanup, render, screen } from "@testing-library/svelte";
import { afterEach, describe, expect, it } from "vitest";
import TaxInsights from "./TaxInsights.svelte";
import { diversification, rateColour, taxSummary } from "./taxInsights";
import type { Household, QuickPlanResult, YearPoint } from "./api";

afterEach(cleanup);

const yr = (year: number, rate: number, agi: number, tax: number): YearPoint => ({
  year, age: year - 1981, balanceCents: 1, spendingCents: 1, taxesCents: tax, marginalRate: rate, agiCents: agi, incomeTaxCents: tax,
});
const years = [yr(2026, 0.24, 20_000_000, 3_000_000), yr(2027, 0.12, 8_000_000, 600_000), yr(2028, 0.24, 12_000_000, 1_400_000)];
const result: QuickPlanResult = {
  successRate: 0.9, depletedYear: null, p10Cents: 1, p50Cents: 2, p90Cents: 3, years,
  realReturn: 0.04, volatility: 0.1, horizonAge: 95, ssClaimAge: 67,
};
const h: Household = {
  people: [], streams: [], filing: "single", state: "TX", spendingCents: 1, cashCents: 100, taxableCents: 100,
  traditionalCents: 700, rothCents: 100, picks: {},
};

describe("taxSummary", () => {
  it("counts years per rate, lowest first, and the effective rate", () => {
    const s = taxSummary(years);
    expect(s.bands).toEqual([{ rate: 0.12, years: 1 }, { rate: 0.24, years: 2 }]);
    expect(s.lifetimeIncomeTaxCents).toBe(5_000_000);
    expect(s.effectiveRate).toBeCloseTo(5_000_000 / 40_000_000);
    expect(s.peakYear).toEqual({ year: 2026, rate: 0.24 });
  });
  it("copes with results that carry no tax data", () => {
    const s = taxSummary([{ year: 2026, age: 45, balanceCents: 1, spendingCents: 1, taxesCents: 1 }]);
    expect(s.bands).toEqual([]);
    expect(s.effectiveRate).toBeNull();
    expect(s.peakYear).toBeNull();
  });
});

describe("diversification and colour", () => {
  it("splits savings by tax treatment", () => {
    const d = diversification(h);
    expect(d.preTax).toBeCloseTo(0.7);
    expect(d.roth).toBeCloseTo(0.1);
    expect(diversification({ ...h, cashCents: 0, taxableCents: 0, traditionalCents: 0, rothCents: 0 }).total).toBe(0);
  });
  it("darkens with the rate", () => {
    const a = Number(/,\s*([\d.]+)\)$/.exec(rateColour(0.1))![1]);
    const b = Number(/,\s*([\d.]+)\)$/.exec(rateColour(0.35))![1]);
    expect(b).toBeGreaterThan(a);
  });
});

describe("TaxInsights", () => {
  it("shows the strip, the totals and the diversification line", () => {
    render(TaxInsights, { result, household: h });
    expect(screen.getByRole("img", { name: /Federal rate on the next dollar/ }).children).toHaveLength(3);
    expect(screen.getByTestId("effective").textContent).toContain("12.5%");
    expect(screen.getByTestId("diversification").textContent).toContain("70% pre-tax");
    expect(document.body.textContent).toContain("2 years at 24%");
  });
  it("shows nothing without tax data", () => {
    render(TaxInsights, { result: { ...result, years: [{ year: 2026, age: 45, balanceCents: 1, spendingCents: 1, taxesCents: 1 }] } });
    expect(screen.queryByLabelText("Tax insights")).toBeNull();
  });
});
