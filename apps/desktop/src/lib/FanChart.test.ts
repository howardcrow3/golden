import { cleanup, render, screen } from "@testing-library/svelte";
import { afterEach, describe, expect, it, vi } from "vitest";
import FanChart from "./FanChart.svelte";
import { bandPath } from "./chart";
import type { QuickPlanResult } from "./api";

afterEach(cleanup);

const base: QuickPlanResult = {
  successRate: 0.8, depletedYear: null, p10Cents: 1, p50Cents: 2, p90Cents: 3,
  years: [
    { year: 2026, age: 45, balanceCents: 500, spendingCents: 1, taxesCents: 1, p10Cents: 300, p50Cents: 500, p90Cents: 800 },
    { year: 2027, age: 46, balanceCents: 600, spendingCents: 1, taxesCents: 1, p10Cents: 200, p50Cents: 600, p90Cents: 1100 },
  ],
  histogram: Array.from({ length: 10 }, (_, k) => ({ fromCents: k * 100, toCents: (k + 1) * 100, count: k === 0 ? 20 : 8 })),
  runs: 100, seed: 20260101, realReturn: 0.04, volatility: 0.1, horizonAge: 95, ssClaimAge: 67,
};

describe("bandPath", () => {
  it("closes a shape between two series", () => {
    const p = bandPath([1, 2], [3, 4], 100, 50);
    expect(p.startsWith("M")).toBe(true);
    expect(p.endsWith("Z")).toBe(true);
    expect(p.match(/L/g)).toHaveLength(3);
  });
  it("is empty with no data", () => {
    expect(bandPath([], [], 100, 50)).toBe("");
  });
});

describe("FanChart", () => {
  it("hands the band, median and histogram to the chart kit and says how it was made", () => {
    const line = vi.fn();
    const columns = vi.fn();
    (globalThis as { TBChart?: unknown }).TBChart = { line, columns };
    render(FanChart, { result: base });
    const l = line.mock.calls[0][1];
    expect(l.series[0].values).toEqual([5, 6]);
    expect(l.bands[0].lo).toEqual([3, 2]);
    expect(l.bands[0].hi).toEqual([8, 11]);
    expect(columns.mock.calls[0][1].series[0].values[0]).toBe(20);
    expect(document.body.textContent).toContain("100 simulated futures, seed 20260101");
    delete (globalThis as { TBChart?: unknown }).TBChart;
  });
  it("draws nothing for results without percentile data", () => {
    const plain = { ...base, histogram: undefined, years: base.years.map((y) => ({ ...y, p10Cents: undefined, p50Cents: undefined, p90Cents: undefined })) };
    render(FanChart, { result: plain });
    expect(screen.queryByTestId("fan-chart")).toBeNull();
  });
});
