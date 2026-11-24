import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { cleanup, render, screen, waitFor } from "@testing-library/svelte";
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

  describe("notifications", () => {
    beforeEach(() => localStorage.clear());

    it("raises one generic OS notification for new warnings, never the message text", async () => {
      const calls: [string, unknown][] = [];
      const invoke = async (c: string, a?: unknown) => { calls.push([c, a]); return null; };
      const result = { ...base, alerts: [{ key: "off_track", severity: "danger", message: "You are off track by $12,345." }] } as QuickPlanResult;
      render(Alerts, { result, invoke });
      await waitFor(() => expect(calls).toEqual([["os_notify", { count: 1 }]]));
      cleanup();
      render(Alerts, { result, invoke });
      await new Promise((r) => setTimeout(r, 20));
      expect(calls).toHaveLength(1);
    });

    it("stays quiet for good news", async () => {
      const calls: string[] = [];
      render(Alerts, { result: { ...base, alerts: [{ key: "ok", severity: "good", message: "On track." }] } as QuickPlanResult, invoke: async (c: string) => { calls.push(c); return null; } });
      await new Promise((r) => setTimeout(r, 20));
      expect(calls).toEqual([]);
    });
  });
});
