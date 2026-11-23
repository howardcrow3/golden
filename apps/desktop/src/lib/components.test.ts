import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import Overview from "./Overview.svelte";
import TaxCalculator from "./TaxCalculator.svelte";

afterEach(cleanup);

describe("Overview", () => {
  it("has a week, month, quarter and year toggle with month selected first", async () => {
    render(Overview);
    expect(screen.getByRole("button", { name: "Month" }).getAttribute("aria-pressed")).toBe("true");
    await fireEvent.click(screen.getByRole("button", { name: "Year" }));
    expect(screen.getByRole("button", { name: "Year" }).getAttribute("aria-pressed")).toBe("true");
    expect(screen.getByRole("button", { name: "Month" }).getAttribute("aria-pressed")).toBe("false");
  });
});

describe("Overview quick actions", () => {
  it("jumps to Ask, What if and Import", async () => {
    const go = vi.fn();
    render(Overview, { onnavigate: go });
    await fireEvent.click(screen.getByRole("button", { name: "Ask about your money" }));
    await fireEvent.click(screen.getByRole("button", { name: "Try a what if" }));
    await fireEvent.click(screen.getByRole("button", { name: "Import a statement" }));
    expect(go.mock.calls.map((c) => c[0])).toEqual(["Ask", "What if", "Import"]);
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

describe("Import folder", () => {
  it("changes the statement folder and shows the new one", async () => {
    const calls: [string, Record<string, unknown> | undefined][] = [];
    const invoke = async (cmd: string, args?: Record<string, unknown>) => {
      calls.push([cmd, args]);
      return cmd === "library_folder" ? "/data/Statements" : cmd === "set_library_folder" ? (args as { path: string }).path : [];
    };
    render(Import, { invoke });
    await screen.findByTestId("folder");
    await fireEvent.click(screen.getByRole("button", { name: "Change statement folder" }));
    const box = screen.getByLabelText("Statement folder") as HTMLInputElement;
    expect(box.value).toBe("/data/Statements");
    await fireEvent.input(box, { target: { value: "/Users/me/Statements" } });
    await fireEvent.click(screen.getByRole("button", { name: "Use this folder" }));
    expect((await screen.findByTestId("folder")).textContent).toBe("/Users/me/Statements");
    expect(screen.getByRole("status").textContent).toMatch(/Files already filed stay where they are/);
    expect(calls.find(([c]) => c === "set_library_folder")![1]).toEqual({ path: "/Users/me/Statements" });
  });
  it("shows the app's reason when the folder is refused", async () => {
    const invoke = async (cmd: string) => {
      if (cmd === "library_folder") return "/data/Statements";
      throw "Enter the full path of the folder";
    };
    render(Import, { invoke });
    await screen.findByTestId("folder");
    await fireEvent.click(screen.getByRole("button", { name: "Change statement folder" }));
    await fireEvent.click(screen.getByRole("button", { name: "Use this folder" }));
    expect((await screen.findByRole("alert")).textContent).toMatch(/full path/);
    expect(screen.getByTestId("folder").textContent).toBe("/data/Statements");
  });
});

describe("Import", () => {
  it("sends the file to the engine and shows the report", async () => {
    const calls: [string, Record<string, unknown> | undefined][] = [];
    const invoke = async (cmd: string, args?: Record<string, unknown>) => {
      calls.push([cmd, args]);
      if (cmd === "library_folder") return "/data/Statements";
      return { account: "Checking", parsed: 3, added: 2, skippedDuplicates: 1, uncategorized: 1, totalCents: -1234, periodEnd: "2026-10-31", filedAs: "2026/Checking/2026-10 Checking statement.csv" };
    };
    render(Import, { invoke });
    const file = new File(["Date,Description,Amount\n"], "oct.csv", { type: "text/csv" });
    await fireEvent.change(screen.getByLabelText("Statement files"), { target: { files: [file] } });
    const report = await screen.findByTestId("report");
    expect(report.textContent).toMatch(/2 added, 1 already imported/);
    expect(report.textContent).toMatch(/-\$12\.34/);
    expect(report.textContent).toMatch(/1 need a category/);
    expect(report.textContent).toMatch(/Filed as 2026\/Checking\/2026-10 Checking statement\.csv/);
    expect((await screen.findByTestId("folder")).textContent).toBe("/data/Statements");
    const [cmd, rawArgs] = calls.find(([c]) => c === "import_file")!;
    const args = rawArgs as { name: string; account: string; kind: string };
    expect(cmd).toBe("import_file");
    expect(args.name).toBe("oct.csv");
    expect(args.account).toBe("Checking");
  });
  it("says whether the balance matches the statement", async () => {
    const base = { account: "A", parsed: 1, added: 1, skippedDuplicates: 0, uncategorized: 0, totalCents: 100, periodEnd: "2026-10-31", filedAs: null };
    const run = async (extra: Record<string, unknown>) => {
      const invoke = async (cmd: string) => (cmd === "library_folder" ? "/d" : { ...base, ...extra });
      const { unmount } = render(Import, { invoke });
      const file = new File(["x"], "a.ofx");
      await fireEvent.change(screen.getByLabelText("Statement files"), { target: { files: [file] } });
      const el = await screen.findByTestId("recon");
      const text = el.textContent ?? "";
      unmount();
      return text;
    };
    expect(await run({ statementBalanceCents: 350000, balanceAfterCents: 350000, reconciled: true, differenceCents: 0 })).toMatch(/matches the statement/);
    expect(await run({ statementBalanceCents: 400000, balanceAfterCents: 350000, reconciled: false, differenceCents: 50000 })).toMatch(/off by \$500\.00/);
  });
  it("shows no balance line when the statement prints none", async () => {
    const invoke = async (cmd: string) => (cmd === "library_folder" ? "/d" : { account: "A", parsed: 1, added: 1, skippedDuplicates: 0, uncategorized: 0, totalCents: 100, periodEnd: null, filedAs: null, reconciled: null });
    render(Import, { invoke });
    await fireEvent.change(screen.getByLabelText("Statement files"), { target: { files: [new File(["x"], "a.csv")] } });
    await screen.findByTestId("report");
    expect(screen.queryByTestId("recon")).toBeNull();
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
import { DEFAULT_PRICING } from "./api";

describe("License prices", () => {
  it("shows $39.99 once and $5 a month by default", () => {
    render(License);
    expect(screen.getByTestId("prices").textContent).toBe(
      "Golden is $39.99 once per major version. Bank connections are an optional $9.97 a month, or $100.00 a year.",
    );
  });
  it("uses the prices the app reports and shows a bad price ID warning", async () => {
    const invoke = async (cmd: string) =>
      cmd === "pricing"
        ? { currency: "USD", coreCents: 2500, plaidCents: 2000, plaidInterval: "year", plaidYearlyCents: 0, corePriceId: null, plaidPriceId: null, warning: "Core price: pro_x is a Paddle product ID." }
        : { state: "trial", daysLeft: 3, canEdit: true, plaidActive: false };
    render(License, { invoke });
    await vi.waitFor(() =>
      expect(screen.getByTestId("prices").textContent).toBe("Golden is $25.00 once per major version. Bank connections are an optional $20.00 a year."),
    );
    expect((await screen.findByRole("alert")).textContent).toMatch(/product ID/);
  });
});

describe("License buy buttons", () => {
  const priced = (url: string | null) => ({
    currency: "USD", coreCents: 3999, plaidCents: 997, plaidInterval: "month", plaidYearlyCents: 10000,
    corePriceId: "pri_a", plaidPriceId: "pri_b", coreCheckoutUrl: url, plaidCheckoutUrl: url, warning: null,
  });
  it("are off until a checkout page is configured", async () => {
    render(License, { invoke: async (cmd: string) => (cmd === "pricing" ? priced(null) : { state: "trial", daysLeft: 3, canEdit: true, plaidActive: false }) });
    const core = (await screen.findByRole("button", { name: /Buy Golden, \$39\.99/ })) as HTMLButtonElement;
    expect(core.disabled).toBe(true);
    expect(screen.getByText("Purchasing opens soon.")).toBeTruthy();
  });
  it("open checkout for the chosen plan when configured", async () => {
    const calls: [string, Record<string, unknown> | undefined][] = [];
    const invoke = async (cmd: string, args?: Record<string, unknown>) => {
      calls.push([cmd, args]);
      return cmd === "pricing" ? priced("https://x.example/c/?price=pri_a&app=golden") : cmd === "open_checkout" ? undefined : { state: "trial", daysLeft: 3, canEdit: true, plaidActive: false };
    };
    render(License, { invoke });
    const plaid = (await screen.findByRole("button", { name: /Add bank connections, \$9\.97 a month/ })) as HTMLButtonElement;
    await vi.waitFor(() => expect(plaid.disabled).toBe(false));
    await fireEvent.click(plaid);
    await vi.waitFor(() => expect(calls.find(([c]) => c === "open_checkout")![1]).toEqual({ plan: "plaid" }));
    expect((await screen.findByRole("status")).textContent).toMatch(/Checkout opened/);
  });
  it("shows the reason when the browser can't be opened", async () => {
    const invoke = async (cmd: string) => {
      if (cmd === "pricing") return priced("https://x.example/c/?price=pri_a&app=golden");
      if (cmd === "open_checkout") throw "Couldn't open your browser: nope";
      return { state: "trial", daysLeft: 3, canEdit: true, plaidActive: false };
    };
    render(License, { invoke });
    const core = (await screen.findByRole("button", { name: /Buy Golden/ })) as HTMLButtonElement;
    await vi.waitFor(() => expect(core.disabled).toBe(false));
    await fireEvent.click(core);
    expect((await screen.findByRole("alert")).textContent).toMatch(/Couldn't open your browser/);
  });
});

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
  it("keeps the self-employed flag through the form", () => {
    const own = { ...h, streams: [{ ...h.streams[0], selfEmployed: true }] };
    expect(fromForm(toForm(own)).streams[0].selfEmployed).toBe(true);
  });

  it("round-trips a home through the form", () => {
    const own = { ...h, home: { valueCents: 60_000_000, basisCents: 30_000_000, mortgageBalanceCents: 20_000_000, mortgageRateBp: 450, mortgageYearsLeft: 15, sellYear: 2035, sellingCostBp: 600, replacementCostCents: 25_000_000, rentAfterCents: 0 } };
    const f = toForm(own);
    expect(f.home?.rate).toBe("4.5");
    expect(fromForm(f).home).toEqual(own.home);
    expect(fromForm(toForm(h)).home).toBeUndefined();
  });
  it("refuses an out-of-range mortgage rate", () => {
    const f = toForm({ ...h, home: { valueCents: 1, basisCents: 1, mortgageBalanceCents: 1, mortgageRateBp: 450, mortgageYearsLeft: 1, sellYear: null, sellingCostBp: 600, replacementCostCents: 0, rentAfterCents: 0 } });
    f.home!.rate = "99";
    expect(() => fromForm(f)).toThrow(/Mortgage rate/);
  });

  it("round-trips a household through the form", () => {
    expect(fromForm(toForm(h))).toEqual({ ...h, events: [], spendingRule: "fixed", withdrawalOrder: "standard", medicare: true });
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
    f.streams.push({ label: "Sam pay", person: 1, amount: "1", start: "", end: "", contribution: "0", selfEmployed: false, kind: "", match: "0", hsa: "0" }, { label: "Lee pay", person: 2, amount: "1", start: "", end: "", contribution: "0", selfEmployed: false, kind: "", match: "0", hsa: "0" });
    const g = removePerson(f, 1);
    expect(g.people.map((p) => p.name)).toEqual(["You", "Lee"]);
    expect(g.streams.map((s) => [s.label, s.person])).toEqual([["Salary", 0], ["Lee pay", 1]]);
  });
});

import WhatIf from "./WhatIf.svelte";

describe("WhatIf", () => {
  const reply = (understood = true) => ({
    understood,
    message: understood ? "Here is what changes: Retire at 65." : "I couldn't turn that into a change yet.",
    changes: understood ? [{ kind: "retireAt", person: 0, age: 65 }] : [],
    descriptions: understood ? ["You retires at 65"] : [],
    before: understood ? { ...planResult, successRate: 0.7 } : null,
    after: understood ? { ...planResult, successRate: 0.85 } : null,
  });
  it("shows before and after, and keeps the change on request", async () => {
    const calls: [string, Record<string, unknown> | undefined][] = [];
    const invoke = async (cmd: string, args?: Record<string, unknown>) => {
      calls.push([cmd, args]);
      return cmd === "whatif" ? reply() : household;
    };
    render(WhatIf, { invoke });
    await fireEvent.input(screen.getByLabelText("What if question"), { target: { value: "retire at 65" } });
    await fireEvent.click(screen.getByRole("button", { name: "Ask" }));
    expect((await screen.findByTestId("before")).textContent).toBe("70%");
    expect(screen.getByTestId("after").textContent).toBe("85%");
    expect(screen.getByText("up 15 points")).toBeTruthy();
    expect(calls.some(([c]) => c === "keep_whatif")).toBe(false); // preview saves nothing
    await fireEvent.click(screen.getByRole("button", { name: "Keep this change" }));
    expect((await screen.findByRole("status")).textContent).toMatch(/Saved to your plan/);
    const kept = calls.find(([c]) => c === "keep_whatif")![1] as { changes: unknown[] };
    expect(kept.changes).toEqual([{ kind: "retireAt", person: 0, age: 65 }]);
  });
  it("dismissing changes nothing", async () => {
    const calls: string[] = [];
    const invoke = async (cmd: string) => {
      calls.push(cmd);
      return reply();
    };
    render(WhatIf, { invoke });
    await fireEvent.click(screen.getByRole("button", { name: "Spend $80k a year" }));
    await fireEvent.click(await screen.findByRole("button", { name: "Dismiss" }));
    expect((await screen.findByRole("status")).textContent).toMatch(/Nothing was changed/);
    expect(calls.filter((c) => !c.startsWith("model_") && !c.endsWith("scenarios") && c !== "undoable_plan")).toEqual(["whatif"]);
  });
  it("saves a what-if as a named scenario", async () => {
    const calls: [string, Record<string, unknown> | undefined][] = [];
    const invoke = async (cmd: string, args?: Record<string, unknown>) => {
      calls.push([cmd, args]);
      if (cmd === "save_scenario") return { id: 1, name: "Early", changes: [], descriptions: [] };
      return reply();
    };
    render(WhatIf, { invoke });
    await fireEvent.click(screen.getByRole("button", { name: "Spend $80k a year" }));
    const save = await screen.findByRole("button", { name: "Save as scenario" });
    expect((save as HTMLButtonElement).disabled).toBe(true);
    await fireEvent.input(screen.getByLabelText("Scenario name"), { target: { value: "Early" } });
    await fireEvent.click(save);
    expect((await screen.findByTestId("scenario-saved")).textContent).toMatch(/Saved as "Early"/);
    const sent = calls.find(([c]) => c === "save_scenario")![1] as { name: string; changes: unknown[] };
    expect(sent.name).toBe("Early");
    expect(sent.changes.length).toBeGreaterThan(0);
  });
  it("shows the help text and no buttons when nothing was understood", async () => {
    render(WhatIf, { invoke: async () => reply(false) });
    await fireEvent.click(screen.getByRole("button", { name: "What if the market crashes?" }));
    expect((await screen.findByTestId("turn")).textContent).toMatch(/couldn't turn that into a change/);
    expect(screen.queryByRole("button", { name: "Keep this change" })).toBeNull();
  });
  it("shows an engine error", async () => {
    render(WhatIf, { invoke: async (cmd: string) => { if (cmd !== "whatif") return undefined; throw new Error("That change doesn't work: nope"); } });
    await fireEvent.input(screen.getByLabelText("What if question"), { target: { value: "retire at 40" } });
    await fireEvent.click(screen.getByRole("button", { name: "Ask" }));
    expect((await screen.findByRole("alert")).textContent).toMatch(/doesn't work/);
  });
  it("ignores an empty question", () => {
    render(WhatIf, { invoke: async () => reply() });
    expect((screen.getByRole("button", { name: "Ask" }) as HTMLButtonElement).disabled).toBe(true);
  });
});

import Transactions from "./Transactions.svelte";

describe("Transactions", () => {
  const txns = [
    { id: 2, day: "2026-10-09", payee: "MYSTERY VENDOR 22", memo: "", amountCents: -1000, category: null, account: "Checking" },
    { id: 1, day: "2026-10-01", payee: "ACME PAYROLL", memo: "", amountCents: 500000, category: "Salary", account: "Checking" },
  ];
  function fake(log: [string, Record<string, unknown> | undefined][], also = 1) {
    return async (cmd: string, args?: Record<string, unknown>) => {
      log.push([cmd, args]);
      if (cmd === "transactions") return (args as { onlyUncategorized: boolean }).onlyUncategorized ? txns.slice(0, 1) : txns;
      if (cmd === "category_choices") return ["Groceries", "Salary"];
      return { category: "Hobbies", alsoUpdated: also };
    };
  }
  it("lists transactions with amounts and categories", async () => {
    render(Transactions, { invoke: fake([]), today: new Date("2026-11-16T00:00:00Z") });
    expect(await screen.findByText("ACME PAYROLL")).toBeTruthy();
    expect(screen.getByText("$5,000.00")).toBeTruthy();
    expect(screen.getByText("-$10.00")).toBeTruthy();
    expect((screen.getByLabelText("Category for ACME PAYROLL on 2026-10-01") as HTMLInputElement).value).toBe("Salary");
  });
  it("sends a recategorization with the remember choice and reports the extra lines", async () => {
    const log: [string, Record<string, unknown> | undefined][] = [];
    render(Transactions, { invoke: fake(log, 2) });
    const input = await screen.findByLabelText("Category for MYSTERY VENDOR 22 on 2026-10-09");
    await fireEvent.change(input, { target: { value: "Hobbies" } });
    expect((await screen.findByRole("status")).textContent).toMatch(/Filed under Hobbies, and 2 more/);
    const call = log.find(([c]) => c === "set_category")![1] as { id: number; category: string; remember: boolean };
    expect(call).toEqual({ id: 2, category: "Hobbies", remember: true });
  });
  it("can turn off remembering and filter to lines that need a category", async () => {
    const log: [string, Record<string, unknown> | undefined][] = [];
    render(Transactions, { invoke: fake(log) });
    await screen.findByText("ACME PAYROLL");
    await fireEvent.click(screen.getByLabelText("Remember this merchant for next time"));
    await fireEvent.click(screen.getByLabelText("Only lines that need a category"));
    await screen.findByLabelText("Category for MYSTERY VENDOR 22 on 2026-10-09");
    expect(screen.queryByText("ACME PAYROLL")).toBeNull();
    await fireEvent.change(screen.getByLabelText("Category for MYSTERY VENDOR 22 on 2026-10-09"), { target: { value: "Hobbies" } });
    await screen.findByRole("status");
    expect((log.find(([c]) => c === "set_category")![1] as { remember: boolean }).remember).toBe(false);
  });
  it("narrows the list as you search and says how much is shown", async () => {
    render(Transactions, { invoke: fake([]) });
    await screen.findByText("ACME PAYROLL");
    expect(screen.getByTestId("shown").textContent).toContain("2 of 2 lines");
    await fireEvent.input(screen.getByLabelText("Search transactions"), { target: { value: "mystery" } });
    expect(screen.queryByText("ACME PAYROLL")).toBeNull();
    expect(screen.getByTestId("shown").textContent).toBe("1 of 2 lines · in $0.00 · out $10.00");
    await fireEvent.input(screen.getByLabelText("Search transactions"), { target: { value: "zzz" } });
    expect(screen.getByText("Nothing matches that search.")).toBeTruthy();
  });
  it("does nothing when the category is unchanged or empty", async () => {
    const log: [string, Record<string, unknown> | undefined][] = [];
    render(Transactions, { invoke: fake(log) });
    const input = await screen.findByLabelText("Category for ACME PAYROLL on 2026-10-01");
    await fireEvent.change(input, { target: { value: "Salary" } });
    await fireEvent.change(input, { target: { value: "  " } });
    expect(log.some(([c]) => c === "set_category")).toBe(false);
  });
  it("exports a CSV and shows where it was saved", async () => {
    const log: [string, Record<string, unknown> | undefined][] = [];
    const base = fake(log);
    const invoke = async (cmd: string, args?: Record<string, unknown>) =>
      cmd === "export_transactions" ? (log.push([cmd, args]), "/data/Exports/golden-transactions.csv") : base(cmd, args);
    render(Transactions, { invoke, today: new Date("2026-11-16T00:00:00Z") });
    await screen.findByText("ACME PAYROLL");
    await fireEvent.click(screen.getByRole("button", { name: "Export CSV" }));
    expect((await screen.findByRole("status")).textContent).toBe("Exported to /data/Exports/golden-transactions.csv");
    expect(log.find(([c]) => c === "export_transactions")![1]).toEqual({ from: "2025-11-16", to: "2026-11-16" });
  });
  it("lists and exports the dates the person picked", async () => {
    const log: [string, Record<string, unknown> | undefined][] = [];
    const base = fake(log);
    const invoke = async (cmd: string, args?: Record<string, unknown>) =>
      cmd === "export_transactions" ? (log.push([cmd, args]), "/data/x.csv") : base(cmd, args);
    render(Transactions, { invoke, today: new Date("2026-11-16T00:00:00Z") });
    await screen.findByText("ACME PAYROLL");
    await fireEvent.input(screen.getByLabelText("From date"), { target: { value: "2026-01-01" } });
    await fireEvent.input(screen.getByLabelText("To date"), { target: { value: "2026-06-30" } });
    await fireEvent.click(screen.getByRole("button", { name: "Export CSV" }));
    await screen.findByText(/Exported to/);
    expect(log.find(([c]) => c === "export_transactions")![1]).toEqual({ from: "2026-01-01", to: "2026-06-30" });
    const lists = log.filter(([c]) => c === "transactions");
    expect(lists[lists.length - 1][1]).toMatchObject({ from: "2026-01-01", to: "2026-06-30" });
  });
  it("refuses an end date before the start date and does not export", async () => {
    const log: [string, Record<string, unknown> | undefined][] = [];
    render(Transactions, { invoke: fake(log), today: new Date("2026-11-16T00:00:00Z") });
    await screen.findByText("ACME PAYROLL");
    await fireEvent.input(screen.getByLabelText("To date"), { target: { value: "2020-01-01" } });
    await fireEvent.click(screen.getByRole("button", { name: "Export CSV" }));
    expect((await screen.findByRole("alert")).textContent).toMatch(/start date must be on or before/);
    expect(log.some(([c]) => c === "export_transactions")).toBe(false);
  });
  it("backs up everything and shows where the file went", async () => {
    const log: [string, Record<string, unknown> | undefined][] = [];
    const base = fake(log);
    const invoke = async (cmd: string, args?: Record<string, unknown>) =>
      cmd === "export_backup" ? (log.push([cmd, args]), "/data/Exports/golden-backup-2026-11-16.json") : base(cmd, args);
    render(Transactions, { invoke, today: new Date("2026-11-16T00:00:00Z") });
    await screen.findByText("ACME PAYROLL");
    await fireEvent.click(screen.getByRole("button", { name: "Back up everything" }));
    expect((await screen.findByRole("status")).textContent).toBe("Backup saved to /data/Exports/golden-backup-2026-11-16.json");
    expect(log.some(([c]) => c === "export_backup")).toBe(true);
  });
  it("restores a backup file and reloads the list", async () => {
    const log: [string, Record<string, unknown> | undefined][] = [];
    const base = fake(log);
    const invoke = async (cmd: string, args?: Record<string, unknown>) =>
      cmd === "restore_backup"
        ? (log.push([cmd, args]), { accounts: 1, added: 3, skippedDuplicates: 1, settingsRestored: ["budgets", "user_rules"] })
        : base(cmd, args);
    render(Transactions, { invoke, today: new Date("2026-11-16T00:00:00Z") });
    await screen.findByText("ACME PAYROLL");
    const before = log.filter(([c]) => c === "transactions").length;
    const file = new File(['{"app":"Golden","backupVersion":1}'], "golden-backup.json", { type: "application/json" });
    await fireEvent.change(screen.getByLabelText("Backup file"), { target: { files: [file] } });
    expect((await screen.findByRole("status")).textContent).toBe("Restored 3 transactions (1 were already here) and your budgets, category rules.");
    expect((log.find(([c]) => c === "restore_backup")![1] as { bytes: number[] }).bytes.length).toBeGreaterThan(10);
    await vi.waitFor(() => expect(log.filter(([c]) => c === "transactions").length).toBeGreaterThan(before));
  });
  it("shows the app's reason when a backup is refused", async () => {
    const base = fake([]);
    const invoke = async (cmd: string, args?: Record<string, unknown>) => {
      if (cmd === "restore_backup") throw "That file isn't a Golden backup.";
      return base(cmd, args);
    };
    render(Transactions, { invoke, today: new Date("2026-11-16T00:00:00Z") });
    await screen.findByText("ACME PAYROLL");
    const file = new File(["nope"], "x.json");
    await fireEvent.change(screen.getByLabelText("Backup file"), { target: { files: [file] } });
    expect((await screen.findByRole("alert")).textContent).toMatch(/isn't a Golden backup/);
  });
  it("shows a friendly message when there is nothing to list", async () => {
    render(Transactions, { invoke: async (cmd: string) => (cmd === "transactions" ? [] : []) });
    expect((await screen.findByText(/Import a statement first/)).textContent).toBeTruthy();
  });
});

describe("Budgets", async () => {
  const { default: Budgets } = await import("./Budgets.svelte");
  const today = new Date("2026-10-20T12:00:00Z");
  const row = (category: string, budgetCents: number, spentCents: number) => ({
    category,
    budgetCents,
    spentCents,
    remainingCents: budgetCents - spentCents,
  });

  it("shows spending against each budget and flags overspending", async () => {
    const invoke = async (cmd: string) =>
      cmd === "budget_status" ? [row("Total", 100000, 10648), row("Subscriptions", 2000, 2648)] : ["Food", "Subscriptions"];
    render(Budgets, { grain: "month", today, invoke });
    const rows = await screen.findAllByTestId("budget-row");
    expect(rows).toHaveLength(2);
    expect(rows[1].textContent).toContain("$26 of $20");
    expect(rows[1].textContent).toContain("$6 over");
    expect(rows[0].textContent).toContain("$894 left");
    expect(screen.getByRole("progressbar", { name: "Subscriptions used" }).getAttribute("aria-valuenow")).toBe("100");
  });

  it("saves a monthly budget in cents and reloads", async () => {
    const calls: [string, Record<string, unknown>?][] = [];
    const invoke = async (cmd: string, args?: Record<string, unknown>) => {
      calls.push([cmd, args]);
      return cmd === "budget_status" ? [] : cmd === "category_choices" ? ["Food"] : undefined;
    };
    render(Budgets, { grain: "month", today, invoke });
    await screen.findByText(/No budgets yet/);
    await fireEvent.change(screen.getByLabelText("Category"), { target: { value: "Food" } });
    await fireEvent.input(screen.getByLabelText("Per month"), { target: { value: "$450.50" } });
    await fireEvent.click(screen.getByRole("button", { name: "Set budget" }));
    await vi.waitFor(() => expect(calls.some(([c]) => c === "set_budget")).toBe(true));
    expect(calls.find(([c]) => c === "set_budget")![1]).toEqual({ category: "Food", monthlyCents: 45050 });
    await vi.waitFor(() => expect(calls.filter(([c]) => c === "budget_status").length).toBe(2));
  });

  it("rejects a non-numeric amount without calling the app", async () => {
    const calls: string[] = [];
    const invoke = async (cmd: string) => {
      calls.push(cmd);
      return cmd === "budget_status" ? [] : [];
    };
    render(Budgets, { grain: "month", today, invoke });
    await screen.findByText(/No budgets yet/);
    await fireEvent.input(screen.getByLabelText("Per month"), { target: { value: "lots" } });
    await fireEvent.click(screen.getByRole("button", { name: "Set budget" }));
    expect((await screen.findByRole("alert")).textContent).toMatch(/dollar amount/);
    expect(calls).not.toContain("set_budget");
  });

  it("stays out of the way in a plain browser", () => {
    render(Budgets, { grain: "month", today });
    expect(screen.queryByTestId("budgets")).toBeNull();
  });
});

describe("Forecast", async () => {
  const { default: ForecastCard } = await import("./Forecast.svelte");
  const month = (m: string, inc: number, sp: number, bal: number) => ({ month: m, incomeCents: inc, spendingCents: sp, netCents: inc - sp, balanceCents: bal });
  const data = {
    startBalanceCents: 900000,
    historyMonths: 12,
    seasonalIncome: true,
    months: [month("2026-11", 0, 0, 900000), month("2026-12", 1000000, 200000, 1700000), month("2027-01", 100000, 300000, 1500000)],
  };

  it("shows the projected months, the starting position and how it was worked out", async () => {
    render(ForecastCard, { invoke: async () => data });
    const rows = await screen.findAllByTestId("forecast-row");
    expect(rows).toHaveLength(3);
    expect(rows[1].textContent).toContain("2026-12");
    expect(rows[1].textContent).toContain("$10,000");
    expect(screen.getByTestId("forecast").textContent).toMatch(/last 12 months of statements, with income placed on the months it usually arrives/);
    expect(screen.getByTestId("forecast").textContent).toContain("$9,000");
    expect(screen.getByRole("img", { name: "Projected position by month" })).toBeTruthy();
  });

  it("marks months that lose money", async () => {
    render(ForecastCard, { invoke: async () => data });
    const rows = await screen.findAllByTestId("forecast-row");
    expect(rows[2].querySelectorAll(".neg").length).toBe(1);
  });

  it("explains when there is not enough history", async () => {
    render(ForecastCard, {
      invoke: async () => {
        throw "Import at least one full month of statements to see a projection.";
      },
    });
    expect((await screen.findByTestId("forecast-note")).textContent).toMatch(/Import at least one full month/);
  });

  it("stays out of the way in a plain browser", () => {
    render(ForecastCard, {});
    expect(screen.queryByTestId("forecast")).toBeNull();
  });

  it("asks the app for the requested number of months", async () => {
    const calls: [string, Record<string, unknown> | undefined][] = [];
    render(ForecastCard, { months: 24, invoke: async (c: string, a?: Record<string, unknown>) => (calls.push([c, a]), data) });
    await screen.findAllByTestId("forecast-row");
    expect(calls[0]).toEqual(["forecast", { months: 24 }]);
  });
});

describe("WhatIf stacking", () => {
  it("offers saved scenarios to build on and sends the choice", async () => {
    const calls: [string, Record<string, unknown> | undefined][] = [];
    const invoke = async (cmd: string, args?: Record<string, unknown>) => {
      calls.push([cmd, args]);
      if (cmd === "list_scenarios") return [{ id: 7, name: "Retire at 62", changes: [], descriptions: [] }];
      if (cmd === "whatif") return { understood: false, message: "I couldn't turn that into a change yet.", changes: [], descriptions: [], before: null, after: null };
      return null;
    };
    render(WhatIf, { invoke });
    const select = (await screen.findByLabelText("Build on")) as HTMLSelectElement;
    await fireEvent.change(select, { target: { value: "7" } });
    await fireEvent.input(screen.getByLabelText("What if question"), { target: { value: "spend $80k" } });
    await fireEvent.click(screen.getByRole("button", { name: "Ask" }));
    await screen.findByText(/couldn't turn that/);
    expect(calls.find(([c]) => c === "whatif")![1]).toMatchObject({ on: 7 });
  });
});

describe("License link notice", () => {
  it("shows what happened when an emailed activation link was opened", async () => {
    render(License, {
      invoke: async (cmd: string) => (cmd === "pricing" ? DEFAULT_PRICING : { state: "licensed", daysLeft: null, canEdit: true, plaidActive: false }),
      notice: "Your licence is active. Thank you!",
    });
    expect((await screen.findByTestId("link-notice")).textContent).toBe("Your licence is active. Thank you!");
  });
});
