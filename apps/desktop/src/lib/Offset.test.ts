import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import { afterEach, describe, expect, it } from "vitest";
import Offset from "./Offset.svelte";

const effect = {
  monthsWithout: 300,
  monthsWith: 250,
  interestWithoutCents: 20000000,
  interestWithCents: 15000000,
  interestSavedCents: 5000000,
};

async function fill(vals: Record<string, string>) {
  for (const [label, v] of Object.entries(vals)) {
    await fireEvent.input(screen.getByLabelText(label), { target: { value: v } });
  }
}

describe("Offset", () => {
  afterEach(cleanup);

  it("shows the saving", async () => {
    let args: unknown;
    const invoke = async (_c: string, a?: unknown) => {
      args = a;
      return effect;
    };
    render(Offset, { invoke });
    await fill({ "Mortgage owed": "300000", "Mortgage rate": "4.5", "Years left": "25", "Offset savings": "60000" });
    await fireEvent.click(screen.getByText("Compare"));
    await waitFor(() => expect(screen.getByTestId("offset-result")).toBeTruthy());
    const t = screen.getByTestId("offset-result").textContent!;
    expect(t).toContain("20 years 10 months");
    expect(t).toContain("25 years 0 months");
    expect(args).toEqual({ balanceCents: 30000000, rateBp: 450, yearsLeft: 25, offsetCents: 6000000 });
  });

  it("asks for numbers when the form is empty", async () => {
    render(Offset, { invoke: async () => effect });
    await fireEvent.click(screen.getByText("Compare"));
    expect(screen.getByRole("alert").textContent).toContain("Enter what you owe");
  });

  it("shows the error from the service", async () => {
    render(Offset, {
      invoke: async () => {
        throw new Error("Enter the years left as a number from 1 to 50.");
      },
    });
    await fill({ "Mortgage owed": "1000", "Mortgage rate": "4", "Years left": "99" });
    await fireEvent.click(screen.getByText("Compare"));
    await waitFor(() => expect(screen.getByRole("alert").textContent).toContain("years left"));
  });
});
