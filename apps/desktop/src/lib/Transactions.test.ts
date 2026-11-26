import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import Transactions from "./Transactions.svelte";

afterEach(cleanup);

const row = { id: 7, day: "2026-10-06", payee: "Corner Cafe", memo: "", amountCents: -1050, category: "Food", account: "Checking", note: "", tags: [] as string[] };

function mock(rows: unknown[]) {
  return vi.fn(async (cmd: string) => (cmd === "transactions" ? rows : cmd === "category_choices" ? ["Food"] : undefined));
}

describe("Transactions notes and tags", () => {
  it("saves a note and tags on a line", async () => {
    const invoke = mock([row]);
    render(Transactions, { invoke });
    const note = await screen.findByLabelText("Note for Corner Cafe on 2026-10-06");
    await fireEvent.change(note, { target: { value: "lunch" } });
    await waitFor(() => expect(invoke).toHaveBeenCalledWith("set_note_tags", { id: 7, note: "lunch", tags: "" }));
  });

  it("saves tags and keeps the existing note", async () => {
    const invoke = mock([{ ...row, note: "lunch", tags: ["work"] }]);
    render(Transactions, { invoke });
    const tags = await screen.findByLabelText("Tags for Corner Cafe on 2026-10-06");
    await fireEvent.change(tags, { target: { value: "work, trip" } });
    await waitFor(() => expect(invoke).toHaveBeenCalledWith("set_note_tags", { id: 7, note: "lunch", tags: "work, trip" }));
  });

  it("finds lines by note or tag in search", async () => {
    render(Transactions, { invoke: mock([{ ...row, note: "birthday", tags: ["trip"] }, { ...row, id: 8, payee: "Gas", note: "", tags: [] }]) });
    const search = await screen.findByLabelText("Search transactions");
    await fireEvent.input(search, { target: { value: "trip" } });
    await waitFor(() => expect(screen.queryByText("Gas")).toBeNull());
    expect(screen.getByText("Corner Cafe")).toBeTruthy();
  });
});
