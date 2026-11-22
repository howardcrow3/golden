import { afterEach, describe, expect, it } from "vitest";
import { cleanup, render, screen } from "@testing-library/svelte";
import Legacy from "./Legacy.svelte";
import type { QuickPlanResult } from "./api";

afterEach(cleanup);

const base = { successRate: 1, depletedYear: null, p10Cents: 0, p50Cents: 0, p90Cents: 0, years: [], realReturn: 0.03, volatility: 0.1, horizonAge: 95, ssClaimAge: 67 } as QuickPlanResult;
const legacy = { grossCents: 500_000_00, exemptionCents: 30_000_000_00, estateTaxCents: 0, inheritedIraPerYearCents: 20_000_00, heirIncomeTaxCents: 48_000_00, netToHeirsCents: 452_000_00 };

describe("Legacy", () => {
  it("shows the estate, no tax under the exemption, and the net", () => {
    render(Legacy, { result: { ...base, legacy } });
    expect(screen.getByTestId("legacy-gross").textContent).toContain("500,000");
    expect(screen.getByTestId("legacy-estate-tax").textContent).toContain("None");
    expect(screen.getByTestId("legacy-net").textContent).toContain("452,000");
    expect(screen.getByTestId("legacy-ira").textContent).toContain("20,000");
  });
  it("shows the tax when the estate is above the exemption", () => {
    render(Legacy, { result: { ...base, legacy: { ...legacy, estateTaxCents: 4_000_000_00 } } });
    expect(screen.getByTestId("legacy-estate-tax").textContent).not.toContain("None");
  });
  it("hides when there is nothing left or an older result lacks it", () => {
    const { container } = render(Legacy, { result: base });
    expect(container.querySelector("section")).toBeNull();
    cleanup();
    const r = render(Legacy, { result: { ...base, legacy: { ...legacy, grossCents: 0 } } });
    expect(r.container.querySelector("section")).toBeNull();
  });
});
