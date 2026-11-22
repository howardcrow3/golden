import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import { afterEach, describe, expect, it } from "vitest";
import PlanReport from "./PlanReport.svelte";
import type { QuickPlanResult } from "./api";

const result: QuickPlanResult = {
  successRate: 0.9, depletedYear: null, p10Cents: 1, p50Cents: 2, p90Cents: 3,
  years: [{ year: 2026, age: 45, balanceCents: 100_00, spendingCents: 50_00, taxesCents: 10_00 }],
  realReturn: 0.04, volatility: 0.1, horizonAge: 95, ssClaimAge: 67,
};

afterEach(cleanup);

describe("PlanReport", () => {
  it("saves each format through the app and shows where it went", async () => {
    const calls: Record<string, unknown>[] = [];
    const invoke = async (cmd: string, args?: Record<string, unknown>) => {
      expect(cmd).toBe("save_report");
      calls.push(args!);
      return `/Exports/file.${args!.kind}`;
    };
    render(PlanReport, { invoke, result });
    await fireEvent.click(screen.getByRole("button", { name: "Save report" }));
    expect((await screen.findByRole("status")).textContent).toContain("Saved to /Exports/file.html");
    await fireEvent.click(screen.getByRole("button", { name: "Save table (CSV)" }));
    await fireEvent.click(screen.getByRole("button", { name: "Save data (JSON)" }));
    await screen.findByText("Saved to /Exports/file.json");
    expect(calls.map((c) => c.kind)).toEqual(["html", "csv", "json"]);
    expect(String(calls[1].content)).toContain("Year,Age,Balance");
    expect(String(calls[0].content)).toContain("<!doctype html>");
  });
  it("shows an error when saving fails", async () => {
    const invoke = async () => { throw new Error("Disk full"); };
    render(PlanReport, { invoke, result });
    await fireEvent.click(screen.getByRole("button", { name: "Save report" }));
    expect((await screen.findByRole("alert")).textContent).toContain("Disk full");
  });
});
