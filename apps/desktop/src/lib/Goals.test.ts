import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import { afterEach, describe, expect, it } from "vitest";
import Goals from "./Goals.svelte";

const row = {
  name: "Trip",
  targetCents: 500000,
  savedCents: 100000,
  byDate: "2027-06-01",
  monthsLeft: 7,
  monthlyNeededCents: 57143,
  percent: 20,
  status: "on_track",
  typicalMonthlyCents: 200000,
};

function fake(calls: [string, unknown][], rows: unknown[] = [row]) {
  return async (c: string, a?: unknown) => {
    calls.push([c, a]);
    return c === "goals" ? rows : null;
  };
}

describe("Goals", () => {
  afterEach(cleanup);

  it("shows progress, the monthly need and where it stands", async () => {
    render(Goals, { invoke: fake([]) });
    await waitFor(() => expect(screen.getByTestId("goal-row")).toBeTruthy());
    const t = screen.getByTestId("goal-row").textContent!;
    expect(t).toContain("Trip");
    expect(t).toContain("$571");
    expect(t).toContain("Within reach");
    expect(screen.getByRole("progressbar").getAttribute("aria-valuenow")).toBe("20");
  });

  it("adds a goal in cents", async () => {
    const calls: [string, unknown][] = [];
    render(Goals, { invoke: fake(calls, []) });
    await waitFor(() => screen.getByLabelText("Goal name"));
    await fireEvent.input(screen.getByLabelText("Goal name"), { target: { value: " Car " } });
    await fireEvent.input(screen.getByLabelText("Goal target"), { target: { value: "12,000" } });
    await fireEvent.input(screen.getByLabelText("Goal date"), { target: { value: "2028-01-01" } });
    await fireEvent.click(screen.getByText("Add goal"));
    await waitFor(() => expect(calls.some(([c]) => c === "set_goal")).toBe(true));
    expect(calls.find(([c]) => c === "set_goal")![1]).toEqual({
      goal: { name: "Car", targetCents: 1200000, savedCents: 0, byDate: "2028-01-01" },
    });
  });

  it("explains bad input without calling the app", async () => {
    const calls: [string, unknown][] = [];
    render(Goals, { invoke: fake(calls, []) });
    await waitFor(() => screen.getByLabelText("Goal name"));
    await fireEvent.click(screen.getByText("Add goal"));
    expect(screen.getByRole("alert").textContent).toContain("name");
    await fireEvent.input(screen.getByLabelText("Goal name"), { target: { value: "X" } });
    await fireEvent.input(screen.getByLabelText("Goal target"), { target: { value: "100" } });
    await fireEvent.input(screen.getByLabelText("Goal date"), { target: { value: "soon" } });
    await fireEvent.click(screen.getByText("Add goal"));
    expect(screen.getByRole("alert").textContent).toContain("YYYY-MM-DD");
    expect(calls.some(([c]) => c === "set_goal")).toBe(false);
  });

  it("removes a goal", async () => {
    const calls: [string, unknown][] = [];
    render(Goals, { invoke: fake(calls) });
    await waitFor(() => screen.getByLabelText("Remove Trip"));
    await fireEvent.click(screen.getByLabelText("Remove Trip"));
    await waitFor(() => expect(calls.some(([c]) => c === "remove_goal")).toBe(true));
    expect(calls.find(([c]) => c === "remove_goal")![1]).toEqual({ name: "Trip" });
  });

  it("is hidden in a plain browser", async () => {
    render(Goals, {});
    await new Promise((r) => setTimeout(r, 20));
    expect(screen.queryByTestId("goals")).toBeNull();
  });
});
