import { afterEach, describe, expect, it } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import AssumptionSuggest from "./AssumptionSuggest.svelte";

afterEach(cleanup);

const list = [
  { key: "horizon_age", label: "Age 95", reason: "Planning further out covers a longer life.", source: "built-in" },
  { key: "real_return", label: "Cautious", reason: "A cautious return is safer for you.", source: "model" },
];

function fake(log: string[], result: unknown = list) {
  return async (cmd: string) => {
    log.push(cmd);
    return cmd === "suggest_assumptions" ? result : undefined;
  };
}

describe("AssumptionSuggest", () => {
  it("shows suggestions with who made them and changes nothing yet", async () => {
    const log: string[] = [];
    render(AssumptionSuggest, { invoke: fake(log) });
    await fireEvent.click(screen.getByRole("button", { name: "Suggest assumptions" }));
    expect((await screen.findAllByTestId("assumption-suggestion")).length).toBe(2);
    expect(screen.getByText(/suggested by the model/)).toBeTruthy();
    expect(screen.getByText(/suggested by Golden/)).toBeTruthy();
    expect(log).toEqual(["suggest_assumptions"]);
  });
  it("accept sets the pick and tells the page", async () => {
    const calls: [string, Record<string, unknown> | undefined][] = [];
    let changed = 0;
    const invoke = async (cmd: string, args?: Record<string, unknown>) => {
      calls.push([cmd, args]);
      return cmd === "suggest_assumptions" ? list : undefined;
    };
    render(AssumptionSuggest, { invoke, onchange: () => changed++ });
    await fireEvent.click(screen.getByRole("button", { name: "Suggest assumptions" }));
    await fireEvent.click(await screen.findByRole("button", { name: /Accept Age 95/ }));
    expect(calls.find(([c]) => c === "set_pick")![1]).toEqual({ key: "horizon_age", label: "Age 95" });
    expect(screen.getAllByTestId("assumption-suggestion").length).toBe(1);
    expect(changed).toBe(1);
  });
  it("reject drops it without saving", async () => {
    const log: string[] = [];
    render(AssumptionSuggest, { invoke: fake(log) });
    await fireEvent.click(screen.getByRole("button", { name: "Suggest assumptions" }));
    await fireEvent.click(await screen.findByRole("button", { name: /Reject Cautious/ }));
    expect(screen.getAllByTestId("assumption-suggestion").length).toBe(1);
    expect(log.includes("set_pick")).toBe(false);
  });
  it("says so when there is nothing to suggest", async () => {
    render(AssumptionSuggest, { invoke: fake([], []) });
    await fireEvent.click(screen.getByRole("button", { name: "Suggest assumptions" }));
    expect((await screen.findByRole("status")).textContent).toMatch(/Nothing to suggest/);
  });
});
