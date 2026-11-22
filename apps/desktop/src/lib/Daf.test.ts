import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import Daf from "./Daf.svelte";

afterEach(cleanup);

const out = { years: 4, taxEachYearCents: 4_000_000, taxBunchedCents: 3_500_000, savedCents: 500_000, itemizesAnyway: false };

describe("Daf", () => {
  it("shows the saving", async () => {
    const invoke = vi.fn(async () => out);
    render(Daf, { invoke });
    await fireEvent.click(screen.getByText("Compare"));
    await waitFor(() => expect(screen.getByTestId("daf-result").textContent).toContain("saves about"));
    expect(invoke).toHaveBeenCalledWith("daf_bunching", {
      filing: "married_joint", incomeCents: 15_000_000, otherItemizedCents: 1_000_000, givingCents: 800_000, years: 4,
    });
  });

  it("says so when you already itemize", async () => {
    render(Daf, { invoke: async () => ({ ...out, savedCents: 0, itemizesAnyway: true }) });
    await fireEvent.click(screen.getByText("Compare"));
    await waitFor(() => expect(screen.getByTestId("daf-result").textContent).toContain("already itemize"));
  });

  it("shows an error from the calculation", async () => {
    render(Daf, { invoke: async () => { throw new Error("Enter what you give each year."); } });
    await fireEvent.click(screen.getByText("Compare"));
    await waitFor(() => expect(screen.getByRole("alert").textContent).toContain("what you give"));
  });
});
