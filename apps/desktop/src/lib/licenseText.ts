import type { LicenseStatus } from "./api";

export type PillTone = "ok" | "soon" | "ended";

/** The short status under the account name in the bottom-left pill, or null until the licence is known. */
export function pillText(l: LicenseStatus | null): { text: string; tone: PillTone } | null {
  if (!l) return null;
  if (l.state === "licensed") return { text: "Licensed", tone: "ok" };
  if (l.state === "expired") return { text: "Trial ended · read-only", tone: "ended" };
  if (l.daysLeft === null) return { text: "Trial starting", tone: "ok" };
  return { text: `Trial · ${l.daysLeft} ${l.daysLeft === 1 ? "day" : "days"} left`, tone: l.daysLeft <= 3 ? "soon" : "ok" };
}

/** The licence block at the top of the account menu. `buy` says whether to offer the Buy button. */
export function menuLicense(l: LicenseStatus | null): { title: string; detail: string; tone: PillTone; buy: boolean } | null {
  if (!l) return null;
  if (l.state === "licensed") return { title: "Licensed", detail: "Thanks for buying Golden.", tone: "ok", buy: false };
  if (l.state === "expired") {
    return { title: "Your free trial has ended", detail: "Golden is read-only: you can still look at and export everything.", tone: "ended", buy: true };
  }
  const left = l.daysLeft === null ? "Your free trial is starting." : `${l.daysLeft} ${l.daysLeft === 1 ? "day" : "days"} left in your free trial.`;
  return { title: "Free trial", detail: `${left} Everything is unlocked.`, tone: l.daysLeft !== null && l.daysLeft <= 3 ? "soon" : "ok", buy: true };
}
