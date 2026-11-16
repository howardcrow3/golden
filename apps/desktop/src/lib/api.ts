export interface TaxRequest {
  wagesCents: number;
  filing: "single" | "married_joint" | "married_separate" | "head_of_household";
  state: string;
}
export interface TaxEstimate {
  federalCents: number;
  stateCents: number;
  ficaCents: number;
  totalCents: number;
  effectiveRate: number;
  marginalRate: number;
  stateIsEstimate: boolean;
}

export type Invoke = (cmd: string, args?: Record<string, unknown>) => Promise<unknown>;

/** True when running inside the Tauri shell (as opposed to a plain browser during UI development). */
export function inTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

async function tauriInvoke(): Promise<Invoke> {
  const mod = await import("@tauri-apps/api/core");
  return mod.invoke as Invoke;
}

export async function estimateTax(req: TaxRequest, invoke?: Invoke): Promise<TaxEstimate> {
  const call = invoke ?? (inTauri() ? await tauriInvoke() : null);
  if (!call) throw new Error("The tax engine runs inside the Golden desktop app.");
  return (await call("estimate_tax", { req })) as TaxEstimate;
}

export interface QuickPlanRequest {
  currentYear: number;
  age: number;
  retireAge: number;
  filing: TaxRequest["filing"];
  state: string;
  salaryCents: number;
  contributionCents: number;
  spendingCents: number;
  cashCents: number;
  taxableCents: number;
  traditionalCents: number;
  rothCents: number;
  ssAnnualCents: number;
  picks: Record<string, string>;
  runs?: number;
}
export interface YearPoint {
  year: number;
  age: number;
  balanceCents: number;
  spendingCents: number;
  taxesCents: number;
}
export interface QuickPlanResult {
  successRate: number;
  depletedYear: number | null;
  p10Cents: number;
  p50Cents: number;
  p90Cents: number;
  years: YearPoint[];
  realReturn: number;
  volatility: number;
  horizonAge: number;
  ssClaimAge: number;
}

export async function quickPlan(req: QuickPlanRequest, invoke?: Invoke): Promise<QuickPlanResult> {
  const call = invoke ?? (inTauri() ? await tauriInvoke() : null);
  if (!call) throw new Error("The planning engine runs inside the Golden desktop app.");
  return (await call("quick_plan", { req })) as QuickPlanResult;
}

export interface ImportReport {
  account: string;
  parsed: number;
  added: number;
  skippedDuplicates: number;
  uncategorized: number;
  totalCents: number;
  periodEnd: string | null;
  filedAs: string | null;
}
export interface AccountRow {
  id: number;
  name: string;
  kind: string;
  balanceCents: number;
  openingCents: number;
}
export interface PeriodRow {
  period: string;
  incomeCents: number;
  spendingCents: number;
  netCents: number;
  count: number;
}
export interface CategoryRow {
  category: string;
  cents: number;
}

async function resolve(invoke?: Invoke): Promise<Invoke> {
  const call = invoke ?? (inTauri() ? await tauriInvoke() : null);
  if (!call) throw new Error("Your data lives in the Golden desktop app.");
  return call;
}

export async function importStatement(bytes: Uint8Array, account: string, kind: string, invoke?: Invoke): Promise<ImportReport> {
  const call = await resolve(invoke);
  return (await call("import_statement", { bytes: Array.from(bytes), account, kind })) as ImportReport;
}
export async function listAccounts(invoke?: Invoke): Promise<AccountRow[]> {
  return (await (await resolve(invoke))("accounts")) as AccountRow[];
}
export async function cashFlow(from: string, to: string, grain: string, invoke?: Invoke): Promise<PeriodRow[]> {
  return (await (await resolve(invoke))("cash_flow", { from, to, grain })) as PeriodRow[];
}
export async function categories(day: string, grain: string, income: boolean, invoke?: Invoke): Promise<CategoryRow[]> {
  return (await (await resolve(invoke))("categories", { day, grain, income })) as CategoryRow[];
}

