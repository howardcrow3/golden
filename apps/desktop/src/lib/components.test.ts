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

import License from "./License.svelte";

describe("License", () => {
  const trial = { state: "trial", daysLeft: 9, canEdit: true, plaidActive: false };
  it("shows the trial countdown", async () => {
    render(License, { invoke: async () => trial });
    expect((await screen.findByTestId("status")).textContent).toBe("Trial: 9 days left.");
  });
  it("activates a key and shows the new status", async () => {
    const invoke = async (cmd: string) =>
      cmd === "activate_license" ? { state: "licensed", daysLeft: null, canEdit: true, plaidActive: true } : trial;
    render(License, { invoke });
    await fireEvent.input(screen.getByLabelText("Licence key"), { target: { value: "abc.def" } });
    await fireEvent.click(screen.getByRole("button", { name: "Activate" }));
    expect((await screen.findByRole("status")).textContent).toMatch(/accepted/);
    expect(screen.getByTestId("status").textContent).toMatch(/Licensed, with bank connections/);
  });
  it("shows why a key was refused", async () => {
    const invoke = async (cmd: string) => {
      if (cmd === "activate_license") throw new Error("licence signature is not valid");
      return trial;
    };
    render(License, { invoke });
    await fireEvent.input(screen.getByLabelText("Licence key"), { target: { value: "x.y" } });
    await fireEvent.click(screen.getByRole("button", { name: "Activate" }));
    expect((await screen.findByRole("alert")).textContent).toMatch(/signature is not valid/);
  });
  it("keeps Activate disabled until something is pasted", () => {
    render(License, { invoke: async () => trial });
    expect((screen.getByRole("button", { name: "Activate" }) as HTMLButtonElement).disabled).toBe(true);
  });
});

import Retirement from "./Retirement.svelte";
import Assumptions from "./Assumptions.svelte";
import type { Household } from "./api";
import { fromForm, newStream, removePerson, toForm } from "./household";
import { assumptionName, optionText } from "./assumptions";

const household = {
  people: [{ name: "You", birthDate: "1981-06-15", retireAge: 62, ssAnnualCents: 3_800_000 }],
  streams: [{ label: "Salary", person: 0, annualCents: 18_000_000, startYear: null, endYear: null, contributionCents: 2_350_000 }],
  filing: "married_joint", state: "TX", spendingCents: 9_000_000, cashCents: 3_000_000,
  taxableCents: 15_000_000, traditionalCents: 60_000_000, rothCents: 12_000_000, picks: {},
};
const planResult = {
  successRate: 0.87, depletedYear: null, p10Cents: 1, p50Cents: 2, p90Cents: 3,
  years: [{ year: 2026, age: 45, balanceCents: 100, spendingCents: 1, taxesCents: 1 }, { year: 2027, age: 46, balanceCents: 200, spendingCents: 1, taxesCents: 1 }],
  realReturn: 0.04, volatility: 0.11, horizonAge: 95, ssClaimAge: 67,
};
const rows = [
  { key: "real_return", active: "Historical", picked: false, options: [
    { label: "Cautious", value: 0.02, likelihood: 0.25, probability: 0.25, source: "Bond-heavy" },
    { label: "Historical", value: 0.04, likelihood: 0.5, probability: 0.5, source: "Long-run balanced" },
  ] },
];

