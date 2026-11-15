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