export interface LicenseStatus {
  state: "trial" | "licensed" | "expired";
  daysLeft: number | null;
  canEdit: boolean;
  plaidActive: boolean;
}
export async function licenseStatus(invoke?: Invoke): Promise<LicenseStatus> {
  return (await (await resolve(invoke))("license_status")) as LicenseStatus;
}
export async function activateLicense(token: string, invoke?: Invoke): Promise<LicenseStatus> {
  return (await (await resolve(invoke))("activate_license", { token })) as LicenseStatus;
}

export interface PersonRow {
  name: string;
  birthDate: string;
  retireAge: number;
  ssAnnualCents: number;
}
export interface StreamRow {
  label: string;
  person: number;
  annualCents: number;
  startYear: number | null;
  endYear: number | null;
  contributionCents: number;
}
export interface Household {
  people: PersonRow[];
  streams: StreamRow[];
  filing: TaxRequest["filing"];
  state: string;
  spendingCents: number;
  cashCents: number;
  taxableCents: number;
  traditionalCents: number;
  rothCents: number;
  picks: Record<string, string>;
}
export interface OptionRow {
  label: string;
  value: number;
  likelihood: number;
  probability: number;
  source: string;
}
export interface AssumptionRow {
  key: string;
  options: OptionRow[];
  active: string;
  picked: boolean;
}
export async function getHousehold(invoke?: Invoke): Promise<Household> {
  return (await (await resolve(invoke))("household")) as Household;
}
export async function saveHousehold(household: Household, invoke?: Invoke): Promise<void> {
  await (await resolve(invoke))("save_household", { household });
}
export async function setPick(key: string, label: string, invoke?: Invoke): Promise<Household> {
  return (await (await resolve(invoke))("set_pick", { key, label })) as Household;
}
export async function getAssumptions(invoke?: Invoke): Promise<AssumptionRow[]> {
  return (await (await resolve(invoke))("assumptions")) as AssumptionRow[];
}
export async function runPlan(runs: number, invoke?: Invoke): Promise<QuickPlanResult> {
  return (await (await resolve(invoke))("run_plan", { runs })) as QuickPlanResult;
}

export type Change =
  | { kind: "setPick"; key: string; label: string }
  | { kind: "retireAt"; person: number; age: number }
  | { kind: "spending"; cents: number }
  | { kind: "moveTo"; state: string }
  | { kind: "oneTime"; label: string; person: number; cents: number; year: number };

export interface WhatIfReply {
  understood: boolean;
  message: string;
  changes: Change[];
  descriptions: string[];
  before: QuickPlanResult | null;
  after: QuickPlanResult | null;
}
export async function whatIf(text: string, runs: number, invoke?: Invoke): Promise<WhatIfReply> {
  return (await (await resolve(invoke))("whatif", { text, runs })) as WhatIfReply;
}
export async function keepWhatIf(changes: Change[], invoke?: Invoke): Promise<Household> {
  return (await (await resolve(invoke))("keep_whatif", { changes })) as Household;
}

export async function importFile(name: string, bytes: Uint8Array, account: string, kind: string, invoke?: Invoke): Promise<ImportReport> {
  const call = await resolve(invoke);
  return (await call("import_file", { name, bytes: Array.from(bytes), account, kind })) as ImportReport;
}
export async function libraryFolder(invoke?: Invoke): Promise<string | null> {
  return (await (await resolve(invoke))("library_folder")) as string | null;
}

export interface TxnRow {
  id: number;
  day: string;
  payee: string;
  memo: string;
  amountCents: number;
  category: string | null;
  account: string;
}
export interface Recategorized {
  category: string;
  alsoUpdated: number;
}
export async function listTransactions(from: string, to: string, onlyUncategorized: boolean, invoke?: Invoke): Promise<TxnRow[]> {
  return (await (await resolve(invoke))("transactions", { from, to, onlyUncategorized })) as TxnRow[];
}
export async function categoryChoices(invoke?: Invoke): Promise<string[]> {
  return (await (await resolve(invoke))("category_choices")) as string[];
}
export async function setCategory(id: number, category: string, remember: boolean, invoke?: Invoke): Promise<Recategorized> {
  return (await (await resolve(invoke))("set_category", { id, category, remember })) as Recategorized;
}
export async function exportTransactions(from: string, to: string, invoke?: Invoke): Promise<string> {
  return (await (await resolve(invoke))("export_transactions", { from, to })) as string;
}

