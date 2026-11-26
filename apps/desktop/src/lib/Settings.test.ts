import { afterEach, describe, expect, it } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import App from "../App.svelte";
import Settings from "./Settings.svelte";

afterEach(cleanup);

describe("Settings modal", () => {
  it("opens from the sidebar, switches tabs and closes", async () => {
    render(App);
    expect(screen.queryByRole("dialog")).toBeNull();
    await fireEvent.click(screen.getByRole("button", { name: "Settings" }));
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
