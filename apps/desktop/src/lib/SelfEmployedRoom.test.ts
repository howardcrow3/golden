import { afterEach, describe, expect, it } from "vitest";
import { cleanup, render, screen } from "@testing-library/svelte";
import SelfEmployedRoom from "./SelfEmployedRoom.svelte";
import type { QuickPlanResult } from "./api";

afterEach(cleanup);
const base = { successRate: 1, depletedYear: null, p10Cents: 0, p50Cents: 0, p90Cents: 0, years: [], realReturn: 0.03, volatility: 0.1, horizonAge: 95, ssClaimAge: 67 } as QuickPlanResult;

describe("SelfEmployedRoom", () => {
  it("shows the limits and the unused room", () => {
    render(SelfEmployedRoom, { result: { ...base, selfEmployedLimits: [{ label: "Consulting", netEarningsCents: 10_000_000, solo401kCents: 4_000_000, sepIraCents: 1_800_000, savingCents: 1_000_000 }] } });
    const t = screen.getByTestId("se-room").textContent ?? "";
    expect(t).toContain("Consulting");
    expect(t).toContain("40,000");
    expect(t).toContain("more");
  });
  it("hides without self-employed income", () => {
    expect(render(SelfEmployedRoom, { result: base }).container.querySelector("section")).toBeNull();
  });
});
