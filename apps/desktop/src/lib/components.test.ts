import { afterEach, describe, expect, it } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import Overview from "./Overview.svelte";
import TaxCalculator from "./TaxCalculator.svelte";

afterEach(cleanup);

describe("Overview", () => {
  it("switches the budget between week, month, quarter and year", async () => {
    render(Overview);
    expect(screen.getByTestId("budget").textContent).toBe("$6,500");
    await fireEvent.click(screen.getByRole("button", { name: "Year" }));
    expect(screen.getByTestId("budget").textContent).toBe("$78,000");
    await fireEvent.click(screen.getByRole("button", { name: "Quarter" }));
    expect(screen.getByTestId("budget").textContent).toBe("$19,500");
    await fireEvent.click(screen.getByRole("button", { name: "Week" }));
    expect(screen.getByTestId("budget").textContent).toBe("$1,495");
  });
});

describe("TaxCalculator", () => {
  it("rejects a non-numeric wage", async () => {
    render(TaxCalculator);
    await fireEvent.input(screen.getByLabelText("Wages"), { target: { value: "lots" } });
    await fireEvent.click(screen.getByRole("button", { name: "Estimate" }));
    expect((await screen.findByRole("alert")).textContent).toMatch(/dollar amount/);
  });
  it("explains that the engine lives in the desktop app when run in a browser", async () => {
    render(TaxCalculator);
    await fireEvent.click(screen.getByRole("button", { name: "Estimate" }));
    expect((await screen.findByRole("alert")).textContent).toMatch(/desktop app/);
  });
});

import Retirement from "./Retirement.svelte";

describe("Retirement", () => {
  it("shows an explanation instead of crashing when the engine is unavailable", async () => {
    render(Retirement);
    await fireEvent.click(screen.getByRole("button", { name: "Run plan" }));
    expect((await screen.findByRole("alert")).textContent).toMatch(/desktop app/);
  });
  it("offers the market what-ifs", () => {
    render(Retirement);
    const sel = screen.getByLabelText("Market what-if") as HTMLSelectElement;
    expect([...sel.options].map((o) => o.value)).toEqual(["", "Cautious", "Optimistic"]);
  });
});
