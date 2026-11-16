import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import { afterEach, describe, expect, it } from "vitest";
import Debts from "./Debts.svelte";

const payoff = (months: number, interest: number, feasible = true) => ({
  feasible,
  months,
  totalInterestCents: interest,
  totalPaidCents: 0,
  order: [],
});

function planWith(over: Record<string, unknown> = {}) {
  return {
    debts: [{ name: "Visa", balanceCents: 300000, aprBp: 2299, minCents: 9000, hasTerms: true }],
    extraCents: 0,
    avalanche: payoff(20, 40000),
    snowball: payoff(21, 42000),
    minimumsOnly: payoff(20, 40000),
    ...over,
  };
}

describe("Debts", () => {
  afterEach(cleanup);

  it("shows both strategies", async () => {
    render(Debts, { invoke: async () => planWith() });
    await waitFor(() => expect(screen.getByTestId("payoff")).toBeTruthy());
    const t = screen.getByTestId("payoff").textContent!;
    expect(t).toContain("20 months");
    expect(t).toContain("21 months");
  });

  it("shows what extra money saves", async () => {
    const invoke = async (c: string, a?: unknown) =>
      c === "debt_plan"
        ? planWith({ extraCents: (a as { extraCents: number }).extraCents, avalanche: payoff(10, 15000), minimumsOnly: payoff(20, 40000) })
        : null;
    render(Debts, { invoke });
    await waitFor(() => screen.getByTestId("payoff"));
    await fireEvent.input(screen.getByLabelText("Extra monthly payment"), { target: { value: "200" } });
    await fireEvent.change(screen.getByLabelText("Extra monthly payment"));
    await waitFor(() => expect(screen.getByTestId("saved").textContent).toContain("250"));
  });

  it("asks for terms when none are saved and sends them in cents and basis points", async () => {
    const calls: [string, unknown][] = [];
    const invoke = async (c: string, a?: unknown) => {
      calls.push([c, a]);
      if (c === "debt_plan")
        return planWith({
          debts: [{ name: "Visa", balanceCents: 300000, aprBp: 0, minCents: 0, hasTerms: false }],
          avalanche: null,
          snowball: null,
          minimumsOnly: null,
        });
      return null;
    };
    render(Debts, { invoke });
    await waitFor(() => screen.getByText(/Add each debt's rate/));
    await fireEvent.input(screen.getByLabelText("Visa interest rate"), { target: { value: "22.99" } });
    await fireEvent.input(screen.getByLabelText("Visa minimum payment"), { target: { value: "90" } });
    await fireEvent.click(screen.getByText("Save"));
    await waitFor(() => expect(calls.some(([c]) => c === "set_debt_terms")).toBe(true));
    expect(calls.find(([c]) => c === "set_debt_terms")![1]).toEqual({ name: "Visa", aprBp: 2299, minCents: 9000 });
  });

  it("explains bad input without calling the app", async () => {
    const calls: string[] = [];
    render(Debts, {
      invoke: async (c: string) => (calls.push(c), planWith({ debts: [{ name: "Visa", balanceCents: 1, aprBp: 0, minCents: 0, hasTerms: false }] })),
    });
    await waitFor(() => screen.getByText("Save"));
    await fireEvent.input(screen.getByLabelText("Visa interest rate"), { target: { value: "lots" } });
    await fireEvent.click(screen.getByText("Save"));
    expect(screen.getByRole("alert").textContent).toContain("Visa");
    expect(calls).not.toContain("set_debt_terms");
  });

  it("says never when payments cannot cover interest, and hides with no debts", async () => {
    render(Debts, { invoke: async () => planWith({ avalanche: payoff(600, 1, false), snowball: payoff(600, 1, false) }) });
    await waitFor(() => screen.getAllByText(/never at these payments/));
    cleanup();
    render(Debts, { invoke: async () => planWith({ debts: [] }) });
    await new Promise((r) => setTimeout(r, 20));
    expect(screen.queryByTestId("debts")).toBeNull();
    cleanup();
    render(Debts, {});
    await new Promise((r) => setTimeout(r, 20));
    expect(screen.queryByTestId("debts")).toBeNull();
  });
});
