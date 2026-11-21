import { afterEach, describe, expect, it } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import Explain from "./Explain.svelte";

afterEach(cleanup);

const facts = { successRate: 0.87, depletedYear: null, retireAge: 62, horizonAge: 95, compare: null };

describe("Explain", () => {
  it("sends only the facts and shows the text with where it came from", async () => {
    const calls: [string, Record<string, unknown> | undefined][] = [];
    const invoke = async (cmd: string, args?: Record<string, unknown>) => {
      calls.push([cmd, args]);
      return { text: "In 87 out of 100 futures your money lasts.", source: "built-in" };
    };
    render(Explain, { facts, invoke });
    expect(screen.queryByTestId("explanation")).toBeNull();
    await fireEvent.click(screen.getByRole("button", { name: "Explain in plain English" }));
    expect((await screen.findByTestId("explanation")).textContent).toMatch(/87 out of 100/);
    expect(screen.getByText(/Written by Golden/)).toBeTruthy();
    expect(calls).toEqual([["explain_result", { facts }]]);
  });
  it("says when the model wrote it", async () => {
    render(Explain, { facts, invoke: async () => ({ text: "Looks fine.", source: "model" }) });
    await fireEvent.click(screen.getByRole("button", { name: "Explain in plain English" }));
    expect(await screen.findByText(/model on this computer/)).toBeTruthy();
  });
  it("shows an error", async () => {
    render(Explain, { facts, invoke: async () => { throw new Error("busy"); } });
    await fireEvent.click(screen.getByRole("button", { name: "Explain in plain English" }));
    expect((await screen.findByRole("alert")).textContent).toBe("busy");
  });
});
