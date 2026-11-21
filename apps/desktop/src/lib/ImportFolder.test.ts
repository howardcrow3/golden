import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import ImportFolder from "./ImportFolder.svelte";

afterEach(cleanup);

const rep = { account: "First Bank 5678", parsed: 2, added: 2, skippedDuplicates: 0, uncategorized: 0, totalCents: 350000, periodEnd: "2026-10-31", filedAs: null };

describe("ImportFolder", () => {
  it("needs a folder before it can run", () => {
    render(ImportFolder, { invoke: async () => ({}), account: "A", kind: "checking" });
    expect((screen.getByRole("button", { name: "Import new files" }) as HTMLButtonElement).disabled).toBe(true);
  });
  it("sends the folder and chosen account, then summarizes", async () => {
    const calls: [string, Record<string, unknown> | undefined][] = [];
    const onreport = vi.fn();
    render(ImportFolder, {
      invoke: async (c: string, a?: Record<string, unknown>) => {
        calls.push([c, a]);
        return { found: 3, alreadyImported: 1, imported: [rep], failed: [{ name: "x.csv", error: "bad row" }] };
      },
      account: "Jar",
      kind: "savings",
      onreport,
    });
    await fireEvent.input(screen.getByLabelText("Folder to import"), { target: { value: " /Users/me/Downloads " } });
    await fireEvent.click(screen.getByRole("button", { name: "Import new files" }));
    const status = await screen.findByTestId("folder-result");
    expect(status.textContent).toMatch(/Found 3 statement files: 1 imported,\s+1 already imported, 1 could not be read/);
    expect((await screen.findByTestId("folder-failure")).textContent).toMatch(/x\.csv: bad row/);
    expect(calls[0]).toEqual(["import_folder", { path: " /Users/me/Downloads ", account: "Jar", kind: "savings" }]);
    expect(onreport).toHaveBeenCalledWith([rep]);
  });
  it("shows the engine's error", async () => {
    render(ImportFolder, { invoke: async () => { throw new Error("Could not open that folder"); }, account: "A", kind: "checking" });
    await fireEvent.input(screen.getByLabelText("Folder to import"), { target: { value: "/nope" } });
    await fireEvent.click(screen.getByRole("button", { name: "Import new files" }));
    expect((await screen.findByRole("alert")).textContent).toMatch(/Could not open/);
  });
});
