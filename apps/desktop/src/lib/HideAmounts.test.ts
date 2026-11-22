import { afterEach, describe, expect, it } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import App from "../App.svelte";
import { amountsHidden, formatCents, setHideAmounts } from "./money";

afterEach(() => {
  cleanup();
  setHideAmounts(false);
  try {
    localStorage.clear();
  } catch {
    // ignore
  }
});

describe("hide amounts", () => {
  it("masks formatted dollars and restores them", () => {
    expect(formatCents(123456)).toBe("$1,234.56");
    setHideAmounts(true);
    expect(amountsHidden()).toBe(true);
    expect(formatCents(123456)).toBe("$•••••");
    expect(formatCents(-5, { whole: true })).toBe("$•••••");
    setHideAmounts(false);
    expect(formatCents(123456)).toBe("$1,234.56");
  });

  it("is a toggle in the sidebar that is remembered", async () => {
    render(App);
    const btn = screen.getByRole("button", { name: "Hide amounts" });
    expect(btn.getAttribute("aria-pressed")).toBe("false");
    await fireEvent.click(btn);
    expect(amountsHidden()).toBe(true);
    expect(screen.getByRole("button", { name: "Show amounts" }).getAttribute("aria-pressed")).toBe("true");
    expect(localStorage.getItem("golden.hideAmounts")).toBe("1");
    await fireEvent.click(screen.getByRole("button", { name: "Show amounts" }));
    expect(amountsHidden()).toBe(false);
    expect(localStorage.getItem("golden.hideAmounts")).toBe("0");
  });
});
