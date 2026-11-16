import type { AssumptionRow } from "./api";

export const ASSUMPTION_NAMES: Record<string, string> = {
  real_return: "Investment return (after inflation)",
  return_volatility: "Market ups and downs",
  horizon_age: "Plan until age",
  ss_claim_age: "Claim Social Security at",
  spending_change: "Spending in retirement",
};

export function assumptionName(key: string): string {
  return ASSUMPTION_NAMES[key] ?? key.replace(/_/g, " ");
}

/** "Cautious · 25% likely" */
export function optionText(o: AssumptionRow["options"][number]): string {
  return `${o.label} · ${Math.round(o.probability * 100)}% likely`;
}
