import { describe, expect, it } from "vitest";
import { filterTxns, groupByCategory, sumTxns } from "./txnFilter";
import type { TxnRow } from "./api";

const row = (id: number, payee: string, amountCents: number, category: string | null = null, account = "Checking", day = "2026-10-01"): TxnRow =>
  ({ id, day, payee, memo: "", amountCents, category, account }) as TxnRow;
const rows = [
  row(1, "ACME PAYROLL", 500_000, "Salary"),
  row(2, "Corner Grocery", -4_250, "Groceries", "Visa card", "2026-10-09"),
  row(3, "CORNER BAKERY", -825, null),
];

describe("filterTxns", () => {
  it("returns everything for an empty or blank search", () => {
    expect(filterTxns(rows, "")).toHaveLength(3);
    expect(filterTxns(rows, "   ")).toHaveLength(3);
  });
  it("matches payee text without regard to case", () => {
    expect(filterTxns(rows, "corner").map((r) => r.id)).toEqual([2, 3]);
  });
  it("needs every word to match, in any order", () => {
    expect(filterTxns(rows, "grocery corner").map((r) => r.id)).toEqual([2]);
    expect(filterTxns(rows, "corner payroll")).toEqual([]);
  });
  it("searches category, account, date and amount too", () => {
    expect(filterTxns(rows, "salary").map((r) => r.id)).toEqual([1]);
    expect(filterTxns(rows, "visa").map((r) => r.id)).toEqual([2]);
    expect(filterTxns(rows, "2026-10-09").map((r) => r.id)).toEqual([2]);
    expect(filterTxns(rows, "42.50").map((r) => r.id)).toEqual([2]);
  });
});

describe("sumTxns", () => {
  it("adds money in and out separately", () => {
    expect(sumTxns(rows)).toEqual({ inCents: 500_000, outCents: 5_075, netCents: 494_925 });
  });
  it("is zero for nothing", () => {
    expect(sumTxns([])).toEqual({ inCents: 0, outCents: 0, netCents: 0 });
  });
});

describe("groupByCategory", () => {
  it("puts uncategorised lines first, then categories by size", () => {
    const lanes = groupByCategory(rows);
    expect(lanes.map((l) => l.category)).toEqual([null, "Salary", "Groceries"]);
    expect(lanes[1].totalCents).toBe(500_000);
    expect(lanes[0].rows.map((r) => r.id)).toEqual([3]);
  });
});
