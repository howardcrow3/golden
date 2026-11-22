import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import PlanFile from "./PlanFile.svelte";

afterEach(cleanup);

describe("PlanFile", () => {
  it("saves the plan file through the report saver", async () => {
    const calls: [string, unknown][] = [];
    const invoke = vi.fn(async (cmd: string, args?: unknown) => {
      calls.push([cmd, args]);
      return cmd === "export_plan_file" ? '{"format":"golden-plan"}' : "/Exports/plan.json";
    });
    render(PlanFile, { invoke });
    await fireEvent.click(screen.getByText("Save plan file"));
    await waitFor(() => expect(screen.getByRole("status").textContent).toContain("/Exports/plan.json"));
    expect(calls.find(([c]) => c === "save_report")![1]).toMatchObject({ kind: "json", content: '{"format":"golden-plan"}' });
  });

  it("opens a file, tells the screen to reload, and reports what came in", async () => {
    const invoke = vi.fn(async () => ({ people: 2, holdings: 3 }));
    const onimported = vi.fn();
    render(PlanFile, { invoke, onimported });
    const file = new File(['{"format":"golden-plan"}'], "plan.json", { type: "application/json" });
    await fireEvent.change(screen.getByLabelText("Plan file"), { target: { files: [file] } });
    await waitFor(() => expect(screen.getByRole("status").textContent).toContain("2 people and 3 holdings"));
    expect(invoke).toHaveBeenCalledWith("import_plan_file", { text: '{"format":"golden-plan"}' });
    expect(onimported).toHaveBeenCalled();
  });

  it("shows why a file was refused and does not reload", async () => {
    const onimported = vi.fn();
    render(PlanFile, { invoke: async () => { throw new Error("That isn't a Golden plan file."); }, onimported });
    await fireEvent.change(screen.getByLabelText("Plan file"), { target: { files: [new File(["x"], "a.json")] } });
    await waitFor(() => expect(screen.getByRole("alert").textContent).toContain("isn't a Golden plan file"));
    expect(onimported).not.toHaveBeenCalled();
  });
});
