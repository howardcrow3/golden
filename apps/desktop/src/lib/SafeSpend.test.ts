import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import { afterEach, describe, expect, it } from "vitest";
import SafeSpend from "./SafeSpend.svelte";

const answer = (over = {}) => ({
  target: 0.85,
  spendingCents: 9_000_000,
  currentCents: 8_000_000,
  nothingSafe: false,
  capped: false,
  ...over,
});

describe("SafeSpend", () => {
  afterEach(cleanup);

  it("asks for the chosen comfort level and shows the room left", async () => {
    const calls: { cmd: string; args: unknown }[] = [];
    const invoke = async (cmd: string, args?: unknown) => {
      calls.push({ cmd, args });
      return answer();
    };
    render(SafeSpend, { invoke });
    await fireEvent.change(screen.getByLabelText("Comfort level"), { target: { value: "0.9" } });
    await fireEvent.click(screen.getByRole("button", { name: "Find my safe spending" }));
    await waitFor(() => expect(screen.getByTestId("safe-answer")).toBeTruthy());
    expect(calls[0].cmd).toBe("safe_spend");
    expect((calls[0].args as { target: number }).target).toBe(0.9);
    const t = screen.getByTestId("safe-answer").textContent ?? "";
    expect(t).toContain("$90,000");
    expect(t).toContain("$10,000 of room");
  });

  it("says so when the current plan spends more than is safe", async () => {
    const invoke = async () => answer({ spendingCents: 6_000_000 });
    render(SafeSpend, { invoke });
    await fireEvent.click(screen.getByRole("button", { name: "Find my safe spending" }));
    await waitFor(() => expect(screen.getByTestId("safe-answer").textContent).toContain("$20,000 more than that"));
  });

  it("handles a plan with nothing safe to spend, and errors", async () => {
    let fail = false;
    const invoke = async () => {
      if (fail) throw new Error("boom");
      return answer({ nothingSafe: true, spendingCents: 0 });
    };
    render(SafeSpend, { invoke });
    await fireEvent.click(screen.getByRole("button", { name: "Find my safe spending" }));
    await waitFor(() => expect(screen.getByTestId("safe-answer").textContent).toContain("does not work"));
    fail = true;
    await fireEvent.click(screen.getByRole("button", { name: "Find my safe spending" }));
    await waitFor(() => expect(screen.getByRole("alert").textContent).toContain("boom"));
  });
});
