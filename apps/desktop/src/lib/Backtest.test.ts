import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import { afterEach, describe, expect, it } from "vitest";
import Backtest from "./Backtest.svelte";

const windows = [1928, 1929, 1966].map((y, i) => ({
  startYear: y, succeeded: i !== 2, finalCents: [90_000_000, 50_000_000, 0][i], depletedYear: i === 2 ? 2005 : null, annualisedReturn: [0.04, 0.03, 0.01][i],
}));
const data = { stockShare: 0.6, years: 46, successRate: 2 / 3, firstYear: 1928, lastYear: 2025, windows, worst: [1966, 1929, 1928] };

describe("Backtest", () => {
  afterEach(cleanup);

  it("replays history with the chosen mix and summarises the result", async () => {
    const calls: [string, unknown][] = [];
    const invoke = async (cmd: string, args?: unknown) => { calls.push([cmd, args]); return data; };
    render(Backtest, { invoke });
    await fireEvent.change(screen.getByLabelText("Stock share"), { target: { value: "0.8" } });
    await fireEvent.click(screen.getByText("Replay history"));
    await waitFor(() => screen.getByTestId("bt-summary"));
    expect(calls[0][0]).toBe("backtest");
    expect(screen.getByTestId("bt-summary").textContent).toContain("2 of 3");
    expect(screen.getByTestId("bt-worst").textContent).toContain("1966");
    expect(screen.getByTestId("bt-worst").textContent).toContain("ran out in 2005");
  });

  it("shows an error from the engine", async () => {
    const invoke = async () => { throw new Error("The plan runs 120 years but history only covers 98."); };
    render(Backtest, { invoke });
    await fireEvent.click(screen.getByText("Replay history"));
    await waitFor(() => screen.getByRole("alert"));
    expect(screen.getByRole("alert").textContent).toContain("history only covers");
  });
});
