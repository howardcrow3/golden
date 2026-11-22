import { afterEach, describe, expect, it } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import Scenarios from "./Scenarios.svelte";

afterEach(cleanup);

const list = [
  { id: 1, name: "Retire at 62", changes: [], descriptions: ["Retire at 62"] },
  { id: 2, name: "Retire at 67", changes: [], descriptions: ["Retire at 67"] },
  { id: 3, name: "Spend less", changes: [], descriptions: ["Spend $60,000 a year"] },
  { id: 4, name: "Move", changes: [], descriptions: ["Move to FL"] },
];
const result = (rate: number) => ({ successRate: rate, depletedYear: null, p10Cents: 100000000, p50Cents: 500000000, p90Cents: 900000000, years: [], realReturn: 0.04, volatility: 0.1, horizonAge: 95, ssClaimAge: 67 });

function fake(calls: [string, Record<string, unknown> | undefined][], undo: string | null = null) {
  return async (c: string, a?: Record<string, unknown>) => {
    calls.push([c, a]);
    if (c === "list_scenarios") return list;
    if (c === "undoable_plan") return undo;
    if (c === "compare_scenarios")
      return { columns: [{ name: "Current plan", descriptions: [], result: result(0.8) }, { name: "Retire at 62", descriptions: [], result: result(0.6) }] };
    return {};
  };
}

describe("Scenarios", () => {
  it("lists saved scenarios and an empty state", async () => {
    const calls: [string, Record<string, unknown> | undefined][] = [];
    render(Scenarios, { invoke: fake(calls) });
    expect((await screen.findAllByTestId("scenario")).length).toBe(4);
    cleanup();
    render(Scenarios, { invoke: async (c: string) => (c === "list_scenarios" ? [] : null) });
    expect(await screen.findByText(/Nothing saved yet/)).toBeTruthy();
  });
  it("compares at most three and shows the table", async () => {
    const calls: [string, Record<string, unknown> | undefined][] = [];
    render(Scenarios, { invoke: fake(calls) });
    await screen.findAllByTestId("scenario");
    for (const n of ["Retire at 62", "Retire at 67", "Spend less"]) await fireEvent.click(screen.getByLabelText(`Compare ${n}`));
    expect((screen.getByLabelText("Compare Move") as HTMLInputElement).disabled).toBe(true);
    await fireEvent.click(screen.getByRole("button", { name: /Compare with my current plan/ }));
    const table = await screen.findByTestId("compare");
    expect(table.textContent).toMatch(/Current plan.*Retire at 62/);
    expect(table.textContent).toMatch(/80%/);
    expect(table.textContent).toMatch(/60%/);
    expect(calls.find(([c]) => c === "compare_scenarios")?.[1]).toEqual({ ids: [1, 2, 3], runs: 400 });
  });
  it("compare is disabled until one is chosen", async () => {
    render(Scenarios, { invoke: fake([]) });
    await screen.findAllByTestId("scenario");
    expect((screen.getByRole("button", { name: /Compare with my current plan/ }) as HTMLButtonElement).disabled).toBe(true);
  });
  it("makes a scenario the plan and tells the page", async () => {
    const calls: [string, Record<string, unknown> | undefined][] = [];
    let changed = 0;
    render(Scenarios, { invoke: fake(calls), onchange: () => changed++ });
    await screen.findAllByTestId("scenario");
    await fireEvent.click(screen.getAllByRole("button", { name: "Make this my plan" })[0]);
    expect((await screen.findByTestId("scenario-note")).textContent).toMatch(/Retire at 62.*now your plan/);
    expect(calls.find(([c]) => c === "make_plan")?.[1]).toEqual({ id: 1 });
    await waitFor(() => expect(changed).toBe(1));
  });
  it("offers undo after a plan was made and runs it", async () => {
    const calls: [string, Record<string, unknown> | undefined][] = [];
    render(Scenarios, { invoke: fake(calls, "Retire at 62") });
    await fireEvent.click(await screen.findByRole("button", { name: "Undo" }));
    expect((await screen.findByTestId("scenario-note")).textContent).toMatch(/back the way it was/);
    expect(calls.some(([c]) => c === "undo_make_plan")).toBe(true);
  });
  it("deletes and shows errors", async () => {
    const calls: [string, Record<string, unknown> | undefined][] = [];
    render(Scenarios, { invoke: fake(calls) });
    await screen.findAllByTestId("scenario");
    await fireEvent.click(screen.getByRole("button", { name: "Delete Move" }));
    expect(calls.find(([c]) => c === "delete_scenario")?.[1]).toEqual({ id: 4 });
    cleanup();
    render(Scenarios, { invoke: async (c: string) => { if (c === "list_scenarios") throw new Error("locked"); return null; } });
    expect((await screen.findByRole("alert")).textContent).toMatch(/locked/);
  });

  it("renames, duplicates and moves scenarios", async () => {
    const calls: [string, Record<string, unknown> | undefined][] = [];
    render(Scenarios, { invoke: fake(calls) });
    await screen.findAllByTestId("scenario");
    await fireEvent.click(screen.getByRole("button", { name: "Rename Retire at 67" }));
    const box = screen.getByLabelText("New name for Retire at 67") as HTMLInputElement;
    await fireEvent.input(box, { target: { value: "Later" } });
    await fireEvent.click(screen.getByRole("button", { name: "Save name" }));
    await waitFor(() => expect(calls.find(([c]) => c === "rename_scenario")?.[1]).toEqual({ id: 2, name: "Later" }));
    await fireEvent.click(screen.getByRole("button", { name: "Duplicate Spend less" }));
    await waitFor(() => expect(calls.find(([c]) => c === "duplicate_scenario")?.[1]).toEqual({ id: 3 }));
    await fireEvent.click(screen.getByRole("button", { name: "Move Spend less up" }));
    await fireEvent.click(screen.getByRole("button", { name: "Move Retire at 62 down" }));
    await waitFor(() => expect(calls.filter(([c]) => c === "move_scenario").map(([, a]) => a)).toEqual([{ id: 3, by: -1 }, { id: 1, by: 1 }]));
  });
  it("overlays the compared plans on one chart", async () => {
    const withYears = (rate: number) => ({ ...result(rate), years: [
      { year: 2026, age: 45, balanceCents: 1, spendingCents: 1, taxesCents: 1, p50Cents: 100 },
      { year: 2027, age: 46, balanceCents: 2, spendingCents: 1, taxesCents: 1, p50Cents: 300 },
    ] });
    const invoke = async (c: string) => {
      if (c === "list_scenarios") return list;
      if (c === "undoable_plan") return null;
      if (c === "compare_scenarios") return { columns: [{ name: "Current plan", descriptions: [], result: withYears(0.8) }, { name: "Retire at 62", descriptions: [], result: withYears(0.6) }] };
      return {};
    };
    render(Scenarios, { invoke });
    await screen.findAllByTestId("scenario");
    await fireEvent.click(screen.getByLabelText("Compare Retire at 62"));
    await fireEvent.click(screen.getByRole("button", { name: /Compare with my current plan/ }));
    const chart = await screen.findByTestId("overlay");
    expect(chart.querySelectorAll("polyline")).toHaveLength(2);
  });
});
