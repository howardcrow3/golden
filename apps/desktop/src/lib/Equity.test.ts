import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import Equity from "./Equity.svelte";

afterEach(cleanup);

const vest = { valueCents: 500_000, withheldCents: 173_250, sharesSold: 35, sharesKept: 65, cashBackCents: 1_750 };
const espp = { purchasePriceCents: 3_400, shares: 294, leftoverCents: 4_400, discountGainCents: 470_400 };

function mock() {
  return vi.fn(async (cmd: string) => (cmd === "rsu_vest" ? vest : cmd === "espp_purchase" ? espp : undefined));
}

describe("Equity", () => {
  it("sends cents and fractions for a vest and shows the result", async () => {
    const invoke = mock();
    render(Equity, { invoke });
    await fireEvent.input(screen.getByLabelText("Shares vesting"), { target: { value: "100" } });
    await fireEvent.input(screen.getByLabelText("Vest price"), { target: { value: "50" } });
    await fireEvent.click(screen.getAllByText("Work it out")[0]);
    await waitFor(() => screen.getByTestId("vest-result"));
    expect(invoke).toHaveBeenCalledWith("rsu_vest", { shares: 100, priceCents: 5000, federalRate: 0.22, otherRate: 0.1265 });
    const text = screen.getByTestId("vest-result").textContent ?? "";
    expect(text).toContain("35");
    expect(text).toContain("65");
    expect(text).toContain("$17.50 cash back");
  });

  it("asks for missing vest numbers without calling the app", async () => {
    const invoke = mock();
    render(Equity, { invoke });
    await fireEvent.click(screen.getAllByText("Work it out")[0]);
    expect(screen.getByRole("alert").textContent).toContain("Enter the shares vesting");
    expect(invoke).not.toHaveBeenCalled();
  });

  it("works out an ESPP purchase with lookback on by default", async () => {
    const invoke = mock();
    render(Equity, { invoke });
    await fireEvent.input(screen.getByLabelText("ESPP saved"), { target: { value: "10000" } });
    await fireEvent.input(screen.getByLabelText("Offering price"), { target: { value: "40" } });
    await fireEvent.input(screen.getByLabelText("Purchase price"), { target: { value: "50" } });
    await fireEvent.click(screen.getAllByText("Work it out")[1]);
    await waitFor(() => screen.getByTestId("espp-result"));
    expect(invoke).toHaveBeenCalledWith("espp_purchase", {
      contributionsCents: 1_000_000,
      offeringCents: 4000,
      purchaseCents: 5000,
      discount: 0.15,
      lookback: true,
    });
    expect(screen.getByTestId("espp-result").textContent).toContain("294");
  });

  it("shows an error from the app", async () => {
    const invoke = vi.fn(async () => {
      throw new Error("The ESPP discount can be at most 15%.");
    });
    render(Equity, { invoke });
    await fireEvent.input(screen.getByLabelText("ESPP saved"), { target: { value: "100" } });
    await fireEvent.input(screen.getByLabelText("Offering price"), { target: { value: "10" } });
    await fireEvent.input(screen.getByLabelText("Purchase price"), { target: { value: "10" } });
    await fireEvent.input(screen.getByLabelText("ESPP discount"), { target: { value: "20" } });
    await fireEvent.click(screen.getAllByText("Work it out")[1]);
    await waitFor(() => screen.getByRole("alert"));
    expect(screen.getByRole("alert").textContent).toContain("at most 15%");
  });
});
