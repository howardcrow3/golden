import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import { afterEach, describe, expect, it } from "vitest";
import Allocate from "./Allocate.svelte";

const result = {
  yearlySavingsCents: 2000000,
  steps: [
    { name: "Emergency cash", cents: 1500000, why: "Cash first." },
    { name: "IRA", cents: 500000, why: "More room." },
  ],
};

describe("Allocate", () => {
  afterEach(cleanup);

  it("asks the app and lists the steps in order", async () => {
    const seen: unknown[] = [];
    const invoke = async (cmd: string, args?: unknown) => {
      seen.push([cmd, args]);
      return result;
    };
    render(Allocate, { invoke });
    await fireEvent.input(screen.getByLabelText("Yearly savings"), { target: { value: "20000" } });
    await fireEvent.click(screen.getByText("Suggest"));
    await waitFor(() => expect(screen.getByTestId("allocation-steps")).toBeTruthy());
    expect(seen).toEqual([["suggest_allocation", { yearlySavingsCents: 2000000 }]]);
    const items = screen.getAllByRole("listitem");
    expect(items[0].textContent).toContain("Emergency cash");
    expect(items[1].textContent).toContain("IRA");
  });

  it("rejects text that isn't an amount without asking the app", async () => {
    const seen: string[] = [];
    render(Allocate, { invoke: async (c: string) => (seen.push(c), result) });
    await fireEvent.input(screen.getByLabelText("Yearly savings"), { target: { value: "lots" } });
    await fireEvent.click(screen.getByText("Suggest"));
    expect(screen.getByRole("alert").textContent).toContain("how much");
    expect(seen).toEqual([]);
  });

  it("shows the app's refusal", async () => {
    render(Allocate, { invoke: async () => { throw new Error("Savings can't be negative."); } });
    await fireEvent.input(screen.getByLabelText("Yearly savings"), { target: { value: "5" } });
    await fireEvent.click(screen.getByText("Suggest"));
    await waitFor(() => expect(screen.getByRole("alert").textContent).toContain("negative"));
  });

  it("says so when there is nothing to place", async () => {
    render(Allocate, { invoke: async () => ({ yearlySavingsCents: 0, steps: [] }) });
    await fireEvent.input(screen.getByLabelText("Yearly savings"), { target: { value: "0" } });
    await fireEvent.click(screen.getByText("Suggest"));
    await waitFor(() => expect(screen.getByText("Nothing to place yet.")).toBeTruthy());
  });
});
