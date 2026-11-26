import { describe, expect, it } from "vitest";
import { dollarsToCents, splitLeftCents, splitToParts } from "./splitMath";

describe("splitMath", () => {
  it("reads plain dollar amounts only", () => {
    expect(dollarsToCents("12.5")).toBe(1250);
    expect(dollarsToCents("$3")).toBe(300);
    expect(dollarsToCents("1,000")).toBeNull();
    expect(dollarsToCents("-2")).toBeNull();
    expect(dollarsToCents("")).toBeNull();
  });
  it("tracks what is left on a spending line", () => {
    expect(splitLeftCents(-1050, [{ category: "Food", amount: "7.50" }])).toBe(300);
  });
  it("builds signed parts that add up", () => {
    const r = splitToParts(-1050, [{ category: "Food", amount: "7.50" }, { category: "Fun", amount: "3" }]);
    expect(r).toEqual({ ok: true, parts: [{ category: "Food", cents: -750 }, { category: "Fun", cents: -300 }] });
  });
  it("explains what is wrong", () => {
    expect(splitToParts(-1050, [{ category: "Food", amount: "10.50" }]).ok).toBe(false);
    const short = splitToParts(-1050, [{ category: "Food", amount: "5" }, { category: "Fun", amount: "3" }]);
    expect(short.ok === false && short.error).toContain("add up");
    const blank = splitToParts(-1050, [{ category: "", amount: "5" }, { category: "Fun", amount: "5.50" }]);
    expect(blank.ok === false && blank.error).toContain("category");
  });
});
