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

  it("adds and removes your own check-ins", async () => {
    const calls: [string, unknown][] = [];
    let journal: unknown[] = [];
    const invoke = async (cmd: string, args?: Record<string, unknown>) => {
      calls.push([cmd, args]);
      if (cmd === "add_journal_entry")
        journal = [{ date: args!.date, note: args!.note, savingsCents: args!.savingsCents, projectedCents: 400_000, aheadCents: (args!.savingsCents as number) - 400_000 }];
      if (cmd === "remove_journal_entry") journal = [];
      return cmd === "progress" ? { ...report, journal } : undefined;
    };
    render(Progress, { invoke });
    await waitFor(() => screen.getByLabelText("Savings total"));
    await fireEvent.input(screen.getByLabelText("Savings total"), { target: { value: "5,000" } });
    await fireEvent.input(screen.getByLabelText("Check-in note"), { target: { value: "Brokerage" } });
    await fireEvent.click(screen.getByText("Add check-in"));
    await waitFor(() => screen.getByTestId("journal-row"));
    expect(screen.getByTestId("journal-row").textContent).toContain("ahead by $1,000");
    expect(calls.find(([c]) => c === "add_journal_entry")![1]).toMatchObject({ savingsCents: 500_000, note: "Brokerage" });
    await fireEvent.click(screen.getByText("Remove"));
    await waitFor(() => expect(screen.queryByTestId("journal-row")).toBeNull());
  });

  it("asks for a dollar amount before adding a check-in", async () => {
    render(Progress, { invoke: async (cmd: string) => (cmd === "progress" ? { ...report, journal: [] } : undefined) });
    await waitFor(() => screen.getByLabelText("Savings total"));
    await fireEvent.click(screen.getByText("Add check-in"));
    expect((await screen.findByRole("alert")).textContent).toContain("dollar amount");
  });
});