export interface BudgetRow {
  category: string;
  budgetCents: number;
  spentCents: number;
  remainingCents: number;
}
/** Monthly budget in cents by category; "*" is the overall budget. */
export const TOTAL_BUDGET = "*";
export async function budgetStatus(day: string, grain: string, invoke?: Invoke): Promise<BudgetRow[]> {
  return (await (await resolve(invoke))("budget_status", { day, grain })) as BudgetRow[];
}
export async function setBudget(category: string, monthlyCents: number, invoke?: Invoke): Promise<void> {
  await (await resolve(invoke))("set_budget", { category, monthlyCents });
}

export async function exportBackup(invoke?: Invoke): Promise<string> {
  return (await (await resolve(invoke))("export_backup")) as string;
}

export interface RestoreReport {
  accounts: number;
  added: number;
  skippedDuplicates: number;
  settingsRestored: string[];
}
export async function restoreBackup(bytes: Uint8Array, invoke?: Invoke): Promise<RestoreReport> {
  return (await (await resolve(invoke))("restore_backup", { bytes: Array.from(bytes) })) as RestoreReport;
}

export interface Pricing {
  currency: string;
  coreCents: number;
  plaidCents: number;
  plaidInterval: "month" | "year";
  corePriceId: string | null;
  plaidPriceId: string | null;
  coreCheckoutUrl: string | null;
  plaidCheckoutUrl: string | null;
  warning: string | null;
}
/** Shown until the app answers (and in a plain browser). Must match crates/fin-license/data/pricing.json. */
export const DEFAULT_PRICING: Pricing = {
  currency: "USD",
  coreCents: 3999,
  plaidCents: 500,
  plaidInterval: "month",
  corePriceId: null,
  plaidPriceId: null,
  coreCheckoutUrl: null,
  plaidCheckoutUrl: null,
  warning: null,
};
export async function getPricing(invoke?: Invoke): Promise<Pricing> {
  try {
    const p = (await (await resolve(invoke))("pricing")) as Pricing;
    return typeof p?.coreCents === "number" && typeof p?.plaidCents === "number" ? p : DEFAULT_PRICING;
  } catch {
    return DEFAULT_PRICING;
  }
}

export async function setLibraryFolder(path: string, invoke?: Invoke): Promise<string> {
  return (await (await resolve(invoke))("set_library_folder", { path })) as string;
}

export async function openCheckout(plan: "core" | "plaid", invoke?: Invoke): Promise<void> {
  await (await resolve(invoke))("open_checkout", { plan });
}

export interface ForecastMonth {
  month: string;
  incomeCents: number;
  spendingCents: number;
  netCents: number;
  balanceCents: number;
}
export interface Forecast {
  startBalanceCents: number;
  historyMonths: number;
  seasonalIncome: boolean;
  months: ForecastMonth[];
}
export async function getForecast(months: number, invoke?: Invoke): Promise<Forecast> {
  return (await (await resolve(invoke))("forecast", { months })) as Forecast;
}

export interface SheetLine {
  name: string;
  kind: string;
  cents: number;
}
export interface BalanceSheet {
  assets: SheetLine[];
  debts: SheetLine[];
  assetsCents: number;
  debtsCents: number;
  netWorthCents: number;
}
export async function getBalanceSheet(invoke?: Invoke): Promise<BalanceSheet> {
  return (await (await resolve(invoke))("balance_sheet")) as BalanceSheet;
}

export interface PaystubSummary {
  employer: string;
  payDate: string | null;
  frequency: string;
  periodsPerYear: number;
  grossCents: number;
  netCents: number;
  federalTaxCents: number;
  stateTaxCents: number;
  socialSecurityCents: number;
  medicareCents: number;
  retirementCents: number;
  benefitsCents: number;
  annualGrossCents: number;
  annualRetirementCents: number;
  incomeTaxRateBp: number;
}
export async function readPaystub(text: string, invoke?: Invoke): Promise<PaystubSummary> {
  return (await (await resolve(invoke))("read_paystub", { text })) as PaystubSummary;
}
export async function applyPaystub(text: string, person: number, invoke?: Invoke): Promise<StreamRow> {
  return (await (await resolve(invoke))("apply_paystub", { text, person })) as StreamRow;
}