describe("Retirement", () => {
  it("loads the saved household, saves edits, and runs the plan", async () => {
    const calls: [string, Record<string, unknown> | undefined][] = [];
    const invoke = async (cmd: string, args?: Record<string, unknown>) => {
      calls.push([cmd, args]);
      if (cmd === "household") return household;
      if (cmd === "assumptions") return rows;
      if (cmd === "run_plan") return planResult;
      return undefined;
    };
    render(Retirement, { invoke });
    const spending = (await screen.findByLabelText("Spending per year")) as HTMLInputElement;
    expect(spending.value).toBe("90,000");
    await fireEvent.input(spending, { target: { value: "80,000" } });
    await fireEvent.click(screen.getByRole("button", { name: "Save and run plan" }));
    expect((await screen.findByTestId("success")).textContent).toMatch(/87%/);
    const saved = calls.find(([c]) => c === "save_household")![1] as { household: { spendingCents: number } };
    expect(saved.household.spendingCents).toBe(8_000_000);
    expect(calls.findIndex(([c]) => c === "save_household")).toBeLessThan(calls.findIndex(([c]) => c === "run_plan"));
  });
  it("tells the person which field is not a dollar amount, without saving", async () => {
    const calls: string[] = [];
    const invoke = async (cmd: string) => {
      calls.push(cmd);
      return cmd === "household" ? household : rows;
    };
    render(Retirement, { invoke });
    await fireEvent.input(await screen.findByLabelText("Cash"), { target: { value: "lots" } });
    await fireEvent.click(screen.getByRole("button", { name: "Save and run plan" }));
    expect((await screen.findByRole("alert")).textContent).toMatch(/Cash: enter a dollar amount/);
    expect(calls).not.toContain("save_household");
  });
  it("adds a second person and a one-time income, and saves both", async () => {
    const calls: [string, Record<string, unknown> | undefined][] = [];
    const invoke = async (cmd: string, args?: Record<string, unknown>) => {
      calls.push([cmd, args]);
      if (cmd === "household") return household;
      if (cmd === "assumptions") return rows;
      if (cmd === "run_plan") return planResult;
      return undefined;
    };
    render(Retirement, { invoke });
    await screen.findByLabelText("Spending per year");
    await fireEvent.click(screen.getByRole("button", { name: "Add person" }));
    await fireEvent.input(screen.getByLabelText("Name 2"), { target: { value: "Sam" } });
    await fireEvent.input(screen.getByLabelText("Birth date 2"), { target: { value: "1983-02-10" } });
    await fireEvent.click(screen.getByRole("button", { name: "Add one-time income" }));
    await fireEvent.input(screen.getByLabelText("Income name 2"), { target: { value: "Stock vest" } });
    await fireEvent.input(screen.getByLabelText("Income amount 2"), { target: { value: "400,000" } });
    await fireEvent.click(screen.getByRole("button", { name: "Save and run plan" }));
    await screen.findByTestId("success");
    const saved = (calls.find(([c]) => c === "save_household")![1] as { household: Household }).household;
    expect(saved.people.map((p) => p.name)).toEqual(["You", "Sam"]);
    expect(saved.streams[1]).toMatchObject({ label: "Stock vest", annualCents: 40_000_000, startYear: new Date().getFullYear() + 1, endYear: new Date().getFullYear() + 1 });
  });
  it("names the field with a bad year and saves nothing", async () => {
    const calls: string[] = [];
    const invoke = async (cmd: string) => {
      calls.push(cmd);
      return cmd === "household" ? household : rows;
    };
    render(Retirement, { invoke });
    await fireEvent.input(await screen.findByLabelText("First year 1"), { target: { value: "soon" } });
    await fireEvent.click(screen.getByRole("button", { name: "Save and run plan" }));
    expect((await screen.findByRole("alert")).textContent).toMatch(/Salary first year: enter a four-digit year/);
    expect(calls).not.toContain("save_household");
  });
  it("explains that the data lives in the desktop app when run in a browser", async () => {
    render(Retirement);
    expect((await screen.findAllByRole("alert"))[0].textContent).toMatch(/desktop app/);
  });
});

describe("Assumptions", () => {
  it("lists options with likelihoods and sends the pick", async () => {
    const calls: [string, Record<string, unknown> | undefined][] = [];
    const invoke = async (cmd: string, args?: Record<string, unknown>) => {
      calls.push([cmd, args]);
      return cmd === "assumptions" ? rows : household;
    };
    render(Assumptions, { invoke });
    const sel = (await screen.findByLabelText("Investment return (after inflation)")) as HTMLSelectElement;
    expect([...sel.options].map((o) => o.textContent)).toEqual(["Likeliest", "Cautious · 25% likely", "Historical · 50% likely"]);
    expect(screen.getByText(/Using Historical: Long-run balanced/)).toBeTruthy();
    await fireEvent.change(sel, { target: { value: "Cautious" } });
    await Promise.resolve();
    expect(calls.some(([c, a]) => c === "set_pick" && (a as { label: string }).label === "Cautious")).toBe(true);
  });
  it("names assumptions in plain words and falls back for unknown keys", () => {
    expect(assumptionName("horizon_age")).toBe("Plan until age");
    expect(assumptionName("some_new_key")).toBe("some new key");
    expect(optionText(rows[0].options[0])).toBe("Cautious · 25% likely");
  });
});

describe("household form helpers", () => {
  const h = household as unknown as Household;
  it("round-trips a household through the form", () => {
    expect(fromForm(toForm(h))).toEqual(h);
  });
  it("reports which money field is wrong", () => {
    const f = toForm(h);
    f.people[0].ss = "abc";
    expect(() => fromForm(f)).toThrow(/You Social Security: enter a dollar amount/);
  });
  it("one-time income starts and ends in the same future year", () => {
    expect(newStream(true, 2026)).toMatchObject({ start: "2027", end: "2027" });
    expect(newStream(false, 2026)).toMatchObject({ start: "", end: "" });
  });
  it("removing a person drops their income and re-points later ones", () => {
    const f = toForm(h);
    f.people.push({ name: "Sam", birthDate: "1983-02-10", retireAge: 60, ss: "0" }, { name: "Lee", birthDate: "1990-01-01", retireAge: 65, ss: "0" });
    f.streams.push({ label: "Sam pay", person: 1, amount: "1", start: "", end: "", contribution: "0" }, { label: "Lee pay", person: 2, amount: "1", start: "", end: "", contribution: "0" });
    const g = removePerson(f, 1);
    expect(g.people.map((p) => p.name)).toEqual(["You", "Lee"]);
    expect(g.streams.map((s) => [s.label, s.person])).toEqual([["Salary", 0], ["Lee pay", 1]]);
  });
});
