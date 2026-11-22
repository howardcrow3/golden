import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import { afterEach, describe, expect, it } from "vitest";
import RothExplorer from "./RothExplorer.svelte";

const row = (label: string, over = {}) => ({
  label,
  convertedCents: 0,
  lifetimeTaxCents: 20_000_000,
  finalBalanceCents: 0,
  legacyCents: 170_000_000,
  successRate: 0.81,
  depletedYear: null,
  ...over,
});

describe("RothExplorer", () => {
  afterEach(cleanup);

  it("asks with the chosen age and marks the best option", async () => {
    const calls: { cmd: string; args: unknown }[] = [];
    const invoke = async (cmd: string, args?: unknown) => {
      calls.push({ cmd, args });
      return [
        row("No conversions"),
        row("Fill the 12% bracket", { convertedCents: 80_000_000, legacyCents: 180_000_000 }),
        row("Fill the 24% bracket", { convertedCents: 120_000_000, legacyCents: 120_000_000, successRate: 0.7 }),
      ];
    };
    render(RothExplorer, { invoke });
    await fireEvent.input(screen.getByLabelText("Convert until age"), { target: { value: "70" } });
    await fireEvent.click(screen.getByRole("button", { name: "Compare strategies" }));
    await waitFor(() => expect(screen.getAllByTestId("roth-row").length).toBe(3));
    expect(calls[0].cmd).toBe("roth_explore");
    expect((calls[0].args as { untilAge: number }).untilAge).toBe(70);
    expect(screen.getByTestId("roth-best").textContent).toContain("Fill the 12% bracket");
    expect(screen.getAllByTestId("roth-row")[1].className).toContain("best");
  });

  it("says when nothing beats doing nothing, and shows errors", async () => {
    let fail = false;
    const invoke = async () => {
      if (fail) throw new Error("no plan");
      return [row("No conversions", { legacyCents: 200_000_000 }), row("Fill the 12% bracket", { legacyCents: 150_000_000 })];
    };
    render(RothExplorer, { invoke });
    await fireEvent.click(screen.getByRole("button", { name: "Compare strategies" }));
    await waitFor(() => expect(screen.getByTestId("roth-best").textContent).toContain("None of these beat"));
    fail = true;
    await fireEvent.click(screen.getByRole("button", { name: "Compare strategies" }));
    await waitFor(() => expect(screen.getByRole("alert").textContent).toContain("no plan"));
  });
});
