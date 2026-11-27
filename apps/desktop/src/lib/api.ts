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

import { shellReporter, wrapInvoke } from "./clientLog";

export type Invoke = (cmd: string, args?: Record<string, unknown>) => Promise<unknown>;

/** True when running inside the Tauri shell (as opposed to a plain browser during UI development). */
export function inTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

async function tauriInvoke(): Promise<Invoke> {
  const mod = await import("@tauri-apps/api/core");
  const raw = mod.invoke as Invoke;
  // Failed commands land in Golden's local log (command name and error text only).
  return wrapInvoke(raw, shellReporter(raw));
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
  /** Balance percentiles across the simulated runs. */
  p10Cents?: number;
  p50Cents?: number;
  p90Cents?: number;
  /** Income on the return, income tax on it and the federal rate on the next dollar. */
  agiCents?: number;
  incomeTaxCents?: number;
  marginalRate?: number;
  /** Where the year's money came from. */
  earnedCents?: number;
  socialSecurityCents?: number;
  withdrawalsCents?: number;
  rmdCents?: number;
  medicareCents?: number;
}
export interface HistogramBin {
  fromCents: number;
  toCents: number;
  count: number;
}
export interface Milestone {
  key: string;
  label: string;
  year: number;
  age: number;
}
export interface QuickPlanResult {
  successRate: number;
  depletedYear: number | null;
  p10Cents: number;
  p50Cents: number;
  p90Cents: number;
  years: YearPoint[];
  histogram?: HistogramBin[];
  runs?: number;
  seed?: number;
  realReturn: number;
  volatility: number;
  horizonAge: number;
  ssClaimAge: number;
  milestones?: Milestone[];
  legacy?: Legacy;
  alerts?: Alert[];
  selfEmployedLimits?: SelfEmployedLimit[];
}

export interface SelfEmployedLimit {
  label: string;
  netEarningsCents: number;
  solo401kCents: number;
  sepIraCents: number;
  savingCents: number;
}

export interface Alert {
  key: string;
  severity: "good" | "info" | "warn" | "danger";
  message: string;
}

