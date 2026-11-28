import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import Receipts from "./Receipts.svelte";

afterEach(cleanup);

const txn = { id: 7, day: "2026-10-17", payee: "TRADER JOES #55", cents: -2418 };
const item = (over: object) => ({
  documentId: 1,
  name: "tj.jpg",
  merchant: "Trader Joe's",
  totalCents: 2418,
  day: "2026-10-17",
  status: "unmatched",
  linked: null,
  candidates: [],
  ...over,
});

function mock(items: unknown[]) {
  const calls: [string, unknown][] = [];
  const invoke = vi.fn(async (cmd: string, args?: Record<string, unknown>) => {
    calls.push([cmd, args]);
    if (cmd === "match_receipts") return { autoLinked: 1, items };
    return undefined;
  });
  return { invoke, calls };
}

describe("Receipts", () => {
  it("shows nothing when there are no receipts", async () => {
    const { invoke } = mock([]);
    render(Receipts, { invoke });
    await waitFor(() => expect(invoke).toHaveBeenCalled());
    expect(screen.queryByLabelText("Receipts")).toBeNull();
  });

  it("shows the charge a receipt was matched to and lets you undo it", async () => {
    const { invoke, calls } = mock([item({ status: "linked", linked: txn })]);
    render(Receipts, { invoke });
    await waitFor(() => screen.getByTestId("receipt-row"));
    const row = screen.getByTestId("receipt-row").textContent ?? "";
    expect(row).toContain("Matched");
    expect(row).toContain("TRADER JOES #55");
    expect(screen.getByRole("status").textContent).toContain("Matched 1 new receipt");
    await fireEvent.click(screen.getByText("Not this charge"));
    await waitFor(() => expect(calls.some(([c]) => c === "unlink_receipt")).toBe(true));
    expect(calls.find(([c]) => c === "unlink_receipt")![1]).toEqual({ documentId: 1 });
  });

  it("offers the possible charges and links the one you pick", async () => {
    const { invoke, calls } = mock([item({ status: "suggested", candidates: [txn, { ...txn, id: 8, day: "2026-10-18" }] })]);
    render(Receipts, { invoke });
    await waitFor(() => screen.getByTestId("receipt-row"));
    expect(screen.getAllByText("This one")).toHaveLength(2);
    await fireEvent.click(screen.getAllByText("This one")[1]);
    await waitFor(() => expect(calls.some(([c]) => c === "link_receipt")).toBe(true));
    expect(calls.find(([c]) => c === "link_receipt")![1]).toEqual({ documentId: 1, txnId: 8 });
  });

  it("explains duplicates and receipts still waiting for their statement", async () => {
    const { invoke } = mock([item({ status: "duplicate" }), item({ documentId: 2, status: "unmatched" })]);
    render(Receipts, { invoke });
    await waitFor(() => screen.getAllByTestId("receipt-row"));
    const text = screen.getAllByTestId("receipt-row").map((r) => r.textContent).join(" ");
    expect(text).toContain("isn't counted again");
    expect(text).toContain("will match when the statement arrives");
  });
});
