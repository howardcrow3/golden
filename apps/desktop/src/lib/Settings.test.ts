import { afterEach, describe, expect, it } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import App from "../App.svelte";
import Settings from "./Settings.svelte";

afterEach(cleanup);

describe("Settings modal", () => {
  it("opens from the account menu, which lists every category at once", async () => {
    render(App);
    expect(screen.queryByRole("dialog")).toBeNull();
    await fireEvent.click(screen.getByTestId("account"));
    for (const name of ["Licence", "Privacy", "Terms", "About"]) expect(screen.getByRole("menuitem", { name })).toBeTruthy();
    await fireEvent.click(screen.getByRole("menuitem", { name: "Licence" }));
    expect(screen.getByRole("dialog", { name: "Settings" })).toBeTruthy();
    expect(screen.getByRole("tab", { name: "Licence" }).getAttribute("aria-selected")).toBe("true");
    await fireEvent.click(screen.getByRole("tab", { name: "About" }));
    expect(screen.getByRole("tab", { name: "About" }).getAttribute("aria-selected")).toBe("true");
    await fireEvent.click(screen.getByRole("button", { name: "Close settings" }));
    expect(screen.queryByRole("dialog")).toBeNull();
  });

  it("closes on Escape and shows Feedback only when it is available", async () => {
    let closed = 0;
    const { unmount } = render(Settings, { onclose: () => closed++ });
    expect(screen.queryByRole("tab", { name: "Feedback" })).toBeNull();
    await fireEvent.keyDown(window, { key: "Escape" });
    expect(closed).toBe(1);
    unmount();
    render(Settings, { canFeedback: true, onclose: () => {} });
    expect(screen.getByRole("tab", { name: "Feedback" })).toBeTruthy();
  });
});

describe("Chat dock", () => {
  it("sits beside work screens, hides, and is absent on the chat screens", async () => {
    try { localStorage.clear(); } catch { /* ignore */ }
    render(App);
    expect(screen.getByRole("complementary", { name: "Chat" })).toBeTruthy();
    expect(screen.getByText("Looking at Overview")).toBeTruthy();
    await fireEvent.click(screen.getByRole("button", { name: "Hide chat" }));
    expect(screen.queryByRole("complementary", { name: "Chat" })).toBeNull();
    expect(localStorage.getItem("golden.chatDock")).toBe("0");
    await fireEvent.click(screen.getByRole("button", { name: "Chat pane" }));
    expect(screen.getByRole("complementary", { name: "Chat" })).toBeTruthy();
    await fireEvent.click(screen.getAllByRole("button", { name: "Ask" })[0]);
    expect(screen.queryByRole("complementary", { name: "Chat" })).toBeNull();
  });
});
