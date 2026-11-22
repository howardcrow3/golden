import { cleanup, render, screen, fireEvent } from "@testing-library/svelte";
import { afterEach, describe, expect, it } from "vitest";
import PlannerScore from "./PlannerScore.svelte";
import { plannerScore } from "./plannerScore";
import type { Household } from "./api";

afterEach(cleanup);
const today = new Date("2026-11-22T12:00:00Z");

const full: Household = {
  people: [{ name: "Ana", birthDate: "1981-06-15", retireAge: 62, ssAnnualCents: 3_800_000 }],
  streams: [{ label: "Salary", person: 0, annualCents: 18_000_000, startYear: null, endYear: null, contributionCents: 0 }],
  filing: "single", state: "TX", spendingCents: 9_000_000, cashCents: 3_000_000, taxableCents: 15_000_000,
  traditionalCents: 60_000_000, rothCents: 12_000_000, picks: { return: "Average" },
  events: [{ label: "Roof", year: 2028, cents: 2_500_000 }],
};

describe("plannerScore", () => {
  it("scores a finished household 100 with no warnings", () => {
    const s = plannerScore(full, today);
    expect(s.score).toBe(100);
    expect(s.items.every((i) => i.done)).toBe(true);
    expect(s.warnings).toEqual([]);
  });
  it("scores an empty household low and says what is missing", () => {
    const empty: Household = {
      people: [{ name: "", birthDate: "", retireAge: 65, ssAnnualCents: 0 }], streams: [], filing: "single",
      state: "", spendingCents: 0, cashCents: 0, taxableCents: 0, traditionalCents: 0, rothCents: 0, picks: {},
    };
    const s = plannerScore(empty, today);
    expect(s.score).toBe(0);
    expect(s.items.filter((i) => !i.done)).toHaveLength(s.items.length);
  });
  it("points add up to 100", () => {
    expect(plannerScore(full, today).items.reduce((a, i) => a + i.points, 0)).toBe(100);
  });
  it("warns about no Social Security near retirement, early retirement penalties, heavy spending and estimate states", () => {
    const h: Household = {
      ...full, state: "NY", spendingCents: 20_000_000,
      people: [{ name: "Ana", birthDate: "1974-01-01", retireAge: 50, ssAnnualCents: 0 }],
      cashCents: 0, taxableCents: 0, rothCents: 0, traditionalCents: 90_000_000,
    };
    const w = plannerScore(h, today).warnings.join("\n");
    expect(w).toContain("no Social Security");
    expect(w).toContain("59½");
    expect(w).toContain("more than 10%");
    expect(w).toContain("NY is an estimate");
  });
  it("rejects impossible dates", () => {
    const h = { ...full, people: [{ ...full.people[0], birthDate: "1981-13-40" }] };
    expect(plannerScore(h, today).items[0].done).toBe(false);
  });
});

describe("PlannerScore", () => {
  it("shows the score and reveals what is missing", async () => {
    const h = { ...full, picks: {}, events: [] };
    render(PlannerScore, { household: h });
    const score = Number(screen.getByTestId("score").textContent);
    expect(score).toBe(plannerScore(h).score);
    expect(screen.queryByText(/Assumptions have been reviewed/)).toBeNull();
    await fireEvent.click(screen.getByRole("button", { name: /what is missing \(2\)/ }));
    expect(screen.getByText(/Assumptions have been reviewed/)).toBeTruthy();
  });
});
