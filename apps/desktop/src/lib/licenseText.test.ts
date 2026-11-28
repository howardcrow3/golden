import { describe, expect, it } from "vitest";
import type { LicenseStatus } from "./api";
import { menuLicense, pillText } from "./licenseText";

const trial: LicenseStatus = { state: "trial", daysLeft: 10, canEdit: true, plaidActive: true };

describe("licence text", () => {
  it("pill shows days left, then Licensed, then read-only", () => {
    expect(pillText(null)).toBeNull();
    expect(pillText(trial)).toEqual({ text: "Trial · 10 days left", tone: "ok" });
    expect(pillText({ ...trial, daysLeft: 1 })).toEqual({ text: "Trial · 1 day left", tone: "soon" });
    expect(pillText({ ...trial, state: "licensed", daysLeft: null })).toEqual({ text: "Licensed", tone: "ok" });
    expect(pillText({ ...trial, state: "expired", daysLeft: null })?.tone).toBe("ended");
  });
  it("menu offers Buy during and after the trial only", () => {
    expect(menuLicense(null)).toBeNull();
    expect(menuLicense(trial)?.buy).toBe(true);
    expect(menuLicense({ ...trial, state: "expired", daysLeft: null })).toMatchObject({ buy: true, tone: "ended" });
    expect(menuLicense({ ...trial, state: "licensed", daysLeft: null })?.buy).toBe(false);
  });
});
