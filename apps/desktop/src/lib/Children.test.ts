import { describe, expect, it } from "vitest";
import type { Household } from "./api";
import { fromForm, newChild, toForm } from "./household";

const base: Household = {
  people: [{ name: "You", birthDate: "1981-06-15", retireAge: 62, ssAnnualCents: 0 }],
  streams: [], filing: "single", state: "TX", spendingCents: 0, cashCents: 0, taxableCents: 0,
  traditionalCents: 0, rothCents: 0, picks: {}, events: [], spendingRule: "fixed", withdrawalOrder: "standard", medicare: true,
};

describe("children in the household form", () => {
  it("omits children when there are none, so older saves round-trip exactly", () => {
    expect("children" in fromForm(toForm(base))).toBe(false);
  });

  it("round-trips a child through the form", () => {
    const h = { ...base, children: [{ name: "Sam", birthYear: 2024, childcareCents: 1_200_000, collegeCents: 3_000_000 }] };
    expect(fromForm(toForm(h)).children).toEqual(h.children);
  });

  it("names unnamed children and rejects a missing birth year with a readable error", () => {
    const f = toForm(base);
    f.children.push({ ...newChild(2026), birthYear: "2026" });
    expect(fromForm(f).children?.[0].name).toBe("Child 1");
    f.children[0].birthYear = "";
    expect(() => fromForm(f)).toThrow(/birth year/);
    f.children[0].birthYear = "2026";
    f.children[0].childcare = "lots";
    expect(() => fromForm(f)).toThrow(/childcare/);
  });
});

describe("city or county tax in the household form", () => {
  it("is omitted when unset and round-trips when set", () => {
    expect("locality" in fromForm(toForm(base))).toBe(false);
    const h = { ...base, state: "PA", locality: "philadelphia" };
    expect(fromForm(toForm(h)).locality).toBe("philadelphia");
  });
});
