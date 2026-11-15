export type Grain = "week" | "month" | "quarter" | "year";
export const GRAINS: Grain[] = ["week", "month", "quarter", "year"];
export const GRAIN_LABEL: Record<Grain, string> = { week: "Week", month: "Month", quarter: "Quarter", year: "Year" };

/** Average length in 1/4800ths of a day; mirrors fin-core so the UI and engine agree. */
const UNITS: Record<Grain, number> = { week: 33600, month: 146097, quarter: 438291, year: 1753164 };

/** Convert a recurring budget amount (cents) between grains by average day count. */
export function convertBudget(cents: number, from: Grain, to: Grain): number {
  return Math.round((cents * UNITS[to]) / UNITS[from]);
}
