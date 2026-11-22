import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import { afterEach, describe, expect, it } from "vitest";
import SsExplorer from "./SsExplorer.svelte";

const rows = [62, 67, 70].map((age, i) => ({
  claimAge: age,
  annualCents: [[2_660_000, 2_660_000], [3_800_000, 3_800_000], [4_712_000, 4_712_000]][i].slice(0, 1),
  lifetimeCents: [90_000_000, 110_000_000, 100_000_000][i],
  legacyCents: [100_000_000, 150_000_000, 149_500_000][i],
  successRate: [0.7, 0.85, 0.86][i],
  depletedYear: null,
}));
const data = {
  rows,
  breakEvens: [
    { earlier: 62, later: 67, age: 78.4 },
    { earlier: 67, later: 70, age: null },
  ],
};

describe("SsExplorer", () => {
  afterEach(cleanup);

  it("lists the ages, marks the earliest near-best age, and gives break-even ages", async () => {
    const calls: string[] = [];
    const invoke = async (cmd: string) => {
      calls.push(cmd);
      return data;
    };
    render(SsExplorer, { invoke });
    await fireEvent.click(screen.getByRole("button", { name: "Compare claiming ages" }));
    await waitFor(() => expect(screen.getAllByTestId("ss-row").length).toBe(3));
    expect(calls).toEqual(["ss_explore"]);
    // 67 and 70 are within 1% of each other: the earlier one wins.
    expect(screen.getByTestId("ss-best").textContent).toContain("Claiming at 67");
    const be = screen.getAllByTestId("ss-breakeven").map((e) => e.textContent);
    expect(be[0]).toContain("about 78.4 years old");
    expect(be[1]).toContain("never");
  });

  it("shows the app's reason when there is nothing to compare", async () => {
    const invoke = async () => {
      throw new Error("Add a Social Security amount for someone first.");
    };
    render(SsExplorer, { invoke });
    await fireEvent.click(screen.getByRole("button", { name: "Compare claiming ages" }));
    await waitFor(() => expect(screen.getByRole("alert").textContent).toContain("Add a Social Security amount"));
  });
});
