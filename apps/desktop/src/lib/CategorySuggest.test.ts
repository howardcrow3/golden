import { afterEach, describe, expect, it } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import CategorySuggest from "./CategorySuggest.svelte";

afterEach(cleanup);

const two = [
  { id: 1, payee: "JOES TAQUERIA", category: "Dining out" },
  { id: 2, payee: "SHELL", category: "Transport" },
];

function fake(log: [string, Record<string, unknown> | undefined][], list: unknown = two, fail = "") {
  return async (cmd: string, args?: Record<string, unknown>) => {
    log.push([cmd, args]);
    if (cmd === "suggest_categories") {
      if (fail) throw new Error(fail);
      return list;
    }
    return { category: String(args?.category), alsoUpdated: 0 };
  };
}

describe("CategorySuggest", () => {
  it("shows suggestions and changes nothing until accepted", async () => {
    const log: [string, Record<string, unknown> | undefined][] = [];
    render(CategorySuggest, { invoke: fake(log) });
    await fireEvent.click(screen.getByRole("button", { name: /Suggest categories/ }));
    expect((await screen.findAllByTestId("suggestion")).length).toBe(2);
    expect(log.map(([c]) => c)).toEqual(["suggest_categories"]);
  });
  it("accepting files it and remembers the merchant", async () => {
    const log: [string, Record<string, unknown> | undefined][] = [];
    let done = 0;
    render(CategorySuggest, { invoke: fake(log), ondone: () => done++ });
    await fireEvent.click(screen.getByRole("button", { name: /Suggest categories/ }));
    await fireEvent.click(await screen.findByRole("button", { name: "Accept Dining out for JOES TAQUERIA" }));
    expect(log.find(([c]) => c === "set_category")![1]).toEqual({ id: 1, category: "Dining out", remember: true });
    expect(screen.getAllByTestId("suggestion").length).toBe(1);
    expect(done).toBe(1);
  });
  it("skip removes one without saving", async () => {
    const log: [string, Record<string, unknown> | undefined][] = [];
    render(CategorySuggest, { invoke: fake(log) });
    await fireEvent.click(screen.getByRole("button", { name: /Suggest categories/ }));
    await fireEvent.click(await screen.findByRole("button", { name: "Skip SHELL" }));
    expect(screen.getAllByTestId("suggestion").length).toBe(1);
    expect(log.some(([c]) => c === "set_category")).toBe(false);
  });
  it("accept all files everything", async () => {
    const log: [string, Record<string, unknown> | undefined][] = [];
    render(CategorySuggest, { invoke: fake(log) });
    await fireEvent.click(screen.getByRole("button", { name: /Suggest categories/ }));
    await fireEvent.click(await screen.findByRole("button", { name: "Accept all" }));
    await new Promise((r) => setTimeout(r, 10));
    expect(log.filter(([c]) => c === "set_category").length).toBe(2);
    expect(screen.queryAllByTestId("suggestion").length).toBe(0);
  });
  it("says so when there is nothing, and shows errors", async () => {
    render(CategorySuggest, { invoke: fake([], []) });
    await fireEvent.click(screen.getByRole("button", { name: /Suggest categories/ }));
    expect((await screen.findByRole("status")).textContent).toMatch(/no confident suggestions/);
    cleanup();
    render(CategorySuggest, { invoke: fake([], [], "No model is set up yet.") });
    await fireEvent.click(screen.getByRole("button", { name: /Suggest categories/ }));
    expect((await screen.findByRole("alert")).textContent).toMatch(/No model/);
  });
});
