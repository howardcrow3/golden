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

describe("healthcare figures in the household form", () => {
  it("sends nothing while they are the defaults, and the figures once changed", () => {
    const f = toForm({ ...base, health: true });
    expect("healthDetail" in fromForm(f)).toBe(false);
    f.healthDetail.pre65 = "12,000";
    f.healthDetail.ltcChance = "25";
    const back = fromForm(f);
    expect(back.healthDetail?.pre65Cents).toBe(1_200_000);
    expect(back.healthDetail?.ltcChanceBp).toBe(2500);
    expect(toForm(back).healthDetail.pre65).toBe("12,000");
  });
  it("does not send them when healthcare is off and rejects a bad percentage", () => {
    const f = toForm(base);
    f.healthDetail.pre65 = "1";
    expect("healthDetail" in fromForm(f)).toBe(false);
    f.health = true;
    f.healthDetail.ltcChance = "150";
    expect(() => fromForm(f)).toThrow(/Chance of needing care/);
  });
});

describe("income types in the household form", () => {
  const withStream = (extra: object): Household => ({
    ...base,
    streams: [{ label: "Pension", person: 0, annualCents: 2_400_000, startYear: null, endYear: null, contributionCents: 0, ...extra }],
  });

  it("round-trips a pension, a match and an HSA, and omits them when unset", () => {
    expect(fromForm(toForm(withStream({ kind: "pension" }))).streams[0].kind).toBe("pension");
    expect(fromForm(toForm(withStream({ kind: "public_pension" }))).streams[0].kind).toBe("public_pension");
    expect(fromForm(toForm(withStream({ kind: "military_pension" }))).streams[0].kind).toBe("military_pension");
    const j = fromForm(toForm(withStream({ employerMatchCents: 300_000, hsaCents: 400_000 }))).streams[0];
    expect(j.employerMatchCents).toBe(300_000);
    expect(j.hsaCents).toBe(400_000);
    const plain = fromForm(toForm(withStream({}))).streams[0];
    expect("kind" in plain || "employerMatchCents" in plain || "hsaCents" in plain).toBe(false);
  });

  it("rejects a bad match amount with a readable error", () => {
    const f = toForm(withStream({}));
    f.streams[0].match = "lots";
    expect(() => fromForm(f)).toThrow(/employer match/);
  });
});
