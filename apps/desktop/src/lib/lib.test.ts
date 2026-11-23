import { describe, expect, it, vi } from "vitest";
import { formatCents, formatPercent, parseDollars } from "./money";
import { convertBudget, GRAINS } from "./grain";
import { estimateTax } from "./api";

describe("money", () => {
  it("formats cents", () => {
    expect(formatCents(123456)).toBe("$1,234.56");
    expect(formatCents(-5)).toBe("-$0.05");
    expect(formatCents(0)).toBe("$0.00");
    expect(formatCents(10_000_000_00, { whole: true })).toBe("$10,000,000");
    expect(formatCents(149_487, { whole: true })).toBe("$1,495");
  });
  it("parses typed amounts", () => {
    expect(parseDollars("$1,234.56")).toBe(123456);
    expect(parseDollars("12")).toBe(1200);
    expect(parseDollars("-3.5")).toBe(-350);
    expect(parseDollars("abc")).toBeNull();
    expect(parseDollars("1.234")).toBeNull();
    expect(parseDollars("")).toBeNull();
  });
  it("round-trips", () => {
    for (const c of [0, 1, 99, 100, 123456789, -4242]) expect(parseDollars(formatCents(c))).toBe(c);
  });
  it("formats percentages", () => expect(formatPercent(0.2234)).toBe("22.3%"));
});

describe("grain", () => {
  it("matches the Rust engine's budget conversion", () => {
    expect(convertBudget(100_000, "month", "week")).toBe(22_998);
    expect(convertBudget(100_000, "month", "year")).toBe(1_200_000);
    expect(convertBudget(300_000, "quarter", "month")).toBe(100_000);
  });
  it("is identity for the same grain", () => {
    for (const g of GRAINS) expect(convertBudget(12_345, g, g)).toBe(12_345);
  });
});

describe("api", () => {
  it("calls the estimate_tax command", async () => {
    const invoke = vi.fn().mockResolvedValue({ totalCents: 1 });
    const out = await estimateTax({ wagesCents: 1, filing: "single", state: "TX" }, invoke);
    expect(invoke).toHaveBeenCalledWith("estimate_tax", { req: { wagesCents: 1, filing: "single", state: "TX" } });
    expect(out.totalCents).toBe(1);
  });
  it("explains itself outside the desktop app", async () => {
    await expect(estimateTax({ wagesCents: 1, filing: "single", state: "TX" })).rejects.toThrow(/desktop app/);
  });
});

import { linePoints, verdict } from "./chart";
import { quickPlan } from "./api";

describe("chart", () => {
  it("scales a series into the box", () => {
    const pts = linePoints([0, 50, 100], 100, 50, 0).split(" ");
    expect(pts).toEqual(["0.0,50.0", "50.0,25.0", "100.0,0.0"]);
  });
  it("handles empty and single-point series", () => {
    expect(linePoints([], 10, 10)).toBe("");
    expect(linePoints([5], 100, 50, 0).split(" ")).toHaveLength(1);
  });
  it("reads success rates in plain words", () => {
    expect(verdict(0.95).tone).toBe("good");
    expect(verdict(0.7).tone).toBe("ok");
    expect(verdict(0.2).tone).toBe("bad");
    expect(verdict(0.85).label).toBe("On track");
  });
});

describe("quickPlan api", () => {
  it("calls the quick_plan command", async () => {
    const invoke = vi.fn().mockResolvedValue({ successRate: 0.9 });
    const req = { picks: {} } as never;
    const out = await quickPlan(req, invoke);
    expect(invoke).toHaveBeenCalledWith("quick_plan", { req });
    expect(out.successRate).toBe(0.9);
  });
  it("explains itself outside the desktop app", async () => {
    await expect(quickPlan({} as never)).rejects.toThrow(/desktop app/);
  });
});

import pricingFile from "../../../../crates/fin-license/data/pricing.json";
import { DEFAULT_PRICING } from "./api";
describe("default pricing", () => {
  it("matches the Rust data file", () => {
    expect(DEFAULT_PRICING.currency).toBe(pricingFile.currency);
    expect(DEFAULT_PRICING.coreCents).toBe(pricingFile.coreCents);
    expect(DEFAULT_PRICING.plaidCents).toBe(pricingFile.plaidCents);
    expect(DEFAULT_PRICING.plaidInterval).toBe(pricingFile.plaidInterval);
    expect(DEFAULT_PRICING.plaidYearlyCents).toBe(pricingFile.plaidYearlyCents);
  });
});

// @ts-expect-error plain JS module shared with the hosted checkout page
import * as checkout from "../../../../site/checkout/checkout.js";
describe("hosted checkout page", () => {
  const PRICE = "pri_01m7h6qqb0y4f9r6mftw05y772";
  it("accepts a valid link and defaults the app", () => {
    expect(checkout.parseQuery(`?price=${PRICE}`)).toEqual({ ok: true, priceId: PRICE, app: "turtlebox", email: null });
    expect(checkout.parseQuery(`?price=${PRICE}&app=golden&email=a@b.co`)).toEqual({ ok: true, priceId: PRICE, app: "golden", email: "a@b.co" });
  });
  it("refuses product IDs, junk prices and odd app names with a reason", () => {
    expect(checkout.parseQuery("?price=pro_01m7h6qqb0y4f9r6mftw05y772").error).toMatch(/price ID/);
    for (const q of ["", "?price=", "?price=x", `?price=${PRICE}&app=Bad App`, `?price=${PRICE}&app=<script>`]) {
      expect(checkout.parseQuery(q).ok, q).toBe(false);
    }
  });
  it("drops a malformed email instead of passing it on", () => {
    expect(checkout.parseQuery(`?price=${PRICE}&email=not-an-email`).email).toBeNull();
    expect(checkout.parseQuery(`?price=${PRICE}&email=a b@c.d`).email).toBeNull();
  });
  it("hands Paddle only the price, the app name and a fixed success page", () => {
    const o = checkout.checkoutOptions({ ok: true, priceId: PRICE, app: "golden", email: null }, "https://x.example/thanks.html");
    expect(o).toEqual({
      items: [{ priceId: PRICE, quantity: 1 }],
      customData: { app: "golden" },
      settings: { displayMode: "overlay", theme: "light", successUrl: "https://x.example/thanks.html" },
    });
  });
  it("stays closed until sales are switched on, and never calls Paddle while closed", () => {
    expect(checkout.SALES_OPEN).toBe(false);
    const el = { textContent: "" };
    const calls: string[] = [];
    const win = {
      location: { search: `?price=${PRICE}&app=golden`, href: "https://x.example/checkout/" },
      Paddle: { Initialize: () => calls.push("init"), Checkout: { open: () => calls.push("open") } },
    };
    const doc = { getElementById: (id: string) => (id === "message" ? el : null) };
    (globalThis as unknown as { document: unknown }).document = doc;
    checkout.main(win);
    expect(el.textContent).toMatch(/open soon/);
    expect(calls).toEqual([]);
  });
});
