import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import GettingStarted from "./GettingStarted.svelte";

afterEach(cleanup);

const steps = (doneKeys: string[]) =>
  [
    { key: "import", title: "Import a bank or card statement", page: "Import" },
    { key: "budget", title: "Set a monthly budget for a category", page: "Overview" },
  ].map((s) => ({ ...s, done: doneKeys.includes(s.key) }));
const reply = (doneKeys: string[]) => async () => ({ steps: steps(doneKeys), done: doneKeys.length, complete: doneKeys.length === 2 });

describe("GettingStarted", () => {
  it("shows progress and the steps still to do as links", async () => {
    render(GettingStarted, { invoke: reply(["import"]) });
    expect((await screen.findByText("Getting started · 1 of 2")).tagName).toBe("H2");
    expect(screen.queryByRole("button", { name: "Import a bank or card statement" })).toBeNull();
    expect(screen.getByText("Import a bank or card statement")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Set a monthly budget for a category" })).toBeTruthy();
  });
  it("opens the page that does a step", async () => {
    const onnavigate = vi.fn();
    render(GettingStarted, { invoke: reply([]), onnavigate });
    await fireEvent.click(await screen.findByRole("button", { name: "Import a bank or card statement" }));
    expect(onnavigate).toHaveBeenCalledWith("Import");
  });
  it("disappears when everything is done or outside the app", async () => {
    render(GettingStarted, { invoke: reply(["import", "budget"]) });
    await new Promise((r) => setTimeout(r, 10));
    expect(screen.queryByTestId("getting-started")).toBeNull();
    cleanup();
    render(GettingStarted, {});
    await new Promise((r) => setTimeout(r, 10));
    expect(screen.queryByTestId("getting-started")).toBeNull();
  });
});
