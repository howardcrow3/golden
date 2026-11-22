import { afterEach, describe, expect, it } from "vitest";
import { cleanup, render, screen } from "@testing-library/svelte";
import Alerts from "./Alerts.svelte";
import type { QuickPlanResult } from "./api";

afterEach(cleanup);

const base = { successRate: 1, depletedYear: null, p10Cents: 0, p50Cents: 0, p90Cents: 0, years: [], realReturn: 0.03, volatility: 0.1, horizonAge: 95, ssClaimAge: 67 } as QuickPlanResult;

describe("Alerts", () => {
  it("lists each alert in the order given", () => {
    render(Alerts, { result: { ...base, alerts: [
      { key: "off_track", severity: "danger", message: "You are off track: the plan works in only 40% of simulated futures." },
      { key: "medicare", severity: "info", message: "Medicare starts next year (2027)." },
    ] } });
    const items = screen.getAllByRole("listitem");
    expect(items).toHaveLength(2);
    expect(items[0].textContent).toContain("off track");
    expect(items[0].className).toContain("danger");
    expect(screen.getByTestId("alert-medicare").textContent).toContain("next year");
  });

  it("shows nothing without alerts or for an older result", () => {
    const a = render(Alerts, { result: { ...base, alerts: [] } });
    expect(a.container.querySelector("section")).toBeNull();
    cleanup();
    const b = render(Alerts, { result: base });
    expect(b.container.querySelector("section")).toBeNull();
  });
});
