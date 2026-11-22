import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import Sepp from "./Sepp.svelte";

afterEach(cleanup);

describe("Sepp", () => {
  it("sends cents and basis points and lists both methods", async () => {
    const invoke = vi.fn(async () => [
      { method: "required_minimum", yearlyCents: 1_381_215, level: false, earliestStopAge: 60 },
      { method: "fixed_amortization", yearlyCents: 2_700_000, level: true, earliestStopAge: 60 },
    ]);
    render(Sepp, { invoke });
    await fireEvent.click(screen.getByText("Work out payments"));
    await waitFor(() => expect(screen.getAllByTestId("sepp-row")).toHaveLength(2));
    expect(invoke).toHaveBeenCalledWith("sepp", { balanceCents: 50_000_000, age: 50, rateBp: 500 });
    expect(screen.getByText("Fixed amortization")).toBeTruthy();
    expect(screen.getAllByText("Age 60").length).toBe(2);
  });

  it("shows the reason when the calculation is refused", async () => {
    const invoke = vi.fn(async () => {
      throw new Error("This calculator covers ages 40 to 70.");
    });
    render(Sepp, { invoke });
    await fireEvent.click(screen.getByText("Work out payments"));
    await waitFor(() => expect(screen.getByRole("alert").textContent).toContain("40 to 70"));
  });
});
