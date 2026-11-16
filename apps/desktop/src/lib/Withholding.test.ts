import { cleanup, render, screen, waitFor } from "@testing-library/svelte";
import { afterEach, describe, expect, it } from "vitest";
import Withholding from "./Withholding.svelte";

const gap = (est: number, w: number) => ({ estimatedCents: est, withheldCents: w, differenceCents: w - est });

function check(over: Record<string, unknown> = {}) {
  return {
    taxableWagesCents: 8832000,
    federal: gap(1000000, 1008000),
    state: gap(300000, 300000),
    missing: [],
    stateIsEstimate: false,
    ...over,
  };
}

describe("Withholding", () => {
  afterEach(cleanup);

  it("says right-on when within $100", async () => {
    render(Withholding, { invoke: async () => check() });
    await waitFor(() => expect(screen.getByTestId("withholding-federal").textContent).toContain("about right"));
  });

  it("predicts a refund when too much is withheld and a bill when too little", async () => {
    render(Withholding, { invoke: async () => check({ federal: gap(1000000, 1500000), state: gap(300000, 100000) }) });
    await waitFor(() => screen.getByTestId("withholding-federal"));
    expect(screen.getByTestId("withholding-federal").textContent).toContain("$5,000 more is withheld");
    expect(screen.getByTestId("withholding-federal").textContent).toContain("refund");
    expect(screen.getByTestId("withholding-state").textContent).toContain("bill");
  });

  it("names jobs that still need a paystub instead of giving a verdict", async () => {
    render(Withholding, { invoke: async () => check({ missing: ["Side gig"] }) });
    await waitFor(() => screen.getByTestId("withholding-missing"));
    expect(screen.getByTestId("withholding-missing").textContent).toContain("Side gig");
    expect(screen.queryByTestId("withholding-federal")).toBeNull();
  });

  it("is hidden with no wages or in a plain browser", async () => {
    render(Withholding, { invoke: async () => check({ taxableWagesCents: 0 }) });
    await new Promise((r) => setTimeout(r, 20));
    expect(screen.queryByTestId("withholding")).toBeNull();
    cleanup();
    render(Withholding, {});
    await new Promise((r) => setTimeout(r, 20));
    expect(screen.queryByTestId("withholding")).toBeNull();
  });
});
