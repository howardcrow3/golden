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

import Import from "./Import.svelte";
import CashFlow from "./CashFlow.svelte";

describe("Import", () => {
  it("sends the file to the engine and shows the report", async () => {
    const calls: unknown[] = [];
    const invoke = async (cmd: string, args?: Record<string, unknown>) => {
      calls.push([cmd, args]);
      return { account: "Checking", parsed: 3, added: 2, skippedDuplicates: 1, uncategorized: 1, totalCents: -1234 };
    };
    render(Import, { invoke });
    const file = new File(["Date,Description,Amount\n"], "oct.csv", { type: "text/csv" });
    await fireEvent.change(screen.getByLabelText("Statement files"), { target: { files: [file] } });
    const report = await screen.findByTestId("report");
    expect(report.textContent).toMatch(/2 added, 1 already imported/);
    expect(report.textContent).toMatch(/-\$12\.34/);
    expect(report.textContent).toMatch(/1 need a category/);
    const [cmd, args] = calls[0] as [string, { account: string; kind: string }];
    expect(cmd).toBe("import_statement");
    expect(args.account).toBe("Checking");
  });
  it("shows the engine's error with the file name", async () => {
    const invoke = async () => {
      throw new Error("could not find a date column");
    };
    render(Import, { invoke });
    const file = new File(["x"], "bad.csv");
    await fireEvent.change(screen.getByLabelText("Statement files"), { target: { files: [file] } });
    expect((await screen.findByRole("alert")).textContent).toMatch(/bad\.csv: could not find a date column/);
  });
});

describe("CashFlow", () => {
  it("renders periods and categories from the engine", async () => {
    const invoke = async (cmd: string) =>
      cmd === "cash_flow"
        ? [{ period: "2026-10", incomeCents: 500000, spendingCents: 120000, netCents: 380000, count: 4 }]
        : [{ category: "Groceries", cents: 8210 }];
    render(CashFlow, { grain: "month", today: new Date("2026-10-20T00:00:00Z"), invoke });
    const table = await screen.findByTestId("cashflow");
    expect(table.textContent).toMatch(/2026-10/);
    expect(table.textContent).toMatch(/\$5,000/);
    expect((await screen.findByTestId("categories")).textContent).toMatch(/Groceries/);
  });
  it("renders nothing when there is no data yet", async () => {
    const invoke = async () => [];
    render(CashFlow, { grain: "month", invoke });
    await new Promise((r) => setTimeout(r, 20));
    expect(screen.queryByTestId("cashflow")).toBeNull();
  });
});
