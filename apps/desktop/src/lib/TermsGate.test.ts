import { afterEach, describe, expect, it } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/svelte";
import App from "../App.svelte";
import TermsGate from "./TermsGate.svelte";
import Settings from "./Settings.svelte";
import { TERMS_VERSION } from "./terms";

afterEach(cleanup);

/** A fake backend that remembers the accepted terms. */
function backend(start: { version: string; acceptedOn: string } | null) {
  let saved = start;
  const calls: { cmd: string; args?: Record<string, unknown> }[] = [];
  const invoke = async (cmd: string, args?: Record<string, unknown>) => {
    calls.push({ cmd, args });
    if (cmd === "terms_acceptance") return saved;
    if (cmd === "accept_terms") return (saved = { version: String(args?.version), acceptedOn: "2026-11-28" });
    throw new Error(`unexpected ${cmd}`);
  };
  return { invoke, calls, get saved() { return saved; } };
}

describe("Terms dialog", () => {
  it("needs the box ticked before Accept works, then records the current version", async () => {
    const b = backend(null);
    let accepted = 0;
    render(TermsGate, { invoke: b.invoke, onaccepted: () => accepted++ });
    expect(screen.getByRole("dialog", { name: "Golden Terms of Use" })).toBeTruthy();
    expect(screen.getByText(/not financial, investment, tax or legal advice/)).toBeTruthy();
    const accept = screen.getByRole("button", { name: "Accept and continue" }) as HTMLButtonElement;
    expect(accept.disabled).toBe(true);
    await fireEvent.click(screen.getByRole("checkbox"));
    expect(accept.disabled).toBe(false);
    await fireEvent.click(accept);
    await waitFor(() => expect(accepted).toBe(1));
    expect(b.saved?.version).toBe(TERMS_VERSION);
  });

  it("can't be closed with Escape, and Quit quits", async () => {
    let quit = 0;
    render(TermsGate, { invoke: backend(null).invoke, onaccepted: () => {}, onquit: () => { quit++; } });
    await fireEvent.keyDown(window, { key: "Escape" });
    expect(screen.getByRole("dialog")).toBeTruthy();
    await fireEvent.click(screen.getByRole("button", { name: "Quit" }));
    expect(quit).toBe(1);
  });

  it("shows the error and stays open when saving fails", async () => {
    let accepted = 0;
    const invoke = async () => { throw new Error("disk full"); };
    render(TermsGate, { invoke, onaccepted: () => accepted++ });
    await fireEvent.click(screen.getByRole("checkbox"));
    await fireEvent.click(screen.getByRole("button", { name: "Accept and continue" }));
    expect((await screen.findByRole("alert")).textContent).toContain("disk full");
    expect(accepted).toBe(0);
  });
});

describe("Terms on launch", () => {
  it("asks on first launch and goes away once accepted", async () => {
    const b = backend(null);
    render(App, { invoke: b.invoke });
    const dialog = within(await screen.findByRole("dialog", { name: "Golden Terms of Use" }));
    await fireEvent.click(dialog.getByRole("checkbox"));
    await fireEvent.click(dialog.getByRole("button", { name: "Accept and continue" }));
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
  });

  it("asks again when an older version was accepted", async () => {
    render(App, { invoke: backend({ version: "2000-01-01", acceptedOn: "2000-01-01" }).invoke });
    expect(await screen.findByRole("dialog", { name: "Golden's terms have changed" })).toBeTruthy();
  });

  it("does not ask when the current version is accepted", async () => {
    const b = backend({ version: TERMS_VERSION, acceptedOn: "2026-11-28" });
    render(App, { invoke: b.invoke });
    await waitFor(() => expect(b.calls.some((c) => c.cmd === "terms_acceptance")).toBe(true));
    await Promise.resolve();
    expect(screen.queryByRole("dialog")).toBeNull();
  });

  it("asks when the saved acceptance can't be read in the app", async () => {
    render(App, { invoke: async () => { throw new Error("locked"); } });
    expect(await screen.findByRole("dialog", { name: "Golden Terms of Use" })).toBeTruthy();
  });
});

describe("Settings > Terms", () => {
  it("shows the terms any time from Settings", async () => {
    render(Settings, { tab: "Terms", onclose: () => {} });
    expect(screen.getByRole("tab", { name: "Terms" }).getAttribute("aria-selected")).toBe("true");
    expect(screen.getByTestId("terms-text").textContent).toContain("up to 2 computers");
  });
});
