import { afterEach, describe, expect, it } from "vitest";
import { cleanup, render, screen } from "@testing-library/svelte";
import Milestones from "./Milestones.svelte";
import type { QuickPlanResult } from "./api";

afterEach(cleanup);

const base = { successRate: 1, depletedYear: null, p10Cents: 0, p50Cents: 0, p90Cents: 0, years: [], realReturn: 0.03, volatility: 0.1, horizonAge: 95, ssClaimAge: 67 } as QuickPlanResult;

describe("Milestones", () => {
  it("lists each milestone with its year and age", () => {
    render(Milestones, { result: { ...base, milestones: [{ key: "retire", label: "Work ends", year: 2040, age: 65 }, { key: "ss", label: "Social Security starts", year: 2042, age: 67 }] } });
    expect(screen.getByTestId("ms-retire").textContent).toContain("2040");
    expect(screen.getByTestId("ms-retire").textContent).toContain("age 65");
    expect(screen.getByTestId("ms-ss").textContent).toContain("Social Security starts");
  });
  it("shows nothing without milestones", () => {
    render(Milestones, { result: { ...base, milestones: [] } });
    expect(screen.queryByLabelText("Milestones")).toBeNull();
  });
  it("copes with an older result that has no milestones", () => {
    render(Milestones, { result: base });
    expect(screen.queryByLabelText("Milestones")).toBeNull();
  });
});
