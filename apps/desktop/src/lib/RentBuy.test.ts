import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import RentBuy from "./RentBuy.svelte";

afterEach(cleanup);
const out = { years: 20, buyNetWorthCents: 90_000_000, rentNetWorthCents: 80_000_000, buyAdvantageCents: 10_000_000, monthlyPaymentCents: 252_800, breakEvenYear: 9 };

describe("RentBuy", () => {
  it("sends cents and basis points and says which is ahead", async () => {
    const invoke = vi.fn(async () => out);
    render(RentBuy, { invoke });
    await fireEvent.click(screen.getByText("Compare"));
    await waitFor(() => expect(screen.getByTestId("rb-answer").textContent).toContain("buying leaves you ahead"));
    expect(invoke).toHaveBeenCalledWith("rent_vs_buy", { req: expect.objectContaining({ priceCents: 50_000_000, mortgageRateBp: 650, years: 20 }) });
    expect(screen.getByTestId("rb-answer").textContent).toContain("after 9 years");
  });
  it("says renting when renting wins and when buying never catches up", async () => {
    render(RentBuy, { invoke: async () => ({ ...out, buyAdvantageCents: -5_000_000, breakEvenYear: null }) });
    await fireEvent.click(screen.getByText("Compare"));
    await waitFor(() => expect(screen.getByTestId("rb-answer").textContent).toContain("renting leaves you ahead"));
    expect(screen.getByTestId("rb-answer").textContent).toContain("does not pull ahead");
  });
  it("rejects a bad amount before asking the engine", async () => {
    const invoke = vi.fn();
    render(RentBuy, { invoke });
    await fireEvent.input(screen.getByLabelText("Price"), { target: { value: "lots" } });
    await fireEvent.click(screen.getByText("Compare"));
    expect((await screen.findByRole("alert")).textContent).toContain("Price");
    expect(invoke).not.toHaveBeenCalled();
  });
});