export interface Legacy {
  grossCents: number;
  exemptionCents: number;
  estateTaxCents: number;
  inheritedIraPerYearCents: number;
  heirIncomeTaxCents: number;
  netToHeirsCents: number;
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
  statementBalanceCents?: number | null;
  balanceAfterCents?: number | null;
  reconciled?: boolean | null;
  differenceCents?: number | null;
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

export async function logFolder(invoke?: Invoke): Promise<string | null> {
  const f = await (await resolve(invoke))("log_folder");
  return typeof f === "string" ? f : null;
}
export async function osNotify(count: number, invoke?: Invoke): Promise<void> {
  await (await resolve(invoke))("os_notify", { count });
}
export async function logTail(lines: number, invoke?: Invoke): Promise<string> {
  return String(await (await resolve(invoke))("log_tail", { lines }));
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
/** What happened when a golden://activate link (from the purchase email) was opened. */
export interface LicenseLinkResult {
  ok: boolean;
  message: string;
}
type Listen = (event: string, handler: (e: { payload: LicenseLinkResult }) => void) => Promise<() => void>;
/** Call `handler` whenever the app activated (or failed to activate) a licence from an emailed link. Returns a stop function. */
export async function onLicenseLink(handler: (r: LicenseLinkResult) => void, listen?: Listen): Promise<() => void> {
  let on = listen;
  if (!on) {
    if (!inTauri()) return () => {};
    on = (await import("@tauri-apps/api/event")).listen as unknown as Listen;
  }
  return on("license-link", (e) => handler(e.payload));
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
  /** Business income: self-employment tax instead of FICA, and the QBI deduction. */
  selfEmployed?: boolean;
  /** "pension" or "other" for income without payroll tax; empty for a job. */
  kind?: string;
  employerMatchCents?: number;
  hsaCents?: number;
}
export interface HomeRow {
  valueCents: number;
  basisCents: number;
  mortgageBalanceCents: number;
  /** Hundredths of a percent: 450 is 4.5%. */
  mortgageRateBp: number;
  mortgageYearsLeft: number;
  sellYear?: number | null;
  sellingCostBp: number;
  replacementCostCents: number;
  rentAfterCents: number;
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
  /** Planned one-off costs (positive) or untaxed inflows (negative); set by the what-if chat. */
  events?: EventRow[];
  spendingRule?: string;
  withdrawalOrder?: string;
  medicare?: boolean;
  /** Add estimated healthcare and long-term care costs. */
  health?: boolean;
  /** Yearly charitable giving, part of spending. */
  givingCents?: number;
  /** The home, with a mortgage and an optional sale. */
  home?: HomeRow | null;
  /** Children: childcare, college and the child tax credit. */
  children?: ChildRow[];
  /** Key of a city or county income tax, such as "nyc". */
  locality?: string;
  /** Your own healthcare figures instead of the built-in estimates. */
  healthDetail?: HealthRow;
}
export interface HealthRow {
  pre65Cents: number;
  post65Cents: number;
  excessInflationBp: number;
  ltcStartAge: number;
  ltcYears: number;
  ltcAnnualCents: number;
  ltcChanceBp: number;
}
export interface ChildRow {
  name: string;
  birthYear: number;
  childcareCents: number;
  collegeCents: number;
}
export interface EventRow {
  label: string;
  year: number;
  cents: number;
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
export interface SafeSpend {
  target: number;
  spendingCents: number;
  currentCents: number;
  nothingSafe: boolean;
  capped: boolean;
}

export async function safeSpend(target: number, invoke?: Invoke): Promise<SafeSpend> {
  return (await (await resolve(invoke))("safe_spend", { target })) as SafeSpend;
}

export interface RothRow {
  label: string;
  convertedCents: number;
  lifetimeTaxCents: number;
  finalBalanceCents: number;
  legacyCents: number;
  successRate: number;
  depletedYear: number | null;
}

export async function rothExplore(untilAge: number, invoke?: Invoke): Promise<RothRow[]> {
  return (await (await resolve(invoke))("roth_explore", { untilAge })) as RothRow[];
}

export interface SsRow {
  claimAge: number;
  annualCents: number[];
  lifetimeCents: number;
  legacyCents: number;
  successRate: number;
  depletedYear: number | null;
}
export interface SsExplore {
  rows: SsRow[];
  breakEvens: { earlier: number; later: number; age: number | null }[];
}

export async function ssExplore(invoke?: Invoke): Promise<SsExplore> {
  return (await (await resolve(invoke))("ss_explore")) as SsExplore;
}

export interface BacktestWindow {
  startYear: number;
  succeeded: boolean;
  finalCents: number;
  depletedYear: number | null;
  annualisedReturn: number;
}
export interface BacktestResult {
  stockShare: number;
  years: number;
  successRate: number;
  firstYear: number;
  lastYear: number;
  windows: BacktestWindow[];
  worst: number[];
}

export async function backtest(stockShare: number, invoke?: Invoke): Promise<BacktestResult> {
  return (await (await resolve(invoke))("backtest", { stockShare })) as BacktestResult;
}

export interface MixRow {
  stockShare: number;
  successRate: number;
  worstYear: number;
  worstFinalCents: number;
  medianFinalCents: number;
}
export async function backtestCompare(shares: number[], invoke?: Invoke): Promise<MixRow[]> {
  return (await (await resolve(invoke))("backtest_compare", { shares })) as MixRow[];
}

export interface BootstrapResult {
  successRate: number;
  p10Cents: number;
  p50Cents: number;
  p90Cents: number;
  runs: number;
  blockYears: number;
  stockShare: number;
}

export async function bootstrap(stockShare: number, blockYears: number, runs: number, invoke?: Invoke): Promise<BootstrapResult> {
  return (await (await resolve(invoke))("bootstrap", { stockShare, blockYears, runs })) as BootstrapResult;
}

export async function runPlan(runs: number, invoke?: Invoke): Promise<QuickPlanResult> {
  return (await (await resolve(invoke))("run_plan", { runs })) as QuickPlanResult;
}

export type Change =
  | { kind: "setPick"; key: string; label: string }
  | { kind: "retireAt"; person: number; age: number }
  | { kind: "spending"; cents: number }
  | { kind: "moveTo"; state: string }
  | { kind: "oneTime"; label: string; person: number; cents: number; year: number }
  | { kind: "raise"; person: number; bp: number }
  | { kind: "plannedCost"; label: string; cents: number; year: number }
  | { kind: "setBalance"; account: "cash" | "taxable" | "traditional" | "roth"; cents: number }
  | { kind: "spendingRule"; rule: "fixed" | "guardrails" | "percent" | "vpw" }
  | { kind: "withdrawalOrder"; order: "standard" | "traditional_first" | "roth_first" | "blended" };

export interface WhatIfReply {
  understood: boolean;
  message: string;
  changes: Change[];
  descriptions: string[];
  before: QuickPlanResult | null;
  after: QuickPlanResult | null;
}
export async function whatIf(text: string, runs: number, invoke?: Invoke, on?: number): Promise<WhatIfReply> {
  return (await (await resolve(invoke))("whatif", { text, runs, on: on ?? null })) as WhatIfReply;
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
  note: string;
  tags: string[];
  splits: { category: string; cents: number }[];
}
export interface Recategorized {
  category: string;
  alsoUpdated: number;
}
export async function listTransactions(from: string, to: string, onlyUncategorized: boolean, invoke?: Invoke): Promise<TxnRow[]> {
  return (await (await resolve(invoke))("transactions", { from, to, onlyUncategorized })) as TxnRow[];
}
/** Every line in the range, with no 500-line cap; for totals. */
export async function listAllTransactions(from: string, to: string, invoke?: Invoke): Promise<TxnRow[]> {
  return (await (await resolve(invoke))("all_transactions", { from, to })) as TxnRow[];
}
export async function categoryChoices(invoke?: Invoke): Promise<string[]> {
  return (await (await resolve(invoke))("category_choices")) as string[];
}
export async function setCategory(id: number, category: string, remember: boolean, invoke?: Invoke): Promise<Recategorized> {
  return (await (await resolve(invoke))("set_category", { id, category, remember })) as Recategorized;
}
export async function setNoteTags(id: number, note: string, tags: string, invoke?: Invoke): Promise<void> {
  await (await resolve(invoke))("set_note_tags", { id, note, tags });
}
export async function setSplits(id: number, parts: { category: string; cents: number }[], invoke?: Invoke): Promise<void> {
  await (await resolve(invoke))("set_splits", { id, parts });
}
export async function exportTransactions(from: string, to: string, invoke?: Invoke): Promise<string> {
  return (await (await resolve(invoke))("export_transactions", { from, to })) as string;
}

/** Save a report, table or data file into the Exports folder; returns where it went. */
export async function saveReport(
  stem: string,
  kind: "html" | "csv" | "json",
  content: string,
  invoke?: Invoke,
): Promise<string> {
  return (await (await resolve(invoke))("save_report", { stem, kind, content })) as string;
}

export interface BudgetRow {
  category: string;
  budgetCents: number;
  spentCents: number;
  remainingCents: number;
  rolloverCents?: number;
  rolls?: boolean;
}
/** Monthly budget in cents by category; "*" is the overall budget. */
export const TOTAL_BUDGET = "*";
export async function budgetStatus(day: string, grain: string, invoke?: Invoke): Promise<BudgetRow[]> {
  return (await (await resolve(invoke))("budget_status", { day, grain })) as BudgetRow[];
}
export async function setBudgetRollover(category: string, on: boolean, invoke?: Invoke): Promise<void> {
  await (await resolve(invoke))("set_budget_rollover", { category, on });
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
  plaidYearlyCents: number;
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
  plaidCents: 997,
  plaidInterval: "month",
  plaidYearlyCents: 10000,
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
export const HOLDING_KINDS = ["Vehicle", "Crypto", "Private stock", "Collectible", "Precious metals", "Property", "Other asset"] as const;
export async function addHolding(name: string, kind: string, valueCents: number, invoke?: Invoke): Promise<void> {
  await (await resolve(invoke))("add_holding", { name, kind, valueCents });
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
export interface WatchList {
  name: string;
  query: string;
}
export async function getWatchLists(invoke?: Invoke): Promise<WatchList[]> {
  return (await (await resolve(invoke))("watch_lists")) as WatchList[];
}
export async function saveWatchLists(lists: WatchList[], invoke?: Invoke): Promise<WatchList[]> {
  return (await (await resolve(invoke))("save_watch_lists", { lists })) as WatchList[];
}
export async function getCancelMarks(invoke?: Invoke): Promise<string[]> {
  return (await (await resolve(invoke))("cancel_marks")) as string[];
}
export async function setCancelMark(payee: string, on: boolean, invoke?: Invoke): Promise<string[]> {
  return (await (await resolve(invoke))("set_cancel_mark", { payee, on })) as string[];
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
export type CareKind = "nursing_home_private" | "nursing_home_semi_private" | "assisted_living" | "adult_day" | "caregiver";
export interface CareCost {
  annualCents: number;
  national: boolean;
}
export async function getCareCost(state: string, kind: CareKind, invoke?: Invoke): Promise<CareCost> {
  return (await (await resolve(invoke))("care_cost", { state, kind })) as CareCost;
}

export interface OffsetEffect {
  monthsWithout: number;
  monthsWith: number;
  interestWithoutCents: number;
  interestWithCents: number;
  interestSavedCents: number;
}
export async function getMortgageOffset(
  balanceCents: number,
  rateBp: number,
  yearsLeft: number,
  offsetCents: number,
  invoke?: Invoke,
): Promise<OffsetEffect> {
  return (await (await resolve(invoke))("mortgage_offset", { balanceCents, rateBp, yearsLeft, offsetCents })) as OffsetEffect;
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
export interface ProgressPoint {
  month: string;
  actualChangeCents: number;
  projectedChangeCents: number;
}
export interface ProgressReport {
  started: string;
  points: ProgressPoint[];
  aheadCents: number;
  journal?: JournalPoint[];
}
export interface JournalPoint {
  date: string;
  note: string;
  savingsCents: number;
  projectedCents: number;
  aheadCents: number;
}
export async function addJournalEntry(date: string, savingsCents: number, note: string, invoke?: Invoke): Promise<void> {
  await (await resolve(invoke))("add_journal_entry", { date, savingsCents, note });
}
export async function removeJournalEntry(date: string, invoke?: Invoke): Promise<void> {
  await (await resolve(invoke))("remove_journal_entry", { date });
}
export async function getProgress(invoke?: Invoke): Promise<ProgressReport | null> {
  const r = (await (await resolve(invoke))("progress")) as ProgressReport | null;
  return r && Array.isArray(r.points) ? r : null;
}
export async function startProgress(invoke?: Invoke): Promise<void> {
  await (await resolve(invoke))("start_progress");
}
export async function stopProgress(invoke?: Invoke): Promise<void> {
  await (await resolve(invoke))("stop_progress");
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

export interface AppInfo {
  name: string;
  version: string;
  /** The CI build number, or "dev" for a local build. */
  build: string;
  commit: string;
  mode: string;
}
export async function appInfo(invoke?: Invoke): Promise<AppInfo> {
  return (await (await resolve(invoke))("app_info")) as AppInfo;
}

export interface ModelSettings {
  endpoint: string;
  name: string;
}
export async function modelSettings(invoke?: Invoke): Promise<ModelSettings> {
  return (await (await resolve(invoke))("model_settings")) as ModelSettings;
}
export async function setModel(endpoint: string, name: string, invoke?: Invoke): Promise<ModelSettings> {
  return (await (await resolve(invoke))("set_model", { endpoint, name })) as ModelSettings;
}

export interface SetupStep {
  key: string;
  title: string;
  done: boolean;
  /** Sidebar page that does it. */
  page: string;
}
export interface Setup {
  steps: SetupStep[];
  done: number;
  complete: boolean;
}
export async function setupProgress(invoke?: Invoke): Promise<Setup> {
  return (await (await resolve(invoke))("setup_progress")) as Setup;
}

export interface ModelStatus {
  ramGb: number;
  id: string;
  name: string;
  sizeMb: number;
  installed: boolean;
  path: string;
}
export interface ModelProgress {
  running: boolean;
  downloadedMb: number;
  totalMb: number;
  error: string;
}
export async function modelStatus(invoke?: Invoke): Promise<ModelStatus> {
  return (await (await resolve(invoke))("model_status")) as ModelStatus;
}
export async function startModelDownload(id: string, invoke?: Invoke): Promise<void> {
  await (await resolve(invoke))("start_model_download", { id });
}
export async function modelProgress(invoke?: Invoke): Promise<ModelProgress> {
  return (await (await resolve(invoke))("model_progress")) as ModelProgress;
}
export async function cancelModelDownload(invoke?: Invoke): Promise<void> {
  await (await resolve(invoke))("cancel_model_download");
}
export async function removeModel(id: string, invoke?: Invoke): Promise<void> {
  await (await resolve(invoke))("remove_model", { id });
}

export interface CategorySuggestion {
  id: number;
  payee: string;
  category: string;
}
export async function suggestCategories(invoke?: Invoke): Promise<CategorySuggestion[]> {
  return (await (await resolve(invoke))("suggest_categories")) as CategorySuggestion[];
}

export interface ExplainFacts {
  successRate: number;
  depletedYear: number | null;
  retireAge: number | null;
  horizonAge: number | null;
  compare: { beforeRate: number; afterRate: number; change: string } | null;
}
export interface Explanation {
  text: string;
  /** "model" or "built-in". */
  source: string;
}
export async function explainResult(facts: ExplainFacts, invoke?: Invoke): Promise<Explanation> {
  return (await (await resolve(invoke))("explain_result", { facts })) as Explanation;
}

export interface AssumptionSuggestion {
  key: string;
  label: string;
  reason: string;
  /** "model" or "built-in". */
  source: string;
}
export async function suggestAssumptions(invoke?: Invoke): Promise<AssumptionSuggestion[]> {
  return (await (await resolve(invoke))("suggest_assumptions")) as AssumptionSuggestion[];
}

export interface AiActivityEntry {
  at: string;
  task: string;
  ok: boolean;
  millis: number;
  promptChars: number;
  replyChars: number;
  error: string | null;
}
export async function aiActivity(invoke?: Invoke): Promise<AiActivityEntry[]> {
  return (await (await resolve(invoke))("ai_activity")) as AiActivityEntry[];
}
export async function clearAiActivity(invoke?: Invoke): Promise<void> {
  await (await resolve(invoke))("clear_ai_activity");
}

export interface Connection {
  name: string;
  host: string;
  sends: string;
  inUse: boolean;
  when: string;
}
export interface Connections {
  private: boolean;
  items: Connection[];
}
export async function connections(invoke?: Invoke): Promise<Connections> {
  return (await (await resolve(invoke))("connections")) as Connections;
}
export async function eraseAllData(confirm: string, invoke?: Invoke): Promise<void> {
  await (await resolve(invoke))("erase_all_data", { confirm });
}

export interface FolderReport {
  found: number;
  alreadyImported: number;
  imported: ImportReport[];
  failed: { name: string; error: string }[];
}
export async function importFolder(path: string, account: string, kind: string, invoke?: Invoke): Promise<FolderReport> {
  return (await (await resolve(invoke))("import_folder", { path, account, kind })) as FolderReport;
}

export interface Watch {
  path: string;
  account: string;
  kind: string;
}
export async function watchedFolder(invoke?: Invoke): Promise<Watch | null> {
  const w = (await (await resolve(invoke))("watched_folder")) as Watch | null;
  return w && typeof w.path === "string" ? w : null;
}
export async function watchFolder(path: string, account: string, kind: string, invoke?: Invoke): Promise<Watch> {
  return (await (await resolve(invoke))("watch_folder", { path, account, kind })) as Watch;
}
export async function stopWatching(invoke?: Invoke): Promise<void> {
  await (await resolve(invoke))("stop_watching");
}

export interface SavedScenario {
  id: number;
  name: string;
  changes: Change[];
  descriptions: string[];
}
export interface CompareColumn {
  name: string;
  descriptions: string[];
  result: QuickPlanResult;
}
export async function listScenarios(invoke?: Invoke): Promise<SavedScenario[]> {
  return (await (await resolve(invoke))("list_scenarios")) as SavedScenario[];
}
export async function saveScenario(name: string, changes: Change[], invoke?: Invoke): Promise<SavedScenario> {
  return (await (await resolve(invoke))("save_scenario", { name, changes })) as SavedScenario;
}
export async function deleteScenario(id: number, invoke?: Invoke): Promise<void> {
  await (await resolve(invoke))("delete_scenario", { id });
}
export async function renameScenario(id: number, name: string, invoke?: Invoke): Promise<SavedScenario> {
  return (await (await resolve(invoke))("rename_scenario", { id, name })) as SavedScenario;
}
export async function duplicateScenario(id: number, invoke?: Invoke): Promise<SavedScenario> {
  return (await (await resolve(invoke))("duplicate_scenario", { id })) as SavedScenario;
}
export async function moveScenario(id: number, by: number, invoke?: Invoke): Promise<SavedScenario[]> {
  return (await (await resolve(invoke))("move_scenario", { id, by })) as SavedScenario[];
}
export async function compareScenarios(ids: number[], runs: number, invoke?: Invoke): Promise<{ columns: CompareColumn[] }> {
  return (await (await resolve(invoke))("compare_scenarios", { ids, runs })) as { columns: CompareColumn[] };
}
export async function makePlan(id: number, invoke?: Invoke): Promise<Household> {
  return (await (await resolve(invoke))("make_plan", { id })) as Household;
}
export async function undoablePlan(invoke?: Invoke): Promise<string | null> {
  return (await (await resolve(invoke))("undoable_plan")) as string | null;
}
export async function undoMakePlan(invoke?: Invoke): Promise<Household> {
  return (await (await resolve(invoke))("undo_make_plan")) as Household;
}

export interface AskAnswer {
  understood: boolean;
  text: string;
  facts: { label: string; cents: number }[];
  behind: { id: number; day: string; payee: string; memo: string; amountCents: number; category: string | null; account: string }[];
}
export async function ask(question: string, invoke?: Invoke): Promise<AskAnswer> {
  return (await (await resolve(invoke))("ask", { question })) as AskAnswer;
}

export interface CloseCheck {
  kind: string;
  text: string;
}
export interface CloseReport {
  month: string;
  closed: boolean;
  finished: boolean;
  incomeCents: number;
  spendingCents: number;
  netCents: number;
  count: number;
  issues: CloseCheck[];
  summary: string;
}
export async function monthCloseCheck(month: string, invoke?: Invoke): Promise<CloseReport> {
  return (await (await resolve(invoke))("month_close_check", { month })) as CloseReport;
}
export async function closeMonth(month: string, invoke?: Invoke): Promise<CloseReport> {
  return (await (await resolve(invoke))("close_month", { month })) as CloseReport;
}
export async function reopenMonth(month: string, invoke?: Invoke): Promise<CloseReport> {
  return (await (await resolve(invoke))("reopen_month", { month })) as CloseReport;
}

export interface SeppRow {
  method: "required_minimum" | "fixed_amortization";
  yearlyCents: number;
  level: boolean;
  earliestStopAge: number;
}

export async function sepp(balanceCents: number, age: number, rateBp: number, invoke?: Invoke): Promise<SeppRow[]> {
  return (await (await resolve(invoke))("sepp", { balanceCents, age, rateBp })) as SeppRow[];
}

export interface Bunching {
  years: number;
  taxEachYearCents: number;
  taxBunchedCents: number;
  savedCents: number;
  itemizesAnyway: boolean;
}

export async function dafBunching(
  filing: string,
  incomeCents: number,
  otherItemizedCents: number,
  givingCents: number,
  years: number,
  invoke?: Invoke,
): Promise<Bunching> {
  return (await (await resolve(invoke))("daf_bunching", { filing, incomeCents, otherItemizedCents, givingCents, years })) as Bunching;
}

/** City and county income taxes the plan knows (mirrors the engine's list). */
export const LOCALITIES: { key: string; name: string; state: string }[] = [
  { key: "nyc", name: "New York City (resident)", state: "NY" },
  { key: "philadelphia", name: "Philadelphia (resident wage tax)", state: "PA" },
  { key: "pittsburgh", name: "Pittsburgh (resident)", state: "PA" },
  { key: "columbus", name: "Columbus, Ohio", state: "OH" },
  { key: "cleveland", name: "Cleveland, Ohio", state: "OH" },
  { key: "detroit", name: "Detroit (resident)", state: "MI" },
  { key: "baltimore", name: "Baltimore City", state: "MD" },
  { key: "montgomery_md", name: "Montgomery County, Maryland", state: "MD" },
];

export interface RetireWhen {
  target: number;
  age: number | null;
  currentAge: number;
  tries: { age: number; successRate: number }[];
}

export async function retireWhen(target: number, invoke?: Invoke): Promise<RetireWhen> {
  return (await (await resolve(invoke))("retire_when", { target })) as RetireWhen;
}

export interface RentBuyRequest {
  priceCents: number;
  downPaymentCents: number;
  mortgageRateBp: number;
  mortgageYears: number;
  carryingBp: number;
  upkeepBp: number;
  appreciationBp: number;
  sellingCostBp: number;
  monthlyRentCents: number;
  rentGrowthBp: number;
  investReturnBp: number;
  years: number;
}

export interface RentBuyOutcome {
  years: number;
  buyNetWorthCents: number;
  rentNetWorthCents: number;
  buyAdvantageCents: number;
  monthlyPaymentCents: number;
  breakEvenYear: number | null;
}

export async function rentVsBuy(req: RentBuyRequest, invoke?: Invoke): Promise<RentBuyOutcome> {
  return (await (await resolve(invoke))("rent_vs_buy", { req })) as RentBuyOutcome;
}

export interface HarvestPlan {
  zeroRateRoomCents: number;
  taxSavedCents: number;
  acaCliffCents: number;
  acaRoomCents: number;
  gainCrossesAcaCliff: boolean;
}

export async function harvestPlan(filing: string, incomeCents: number, gainsCents: number, householdSize: number, invoke?: Invoke): Promise<HarvestPlan> {
  return (await (await resolve(invoke))("harvest_plan", { filing, incomeCents, gainsCents, householdSize })) as HarvestPlan;
}

export type AssetClass = "stock" | "bond" | "cash" | "other";
export type RiskProfile = "conservative" | "moderate" | "aggressive";
export interface Holding {
  name: string;
  valueCents: number;
  class: AssetClass;
  expenseBp: number;
  singleStock: boolean;
}
export interface Checkup {
  totalCents: number;
  mix: { class: AssetClass; share: number; target: number }[];
  stockGap: number;
  weightedFee: number;
  yearlyFeeCents: number;
  feeCost20yCents: number;
  flags: { name: string; share: number; reason: string }[];
}
export async function getHoldings(invoke?: Invoke): Promise<Holding[]> {
  return (await (await resolve(invoke))("holdings")) as Holding[];
}
/** Read a broker positions CSV into holdings (nothing is saved). */
export async function parseHoldings(text: string, invoke?: Invoke): Promise<Holding[]> {
  return (await (await resolve(invoke))("parse_holdings", { text })) as Holding[];
}
export async function saveHoldings(holdings: Holding[], invoke?: Invoke): Promise<void> {
  await (await resolve(invoke))("save_holdings", { holdings });
}
export async function investmentCheckup(risk: RiskProfile, invoke?: Invoke): Promise<Checkup> {
  return (await (await resolve(invoke))("investment_checkup", { risk })) as Checkup;
}

export async function exportPlanFile(invoke?: Invoke): Promise<string> {
  return (await (await resolve(invoke))("export_plan_file")) as string;
}
export async function importPlanFile(text: string, invoke?: Invoke): Promise<{ people: number; holdings: number }> {
  return (await (await resolve(invoke))("import_plan_file", { text })) as { people: number; holdings: number };
}

export interface VaultDoc {
  id: number;
  name: string;
  kind: string;
  year: number | null;
  added: string;
  size: number;
  fields: { label: string; cents: number }[];
  hasText?: boolean;
}
export async function vaultAdd(name: string, bytes: Uint8Array, invoke?: Invoke): Promise<VaultDoc> {
  return (await (await resolve(invoke))("vault_add", { name, bytes: Array.from(bytes) })) as VaultDoc;
}
export async function vaultList(query: string, invoke?: Invoke): Promise<VaultDoc[]> {
  return (await (await resolve(invoke))("vault_list", { query })) as VaultDoc[];
}
export async function vaultDelete(id: number, invoke?: Invoke): Promise<void> {
  await (await resolve(invoke))("vault_delete", { id });
}
export async function vaultExport(id: number, invoke?: Invoke): Promise<string> {
  return (await (await resolve(invoke))("vault_export", { id })) as string;
}

export interface BankItem {
  itemId: string;
  institution: string;
  cursor: string;
  /** "active", "paused" or "login_required" */
  status: string;
}
export interface BankSyncReport {
  added: number;
  updated: number;
  removed: number;
  skipped: number;
  reconciled: number;
}
export interface BankOutcome {
  bank: string;
  report: BankSyncReport | null;
  error: string | null;
}
export interface BankLinkReturn {
  publicToken: string;
  institution: string;
}
export async function bankItems(invoke?: Invoke): Promise<BankItem[]> {
  return (await (await resolve(invoke))("bank_items")) as BankItem[];
}
/** Opens the bank sign-in page in the browser; the app is sent back a link when it finishes. */
export async function bankConnect(invoke?: Invoke): Promise<void> {
  await (await resolve(invoke))("bank_connect");
}
export async function bankFinishLink(publicToken: string, institution: string, invoke?: Invoke): Promise<BankSyncReport> {
  return (await (await resolve(invoke))("bank_finish_link", { publicToken, institution })) as BankSyncReport;
}
export async function bankSyncAll(invoke?: Invoke): Promise<BankOutcome[]> {
  return (await (await resolve(invoke))("bank_sync_all")) as BankOutcome[];
}
export async function bankSetPaused(itemId: string, paused: boolean, invoke?: Invoke): Promise<void> {
  await (await resolve(invoke))("bank_set_paused", { itemId, paused });
}
export async function bankUnlink(itemId: string, invoke?: Invoke): Promise<number> {
  return (await (await resolve(invoke))("bank_unlink", { itemId })) as number;
}
type BankListen = (event: string, handler: (e: { payload: BankLinkReturn }) => void) => Promise<() => void>;
/** Call `handler` when the person comes back from their bank's sign-in page. Returns a stop function. */
export async function onBankLink(handler: (r: BankLinkReturn) => void, listen?: BankListen): Promise<() => void> {
  let on = listen;
  if (!on) {
    if (!inTauri()) return () => {};
    on = (await import("@tauri-apps/api/event")).listen as unknown as BankListen;
  }
  return on("plaid-link", (e) => handler(e.payload));
}

export interface VestResult {
  valueCents: number;
  withheldCents: number;
  sharesSold: number;
  sharesKept: number;
  cashBackCents: number;
}
export async function getRsuVest(shares: number, priceCents: number, federalRate: number, otherRate: number, invoke?: Invoke): Promise<VestResult> {
  return (await (await resolve(invoke))("rsu_vest", { shares, priceCents, federalRate, otherRate })) as VestResult;
}
export interface EsppResult {
  purchasePriceCents: number;
  shares: number;
  leftoverCents: number;
  discountGainCents: number;
}
export async function getEsppPurchase(
  contributionsCents: number,
  offeringCents: number,
  purchaseCents: number,
  discount: number,
  lookback: boolean,
  invoke?: Invoke,
): Promise<EsppResult> {
  return (await (await resolve(invoke))("espp_purchase", { contributionsCents, offeringCents, purchaseCents, discount, lookback })) as EsppResult;
}

/** "Send feedback". Sending and signing happen in the desktop shell. */
export interface FeedbackInput {
  rating: number | null;
  category: "feature" | "issue" | "praise" | "other";
  message: string;
  email: string | null;
  /** "Send anonymously": no email is sent, and the service keeps none. */
  anonymous: boolean;
}

const EMAIL_RE = /^[^\s@"<>()[\],;:\\]+@(?:[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?\.)+[a-z]{2,63}$/i;
/** Blank, or an address in the shape the feedback service accepts (it checks again). */
export function feedbackEmailOk(email: string): boolean {
  const e = email.trim();
  return !e || (e.length <= 254 && EMAIL_RE.test(e) && !/^\.|\.\.|\.@/.test(e));
}

/** True when this build can send feedback (it carries a feedback key). */
export async function feedbackAvailable(invoke?: Invoke): Promise<boolean> {
  try {
    return (await (await resolve(invoke))("feedback_available")) === true;
  } catch {
    return false;
  }
}

export async function sendFeedback(input: FeedbackInput, invoke?: Invoke): Promise<void> {
  await (await resolve(invoke))("send_feedback", { input });
}

export interface SsaSummary {
  monthlyAt62Cents: number | null;
  monthlyAtFullRetirementCents: number | null;
  fullRetirementAge: number | null;
  monthlyAt70Cents: number | null;
}
export async function readSsaStatement(text: string, invoke?: Invoke): Promise<SsaSummary> {
  return (await (await resolve(invoke))("read_ssa_statement", { text })) as SsaSummary;
}
/** Sets a person's yearly Social Security; `claim` is "62", "fra" or "70". Returns the yearly cents. */
export async function applySsaStatement(text: string, person: number, claim: string, invoke?: Invoke): Promise<number> {
  return (await (await resolve(invoke))("apply_ssa_statement", { text, person, claim })) as number;
}
