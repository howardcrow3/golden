import { afterEach, describe, expect, it } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import AiActivity from "./AiActivity.svelte";

afterEach(cleanup);

const rows = [
  { at: "2026-11-21 06:00:00", task: "explain", ok: true, millis: 1500, promptChars: 400, replyChars: 120, error: null },
  { at: "2026-11-21 05:59:00", task: "categorize", ok: false, millis: 300, promptChars: 90, replyChars: 0, error: "model crashed" },
];

function open() {
  const el = document.querySelector("details")!;
  el.open = true;
  el.dispatchEvent(new Event("toggle"));
}

describe("AiActivity", () => {
  it("loads nothing until opened", () => {
    const log: string[] = [];
    render(AiActivity, { invoke: async (c: string) => (log.push(c), []) });
    expect(log).toEqual([]);
  });
  it("lists runs with their outcome", async () => {
    render(AiActivity, { invoke: async () => rows });
    open();
    const found = await screen.findAllByTestId("activity-row");
    expect(found.length).toBe(2);
    expect(found[0].textContent).toMatch(/Explain results, worked, 1\.5s, 400 characters in, 120 out/);
    expect(found[1].textContent).toMatch(/Categorize transactions, failed.*model crashed/);
  });
  it("says so when the model has not run, and clears", async () => {
    const calls: string[] = [];
    let data = rows;
    render(AiActivity, {
      invoke: async (c: string) => {
        calls.push(c);
        if (c === "clear_ai_activity") data = [];
        return c === "ai_activity" ? data : undefined;
      },
    });
    open();
    await screen.findAllByTestId("activity-row");
    await fireEvent.click(screen.getByRole("button", { name: "Clear" }));
    expect(await screen.findByText(/hasn't run yet/)).toBeTruthy();
    expect(calls).toContain("clear_ai_activity");
  });
  it("shows an error", async () => {
    render(AiActivity, { invoke: async () => { throw new Error("locked"); } });
    open();
    expect((await screen.findByRole("alert")).textContent).toMatch(/locked/);
  });
});