export interface AllocationStep {
  name: string;
  cents: number;
  why: string;
}
export interface Allocation {
  yearlySavingsCents: number;
  steps: AllocationStep[];
}
export async function suggestAllocation(yearlySavingsCents: number, invoke?: Invoke): Promise<Allocation> {
  return (await (await resolve(invoke))("suggest_allocation", { yearlySavingsCents })) as Allocation;
}

export async function setOpeningBalance(accountId: number, cents: number, invoke?: Invoke): Promise<void> {
  await (await resolve(invoke))("set_opening_balance", { accountId, cents });
}

export interface Recurring {
  payee: string;
  cadence: string;
  typicalCents: number;
  timesSeen: number;
  lastSeen: string;
  nextExpected: string;
  yearlyCents: number;
}
export async function getRecurring(from: string, to: string, invoke?: Invoke): Promise<Recurring[]> {
  return (await (await resolve(invoke))("recurring", { from, to })) as Recurring[];
}

export interface DebtRow {
  name: string;
  balanceCents: number;
  aprBp: number;
  minCents: number;
  hasTerms: boolean;
}
export interface PaidOff {
  name: string;
  month: number;
}
export interface Payoff {
  feasible: boolean;
  months: number;
  totalInterestCents: number;
  totalPaidCents: number;
  order: PaidOff[];
}
export interface DebtPlan {
  debts: DebtRow[];
  extraCents: number;
  avalanche: Payoff | null;
  snowball: Payoff | null;
  minimumsOnly: Payoff | null;
}
export async function getDebtPlan(extraCents: number, invoke?: Invoke): Promise<DebtPlan> {
  return (await (await resolve(invoke))("debt_plan", { extraCents })) as DebtPlan;
}
export async function setDebtTerms(name: string, aprBp: number, minCents: number, invoke?: Invoke): Promise<void> {
  await (await resolve(invoke))("set_debt_terms", { name, aprBp, minCents });
}

export interface Insight {
  kind: string;
  text: string;
}
export async function getInsights(invoke?: Invoke): Promise<Insight[]> {
  return (await (await resolve(invoke))("insights")) as Insight[];
}

export interface Goal {
  name: string;
  targetCents: number;
  savedCents: number;
  byDate: string;
}
export interface GoalRow extends Goal {
  monthsLeft: number;
  monthlyNeededCents: number;
  percent: number;
  status: "done" | "on_track" | "behind" | "past_due" | "unknown";
  typicalMonthlyCents: number | null;
}
export async function getGoals(invoke?: Invoke): Promise<GoalRow[]> {
  return (await (await resolve(invoke))("goals")) as GoalRow[];
}
export async function setGoal(goal: Goal, invoke?: Invoke): Promise<void> {
  await (await resolve(invoke))("set_goal", { goal });
}
export async function removeGoal(name: string, invoke?: Invoke): Promise<void> {
  await (await resolve(invoke))("remove_goal", { name });
}

export interface NetWorthPoint {
  month: string;
  netWorthCents: number;
}
export async function getNetWorthHistory(months: number, invoke?: Invoke): Promise<NetWorthPoint[]> {
  return (await (await resolve(invoke))("net_worth_history", { months })) as NetWorthPoint[];
}

export interface Gap {
  estimatedCents: number;
  withheldCents: number;
  differenceCents: number;
}
export interface WithholdingCheck {
  taxableWagesCents: number;
  federal: Gap;
  state: Gap;
  missing: string[];
  stateIsEstimate: boolean;
}
export async function getWithholdingCheck(invoke?: Invoke): Promise<WithholdingCheck> {
  return (await (await resolve(invoke))("withholding_check")) as WithholdingCheck;
}
