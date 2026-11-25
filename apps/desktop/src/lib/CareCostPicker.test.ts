import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import { afterEach, describe, expect, it } from "vitest";
import CareCostPicker from "./CareCostPicker.svelte";

describe("CareCostPicker", () => {
  afterEach(cleanup);

  it("fills in the state's median for the chosen kind of care", async () => {
    let args: unknown;
    let used = "";
    const invoke = async (_c: string, a?: unknown) => { args = a; return { annualCents: 20_075_000, national: false }; };
    render(CareCostPicker, { region: "ny", onuse: (d: string) => (used = d), invoke });
    await fireEvent.change(screen.getByLabelText("Kind of care"), { target: { value: "assisted_living" } });
    await fireEvent.click(screen.getByText("Use my state's median"));
    await waitFor(() => screen.getByTestId("care-note"));
    expect(args).toEqual({ state: "ny", kind: "assisted_living" });
    expect(used).toBe("200750");
    expect(screen.getByTestId("care-note").textContent).toContain("NY median");
  });

  it("says when the national median was used", async () => {
    render(CareCostPicker, { region: "dc", onuse: () => {}, invoke: async () => ({ annualCents: 12_957_500, national: true }) });
    await fireEvent.click(screen.getByText("Use my state's median"));
    await waitFor(() => screen.getByTestId("care-note"));
    expect(screen.getByTestId("care-note").textContent).toContain("national median");
  });

  it("shows an error", async () => {
    render(CareCostPicker, { region: "ny", onuse: () => {}, invoke: async () => { throw new Error("Unknown kind of care."); } });
    await fireEvent.click(screen.getByText("Use my state's median"));
    await waitFor(() => screen.getByRole("alert"));
    expect(screen.getByRole("alert").textContent).toContain("Unknown");
  });
});
