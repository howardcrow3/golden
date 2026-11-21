import { afterEach, describe, expect, it } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import Privacy from "./Privacy.svelte";

afterEach(cleanup);

const items = [
  { name: "Built-in model download", host: "huggingface.co", sends: "A request for the model file.", inUse: false, when: "Only when you press Download" },
  { name: "Model on this computer", host: "this computer", sends: "Stays here.", inUse: true, when: "Only if you set a local model address" },
];

describe("Privacy", () => {
  it("shows the private status and every connection", async () => {
    render(Privacy, { invoke: async () => ({ private: true, items }) });
    expect((await screen.findByTestId("private-status")).textContent).toMatch(/Private: nothing leaves this computer/);
    const rows = screen.getAllByTestId("connection");
    expect(rows.length).toBe(2);
    expect(rows[0].textContent).toMatch(/huggingface\.co/);
    expect(rows[0].textContent).toMatch(/Off/);
    expect(rows[1].textContent).toMatch(/In use/);
  });
  it("warns when something outside is connected", async () => {
    render(Privacy, { invoke: async () => ({ private: false, items }) });
    expect((await screen.findByTestId("private-status")).textContent).toMatch(/outside service/);
  });
  it("shows an error", async () => {
    render(Privacy, { invoke: async () => { throw new Error("locked"); } });
    expect((await screen.findByRole("alert")).textContent).toMatch(/locked/);
  });
  it("only deletes after the word is typed", async () => {
    const calls: [string, Record<string, unknown> | undefined][] = [];
    render(Privacy, {
      invoke: async (c: string, a?: Record<string, unknown>) => {
        calls.push([c, a]);
        return c === "connections" ? { private: true, items } : c === "ai_activity" ? [] : undefined;
      },
    });
    const btn = screen.getByRole("button", { name: "Delete everything" }) as HTMLButtonElement;
    expect(btn.disabled).toBe(true);
    await fireEvent.input(screen.getByLabelText("Confirm deletion"), { target: { value: "delete" } });
    expect(btn.disabled).toBe(true);
    await fireEvent.input(screen.getByLabelText("Confirm deletion"), { target: { value: "DELETE" } });
    expect(btn.disabled).toBe(false);
    await fireEvent.click(btn);
    expect(await screen.findByTestId("erased")).toBeTruthy();
    expect(calls.find(([c]) => c === "erase_all_data")?.[1]).toEqual({ confirm: "DELETE" });
  });
  it("shows the engine's refusal", async () => {
    render(Privacy, {
      invoke: async (c: string) => {
        if (c === "erase_all_data") throw new Error("Type DELETE to confirm.");
        return c === "connections" ? { private: true, items } : [];
      },
    });
    await fireEvent.input(screen.getByLabelText("Confirm deletion"), { target: { value: "DELETE" } });
    await fireEvent.click(screen.getByRole("button", { name: "Delete everything" }));
    expect((await screen.findByRole("alert")).textContent).toMatch(/Type DELETE/);
  });
});
