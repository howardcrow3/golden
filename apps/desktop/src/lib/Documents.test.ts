import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import Documents from "./Documents.svelte";

afterEach(cleanup);

const w2 = { id: 1, name: "w2.pdf", kind: "W-2", year: 2025, added: "2026-11-22", size: 2048, fields: [{ label: "Wages", cents: 8_500_000 }] };

function mock(initial: unknown[] = []) {
  let docs = [...initial];
  const calls: [string, unknown][] = [];
  const invoke = vi.fn(async (cmd: string, args?: Record<string, unknown>) => {
    calls.push([cmd, args]);
    if (cmd === "vault_list") return docs;
    if (cmd === "vault_add") { docs = [w2]; return w2; }
    if (cmd === "vault_delete") { docs = []; return undefined; }
    if (cmd === "vault_export") return "/Exports/vault-1-w2.pdf";
    return undefined;
  });
  return { invoke, calls };
}

describe("Documents", () => {
  it("shows an empty vault", async () => {
    render(Documents, { invoke: mock().invoke });
    await waitFor(() => screen.getByTestId("docs-empty"));
    expect(screen.getByTestId("docs-empty").textContent).toContain("No documents yet");
  });

  it("adds a file and shows what was read from it", async () => {
    const { invoke, calls } = mock();
    render(Documents, { invoke });
    await fireEvent.change(screen.getByLabelText("Document files"), { target: { files: [new File(["abc"], "w2.pdf")] } });
    await waitFor(() => screen.getByTestId("doc-row"));
    const row = screen.getByTestId("doc-row").textContent ?? "";
    expect(row).toContain("W-2 2025");
    expect(row).toContain("Wages: $85,000.00");
    expect(screen.getByRole("status").textContent).toContain("Added 1 document");
    expect((calls.find(([c]) => c === "vault_add")![1] as { bytes: number[] }).bytes).toEqual([97, 98, 99]);
  });

  it("saves a copy and deletes", async () => {
    const { invoke } = mock([w2]);
    render(Documents, { invoke });
    await waitFor(() => screen.getByTestId("doc-row"));
    await fireEvent.click(screen.getByText("Save a copy"));
    await waitFor(() => expect(screen.getByRole("status").textContent).toContain("/Exports/vault-1-w2.pdf"));
    await fireEvent.click(screen.getByLabelText("Delete w2.pdf"));
    await waitFor(() => expect(screen.queryByTestId("doc-row")).toBeNull());
  });

  it("searches as you type and shows errors", async () => {
    const calls: unknown[] = [];
    const invoke = vi.fn(async (cmd: string, args?: unknown) => {
      if (cmd === "vault_list") { calls.push(args); return []; }
      throw new Error("boom");
    });
    render(Documents, { invoke });
    await fireEvent.input(screen.getByLabelText("Search documents"), { target: { value: "roof" } });
    await waitFor(() => expect(calls).toContainEqual({ query: "roof" }));
    expect(screen.getByTestId("docs-empty").textContent).toContain("Nothing matches");
  });

  it("says when a file had no readable text", async () => {
    const scan = { ...w2, id: 2, name: "scan.jpg", kind: "Other", year: null, fields: [], hasText: false };
    render(Documents, { invoke: mock([scan]).invoke });
    await waitFor(() => screen.getByTestId("doc-unread"));
    expect(screen.getByTestId("doc-unread").textContent).toContain("Couldn't read any text");
  });

  it("stays quiet when text was read", async () => {
    render(Documents, { invoke: mock([{ ...w2, hasText: true }]).invoke });
    await waitFor(() => screen.getByTestId("doc-row"));
    expect(screen.queryByTestId("doc-unread")).toBeNull();
  });
});
