import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import { afterEach, describe, expect, it } from "vitest";
import Progress from "./Progress.svelte";
import { pairPoints } from "./chart";

const report = {
  started: "2026-09-15",
  aheadCents: 120_000,
  points: [
    { month: "2026-09", actualChangeCents: 0, projectedChangeCents: 0 },
    { month: "2026-10", actualChangeCents: 350_000, projectedChangeCents: 300_000 },
    { month: "2026-11", actualChangeCents: 850_000, projectedChangeCents: 730_000 },
  ],
};

describe("Progress", () => {
  afterEach(cleanup);

  it("offers to start tracking, then shows the comparison", async () => {
    let state: unknown = null;
    const calls: string[] = [];
    const invoke = async (cmd: string) => {
      calls.push(cmd);
      if (cmd === "start_progress") state = report;
      if (cmd === "stop_progress") state = null;
      return cmd === "progress" ? state : undefined;
    };
    render(Progress, { invoke });
    await waitFor(() => screen.getByText("Start tracking from today"));
    await fireEvent.click(screen.getByText("Start tracking from today"));
    await waitFor(() => screen.getByTestId("progress-summary"));
    expect(screen.getByTestId("progress-summary").textContent).toContain("beaten");
    expect(screen.getByTestId("progress-summary").textContent).toContain("$1,200");
    expect(screen.getByTestId("actual-line")).toBeTruthy();
    await fireEvent.click(screen.getByText("Stop tracking"));
    await waitFor(() => screen.getByText("Start tracking from today"));
    expect(calls).toContain("stop_progress");
  });

  it("says trailed when behind and waits for a second month", async () => {
    const behind = { ...report, aheadCents: -50_000, points: report.points.slice(0, 1) };
    render(Progress, { invoke: async () => behind });
    await waitFor(() => screen.getByTestId("progress-summary"));
    expect(screen.getByTestId("progress-summary").textContent).toContain("trailed");
    expect(screen.queryByTestId("actual-line")).toBeNull();
  });
});

describe("pairPoints", () => {
  it("draws both series on one scale", () => {
    const [a, b] = pairPoints([0, 10], [0, 20], 100, 100, 0);
    expect(a).toBe("0.0,100.0 100.0,50.0");
    expect(b).toBe("0.0,100.0 100.0,0.0");
  });
  it("handles nothing", () => {
    expect(pairPoints([], [], 10, 10)).toEqual(["", ""]);
  });
});
