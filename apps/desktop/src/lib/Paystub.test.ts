import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import { afterEach, describe, expect, it } from "vitest";
import Paystub from "./Paystub.svelte";

const summary = {
  employer: "Acme Corp",
  payDate: "2026-10-20",
  frequency: "semimonthly",
  periodsPerYear: 24,
  grossCents: 400000,
  netCents: 266400,
  federalTaxCents: 42000,
  stateTaxCents: 18000,
  socialSecurityCents: 24800,
  medicareCents: 5800,
  retirementCents: 32000,
  benefitsCents: 11000,
  annualGrossCents: 9600000,
  annualRetirementCents: 768000,
  incomeTaxRateBp: 1500,
};

function fake(calls: string[], over: Record<string, unknown> = {}) {
  return async (cmd: string) => {
    calls.push(cmd);
    if (cmd in over) {
      const v = over[cmd];
      if (v instanceof Error) throw v;
      return v;
    }
    if (cmd === "read_paystub") return summary;
    return { label: "Acme Corp salary", annualCents: 9600000 };
  };
}

describe("Paystub", () => {
  afterEach(cleanup);

  it("needs text before it can read", () => {
    render(Paystub, { invoke: fake([]) });
    expect((screen.getByText("Read paystub") as HTMLButtonElement).disabled).toBe(true);
  });

  it("explains what it found", async () => {
    render(Paystub, { invoke: fake([]) });
    await fireEvent.input(screen.getByLabelText("Paystub text"), { target: { value: "Gross Pay 4,000.00" } });
    await fireEvent.click(screen.getByText("Read paystub"));
    await waitFor(() => expect(screen.getByTestId("paystub-summary")).toBeTruthy());
    const t = screen.getByTestId("paystub-summary").textContent!;
    expect(t).toContain("Acme Corp");
    expect(t).toContain("15.0%");
    expect(t).toContain("semimonthly");
  });

  it("adds the pay to the plan only when asked", async () => {
    const calls: string[] = [];
    render(Paystub, { invoke: fake(calls) });
    await fireEvent.input(screen.getByLabelText("Paystub text"), { target: { value: "x" } });
    await fireEvent.click(screen.getByText("Read paystub"));
    await waitFor(() => screen.getByText("Add to my plan"));
    expect(calls).toEqual(["read_paystub"]);
    await fireEvent.click(screen.getByText("Add to my plan"));
    await waitFor(() => expect(screen.getByTestId("paystub-done").textContent).toContain("Acme Corp salary"));
    expect(calls).toEqual(["read_paystub", "apply_paystub"]);
  });

  it("shows a refusal readably", async () => {
    render(Paystub, { invoke: fake([], { read_paystub: new Error("I couldn't find the gross pay on this paystub.") }) });
    await fireEvent.input(screen.getByLabelText("Paystub text"), { target: { value: "hello" } });
    await fireEvent.click(screen.getByText("Read paystub"));
    await waitFor(() => expect(screen.getByRole("alert").textContent).toContain("gross pay"));
  });

  it("does not offer to add when the pay period is unknown", async () => {
    render(Paystub, { invoke: fake([], { read_paystub: { ...summary, frequency: "", periodsPerYear: 0 } }) });
    await fireEvent.input(screen.getByLabelText("Paystub text"), { target: { value: "x" } });
    await fireEvent.click(screen.getByText("Read paystub"));
    await waitFor(() => screen.getByText(/pay period isn't on this stub/));
    expect(screen.queryByText("Add to my plan")).toBeNull();
  });
});
