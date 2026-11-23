import { afterEach, describe, expect, it } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import Banks from "./Banks.svelte";

afterEach(cleanup);

const lic = (plaid: boolean) => ({ state: "licensed", daysLeft: null, canEdit: true, plaidActive: plaid });
const item = (over = {}) => ({ itemId: "i1", institution: "Chase", cursor: "c", status: "active", ...over });
const rep = { added: 3, updated: 1, removed: 0, skipped: 0, reconciled: 2 };

function make(state: { items: unknown[]; plaid?: boolean; fail?: Record<string, string> }) {
  const calls: [string, Record<string, unknown> | undefined][] = [];
  const invoke = async (c: string, a?: Record<string, unknown>) => {
    calls.push([c, a]);
    if (state.fail?.[c]) throw new Error(state.fail[c]);
    switch (c) {
      case "license_status": return lic(state.plaid ?? true);
      case "bank_items": return state.items;
      case "bank_sync_all": return [{ bank: "Chase", report: rep, error: null }];
      case "bank_finish_link": return rep;
      case "bank_unlink": return 5;
      default: return undefined;
    }
  };
  return { invoke, calls };
}

describe("Banks", () => {
  it("explains the add-on when it is not active", async () => {
    const { invoke } = make({ items: [], plaid: false });
    render(Banks, { invoke });
    expect((await screen.findByTestId("bank-locked")).textContent).toMatch(/Plaid add-on/);
    expect(screen.queryByRole("button", { name: "Connect a bank" })).toBeNull();
  });

  it("lists linked banks with their status", async () => {
    const { invoke } = make({ items: [item(), item({ itemId: "i2", institution: "Ally", status: "paused" })] });
    render(Banks, { invoke });
    const rows = await screen.findAllByTestId("bank-item");
    expect(rows.length).toBe(2);
    expect(rows[0].textContent).toMatch(/Chase/);
    expect(rows[0].textContent).toMatch(/Syncing/);
    expect(rows[1].textContent).toMatch(/Paused/);
    expect(rows[1].textContent).toMatch(/Resume/);
  });

  it("says when nothing is connected", async () => {
    render(Banks, { invoke: make({ items: [] }).invoke });
    expect(await screen.findByTestId("bank-empty")).toBeTruthy();
    expect((screen.getByRole("button", { name: "Sync now" }) as HTMLButtonElement).disabled).toBe(true);
  });

  it("opens the sign-in when Connect is pressed", async () => {
    const m = make({ items: [] });
    render(Banks, { invoke: m.invoke });
    await fireEvent.click(await screen.findByRole("button", { name: "Connect a bank" }));
    await waitFor(() => expect(m.calls.some(([c]) => c === "bank_connect")).toBe(true));
    expect((await screen.findByTestId("bank-note")).textContent).toMatch(/browser opened/);
  });

  it("finishes linking when the person returns from the bank", async () => {
    const m = make({ items: [] });
    let handler: ((e: { payload: { publicToken: string; institution: string } }) => void) | undefined;
    const listen = async (_e: string, h: typeof handler) => {
      handler = h;
      return () => {};
    };
    render(Banks, { invoke: m.invoke, listen: listen as never });
    await screen.findByRole("button", { name: "Connect a bank" });
    await waitFor(() => expect(handler).toBeTruthy());
    handler!({ payload: { publicToken: "public-1", institution: "Chase" } });
    await waitFor(() => expect(screen.getByTestId("bank-note").textContent).toMatch(/Chase connected: 3 new, 1 updated, 0 removed, 2 replaced by statements/));
    const call = m.calls.find(([c]) => c === "bank_finish_link")!;
    expect(call[1]).toEqual({ publicToken: "public-1", institution: "Chase" });
  });

  it("syncs everything and reports per bank", async () => {
    const m = make({ items: [item()] });
    render(Banks, { invoke: m.invoke });
    await fireEvent.click(await screen.findByRole("button", { name: "Sync now" }));
    expect((await screen.findByTestId("bank-note")).textContent).toMatch(/Chase: 3 new/);
  });

  it("pauses a bank", async () => {
    const m = make({ items: [item()] });
    render(Banks, { invoke: m.invoke });
    await fireEvent.click(await screen.findByRole("button", { name: "Pause" }));
    await waitFor(() => expect(m.calls.find(([c]) => c === "bank_set_paused")?.[1]).toEqual({ itemId: "i1", paused: true }));
  });

  it("asks twice before unlinking", async () => {
    const m = make({ items: [item()] });
    render(Banks, { invoke: m.invoke });
    await fireEvent.click(await screen.findByRole("button", { name: "Unlink" }));
    expect(m.calls.some(([c]) => c === "bank_unlink")).toBe(false);
    await fireEvent.click(screen.getByRole("button", { name: "Click again to unlink" }));
    await waitFor(() => expect(screen.getByTestId("bank-note").textContent).toMatch(/5 synced lines removed/));
  });

  it("shows errors and tells a signed-out bank to relink", async () => {
    const m = make({ items: [item({ status: "login_required" })], fail: { bank_sync_all: "no network" } });
    render(Banks, { invoke: m.invoke });
    const row = await screen.findByTestId("bank-item");
    expect(row.textContent).toMatch(/sign in again/i);
    expect(screen.queryByRole("button", { name: "Pause" })).toBeNull();
    await fireEvent.click(screen.getByRole("button", { name: "Sync now" }));
    expect((await screen.findByRole("alert")).textContent).toMatch(/no network/);
  });
});
