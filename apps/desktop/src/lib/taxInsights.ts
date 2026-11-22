import type { Household, YearPoint } from "./api";

export interface RateBand {
  rate: number;
  years: number;
}

export interface TaxSummary {
  /** Years in each federal rate on the next dollar, lowest rate first. */
  bands: RateBand[];
  /** Income tax over income on the return across the plan, or null with no income. */
  effectiveRate: number | null;
  lifetimeIncomeTaxCents: number;
  /** The year with the highest rate on the next dollar (first such year). */
  peakYear: { year: number; rate: number } | null;
}

export function taxSummary(years: YearPoint[]): TaxSummary {
  const counted = years.filter((y) => y.marginalRate !== undefined);
  const byRate = new Map<number, number>();
  for (const y of counted) {
    const r = Math.round(y.marginalRate! * 100) / 100;
    byRate.set(r, (byRate.get(r) ?? 0) + 1);
  }
  const bands = [...byRate.entries()].sort((a, b) => a[0] - b[0]).map(([rate, n]) => ({ rate, years: n }));
  const tax = counted.reduce((s, y) => s + (y.incomeTaxCents ?? 0), 0);
  const income = counted.reduce((s, y) => s + (y.agiCents ?? 0), 0);
  let peak: TaxSummary["peakYear"] = null;
  for (const y of counted) {
    if (!peak || y.marginalRate! > peak.rate) peak = { year: y.year, rate: y.marginalRate! };
  }
  return { bands, effectiveRate: income > 0 ? tax / income : null, lifetimeIncomeTaxCents: tax, peakYear: peak };
}

export interface Diversification {
  taxable: number;
  preTax: number;
  roth: number;
  cash: number;
  total: number;
}

/** Share of savings in each tax treatment today. */
export function diversification(h: Household): Diversification {
  const total = h.cashCents + h.taxableCents + h.traditionalCents + h.rothCents;
  const share = (c: number) => (total > 0 ? c / total : 0);
  return { cash: share(h.cashCents), taxable: share(h.taxableCents), preTax: share(h.traditionalCents), roth: share(h.rothCents), total };
}

/** A colour from pale to strong for a federal rate between 0 and 37%. */
export function rateColour(rate: number): string {
  const t = Math.min(1, Math.max(0, rate / 0.37));
  const alpha = 0.15 + 0.85 * t;
  return `rgba(190, 120, 20, ${alpha.toFixed(2)})`;
}
