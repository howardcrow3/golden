import { afterEach, describe, expect, it } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import ModelSetup from "./ModelSetup.svelte";

afterEach(cleanup);

function fake(log: [string, Record<string, unknown> | undefined][], saved = { endpoint: "", name: "" }, fail = "") {
  return async (cmd: string, args?: Record<string, unknown>) => {
    log.push([cmd, args]);
    if (cmd === "model_settings") return saved;
    if (fail) throw new Error(fail);
    return { endpoint: String(args?.endpoint ?? ""), name: String(args?.name ?? "") };
  };
}

describe("ModelSetup", () => {
  it("loads what was saved", async () => {
    render(ModelSetup, { invoke: fake([], { endpoint: "http://127.0.0.1:11434/api/generate", name: "llama3.2" }) });
    expect(await screen.findByText("Using a model on this computer")).toBeTruthy();
    expect((screen.getByLabelText("Model address") as HTMLInputElement).value).toBe("http://127.0.0.1:11434/api/generate");
    expect((screen.getByLabelText("Model name") as HTMLInputElement).value).toBe("llama3.2");
  });
  it("saves the address and name", async () => {
    const log: [string, Record<string, unknown> | undefined][] = [];
    render(ModelSetup, { invoke: fake(log) });
    await new Promise((r) => setTimeout(r, 5)); // let the saved settings load first
    await fireEvent.input(screen.getByLabelText("Model address"), { target: { value: "http://127.0.0.1:11434/api/generate" } });
    await fireEvent.input(screen.getByLabelText("Model name"), { target: { value: "llama3.2" } });
    await fireEvent.click(screen.getByRole("button", { name: "Save" }));
    expect((await screen.findByRole("status")).textContent).toMatch(/asks that model first/);
    expect(log.find(([c]) => c === "set_model")![1]).toEqual({ endpoint: "http://127.0.0.1:11434/api/generate", name: "llama3.2" });
  });
  it("says it is turned off when the address is cleared", async () => {
    render(ModelSetup, { invoke: fake([]) });
    await fireEvent.click(screen.getByRole("button", { name: "Save" }));
    expect((await screen.findByRole("status")).textContent).toMatch(/Turned off/);
  });
  it("shows why an address was refused", async () => {
    render(ModelSetup, { invoke: fake([], { endpoint: "", name: "" }, "Golden only talks to a model running on this computer") });
    await fireEvent.click(screen.getByRole("button", { name: "Save" }));
    expect((await screen.findByRole("alert")).textContent).toMatch(/only talks to a model running on this computer/);
  });
});
