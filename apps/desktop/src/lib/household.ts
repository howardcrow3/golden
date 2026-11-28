import type { EventRow, Household, PersonRow, StreamRow } from "./api";
import { formatCents, parseDollars } from "./money";

/** Editable text version of a household: money and years are strings while being typed. */
export interface Form {
  people: { name: string; birthDate: string; retireAge: number; ss: string }[];
  streams: { label: string; person: number; amount: string; start: string; end: string; contribution: string; selfEmployed: boolean; kind: string; match: string; hsa: string }[];
  filing: Household["filing"];
  state: string;
  spending: string;
  cash: string;
  taxable: string;
  traditional: string;
  roth: string;
  picks: Record<string, string>;
  /** Not edited here, but kept so saving the form does not drop what the what-if chat added. */
  events: EventRow[];
  spendingRule: string;
  withdrawalOrder: string;
  medicare: boolean;
  health: boolean;
  giving: string;
  /** The home, when it is part of the plan. */
  home: HomeForm | null;
  children: ChildForm[];
  locality: string;
  healthDetail: HealthForm;
  /** The first person's share of the pre-tax balance as a percentage; empty to split by pay. */
  traditionalFirst: string;
}

/** Healthcare figures as typed: dollars, whole years, and percentages. */
export interface HealthForm {
  pre65: string;
  post65: string;
  excessInflation: string;
  ltcStartAge: string;
  ltcYears: string;
  ltcAnnual: string;
  ltcChance: string;
}

export const DEFAULT_HEALTH: HealthForm = { pre65: "9,600", post65: "5,500", excessInflation: "1.5", ltcStartAge: "82", ltcYears: "3", ltcAnnual: "110,000", ltcChance: "50" };

export interface ChildForm {
  name: string;
  birthYear: string;
  childcare: string;
  college: string;
}

export function newChild(thisYear: number): ChildForm {
  return { name: "", birthYear: String(thisYear), childcare: "0", college: "0" };
}

export interface HomeForm {
  value: string;
  basis: string;
  balance: string;
  /** Percent, such as 4.5. */
  rate: string;
  yearsLeft: string;
  sellYear: string;
  /** Percent of the price. */
  sellingCost: string;
  replacement: string;
  rentAfter: string;
}

export function newHome(): HomeForm {
  return { value: "0", basis: "0", balance: "0", rate: "4.5", yearsLeft: "25", sellYear: "", sellingCost: "6", replacement: "0", rentAfter: "0" };
}

const bp = (text: string, label: string): number => {
  const n = Number(text.trim());
  if (!Number.isFinite(n) || n < 0 || n > 30) throw new Error(`${label}: enter a percentage from 0 to 30.`);
  return Math.round(n * 100);
};

const pct = (text: string, label: string, max: number): number => {
  const n = Number(text.trim());
  if (!Number.isFinite(n) || n < 0 || n > max) throw new Error(`${label}: enter a percentage from 0 to ${max}.`);
  return Math.round(n * 100);
};

const dollars = (c: number) => formatCents(c, { whole: true }).replace("$", "");

export function toForm(h: Household): Form {
  return {
    people: h.people.map((p) => ({ name: p.name, birthDate: p.birthDate, retireAge: p.retireAge, ss: dollars(p.ssAnnualCents) })),
    streams: h.streams.map((s) => ({
      label: s.label,
      person: s.person,
      amount: dollars(s.annualCents),
      start: s.startYear?.toString() ?? "",
      end: s.endYear?.toString() ?? "",
      contribution: dollars(s.contributionCents),
      selfEmployed: s.selfEmployed ?? false,
      kind: s.kind ?? "",
      match: dollars(s.employerMatchCents ?? 0),
      hsa: dollars(s.hsaCents ?? 0),
    })),
    filing: h.filing,
    state: h.state,
    spending: dollars(h.spendingCents),
    cash: dollars(h.cashCents),
    taxable: dollars(h.taxableCents),
    traditional: dollars(h.traditionalCents),
    roth: dollars(h.rothCents),
    picks: h.picks,
    events: h.events ?? [],
    spendingRule: h.spendingRule ?? "fixed",
    withdrawalOrder: h.withdrawalOrder ?? "standard",
    medicare: h.medicare ?? true,
    health: h.health ?? false,
    giving: dollars(h.givingCents ?? 0),
    home: h.home
      ? {
          value: dollars(h.home.valueCents),
          basis: dollars(h.home.basisCents),
          balance: dollars(h.home.mortgageBalanceCents),
          rate: String(h.home.mortgageRateBp / 100),
          yearsLeft: String(h.home.mortgageYearsLeft),
          sellYear: h.home.sellYear?.toString() ?? "",
          sellingCost: String(h.home.sellingCostBp / 100),
          replacement: dollars(h.home.replacementCostCents),
          rentAfter: dollars(h.home.rentAfterCents),
        }
      : null,
    locality: h.locality ?? "",
    traditionalFirst: h.traditionalFirstBp == null ? "" : String(h.traditionalFirstBp / 100),
    healthDetail: h.healthDetail
      ? {
          pre65: dollars(h.healthDetail.pre65Cents),
          post65: dollars(h.healthDetail.post65Cents),
          excessInflation: String(h.healthDetail.excessInflationBp / 100),
          ltcStartAge: String(h.healthDetail.ltcStartAge),
          ltcYears: String(h.healthDetail.ltcYears),
          ltcAnnual: dollars(h.healthDetail.ltcAnnualCents),
          ltcChance: String(h.healthDetail.ltcChanceBp / 100),
        }
      : { ...DEFAULT_HEALTH },
    children: (h.children ?? []).map((c) => ({
      name: c.name,
      birthYear: String(c.birthYear),
      childcare: dollars(c.childcareCents),
      college: dollars(c.collegeCents),
    })),
  };
}

function money(text: string, label: string): number {
  const c = parseDollars(text);
  if (c === null) throw new Error(`${label}: enter a dollar amount.`);
  return c;
}

function year(text: string, label: string): number | null {
  const t = text.trim();
  if (t === "") return null;
  if (!/^\d{4}$/.test(t)) throw new Error(`${label}: enter a four-digit year, or leave it empty.`);
  return Number(t);
}

/** Convert the edited form back to what the engine stores; throws a readable error naming the field. */
export function fromForm(f: Form): Household {
  const people: PersonRow[] = f.people.map((p, i) => ({
    name: p.name.trim() || `Person ${i + 1}`,
    birthDate: p.birthDate.trim(),
    retireAge: Number(p.retireAge),
    ssAnnualCents: money(p.ss, `${p.name || `Person ${i + 1}`} Social Security`),
  }));
  const streams: StreamRow[] = f.streams.map((s) => {
    const label = s.label.trim() || "Income";
    return {
      label,
      person: s.person,
      annualCents: money(s.amount, `${label} amount`),
      startYear: year(s.start, `${label} first year`),
      endYear: year(s.end, `${label} last year`),
      contributionCents: money(s.contribution, `${label} 401(k) saving`),
      ...(s.selfEmployed ? { selfEmployed: true } : {}),
      ...(s.kind ? { kind: s.kind } : {}),
      ...(money(s.match || "0", `${label} employer match`) > 0 ? { employerMatchCents: money(s.match, `${label} employer match`) } : {}),
      ...(money(s.hsa || "0", `${label} HSA saving`) > 0 ? { hsaCents: money(s.hsa, `${label} HSA saving`) } : {}),
    };
  });
  return {
    people,
    streams,
    filing: f.filing,
    state: f.state.trim().toUpperCase(),
    spendingCents: money(f.spending, "Spending per year"),
    cashCents: money(f.cash, "Cash"),
    taxableCents: money(f.taxable, "Taxable investments"),
    traditionalCents: money(f.traditional, "Traditional 401(k)/IRA"),
    rothCents: money(f.roth, "Roth"),
    picks: f.picks,
    events: f.events,
    spendingRule: f.spendingRule,
    withdrawalOrder: f.withdrawalOrder,
    medicare: f.medicare,
    ...(f.health ? { health: true } : {}),
    ...(money(f.giving || "0", "Charitable giving") > 0 ? { givingCents: money(f.giving, "Charitable giving") } : {}),
    ...(f.home
      ? {
          home: {
            valueCents: money(f.home.value, "Home value"),
            basisCents: money(f.home.basis, "Home purchase price"),
            mortgageBalanceCents: money(f.home.balance, "Mortgage balance"),
            mortgageRateBp: bp(f.home.rate, "Mortgage rate"),
            mortgageYearsLeft: Math.max(0, Math.round(Number(f.home.yearsLeft) || 0)),
            sellYear: year(f.home.sellYear, "Year to sell"),
            sellingCostBp: bp(f.home.sellingCost, "Selling costs"),
            replacementCostCents: money(f.home.replacement, "Cost of the next home"),
            rentAfterCents: money(f.home.rentAfter, "Rent after selling"),
          },
        }
      : {}),
    ...(f.health && JSON.stringify(f.healthDetail) !== JSON.stringify(DEFAULT_HEALTH)
      ? {
          healthDetail: {
            pre65Cents: money(f.healthDetail.pre65, "Healthcare before 65"),
            post65Cents: money(f.healthDetail.post65, "Healthcare from 65"),
            excessInflationBp: pct(f.healthDetail.excessInflation, "Healthcare cost growth", 10),
            ltcStartAge: Math.max(0, Math.round(Number(f.healthDetail.ltcStartAge) || 0)),
            ltcYears: Math.max(0, Math.round(Number(f.healthDetail.ltcYears) || 0)),
            ltcAnnualCents: money(f.healthDetail.ltcAnnual, "Long-term care cost"),
            ltcChanceBp: pct(f.healthDetail.ltcChance, "Chance of needing care", 100),
          },
        }
      : {}),
    ...(f.locality ? { locality: f.locality } : {}),
    ...(f.traditionalFirst.trim() !== "" && f.people.length >= 2 ? { traditionalFirstBp: pct(f.traditionalFirst, "Pre-tax share", 100) } : {}),
    ...(f.children.length > 0
      ? {
          children: f.children.map((c, i) => {
            const name = c.name.trim() || `Child ${i + 1}`;
            const by = year(c.birthYear, `${name} birth year`);
            if (by === null) throw new Error(`${name} birth year: enter a four-digit year.`);
            return {
              name,
              birthYear: by,
              childcareCents: money(c.childcare, `${name} childcare`),
              collegeCents: money(c.college, `${name} college`),
            };
          }),
        }
      : {}),
  };
}

export function newPerson(): Form["people"][number] {
  return { name: "", birthDate: "", retireAge: 65, ss: "0" };
}

export function newStream(oneTime: boolean, thisYear: number): Form["streams"][number] {
  return oneTime
    ? { label: "Bonus", person: 0, amount: "0", start: String(thisYear + 1), end: String(thisYear + 1), contribution: "0", selfEmployed: false, kind: "", match: "0", hsa: "0" }
    : { label: "Income", person: 0, amount: "0", start: "", end: "", contribution: "0", selfEmployed: false, kind: "", match: "0", hsa: "0" };
}

/** Removing a person drops their income and re-points everyone after them. */
export function removePerson(f: Form, index: number): Form {
  return {
    ...f,
    people: f.people.filter((_, i) => i !== index),
    streams: f.streams
      .filter((s) => s.person !== index)
      .map((s) => ({ ...s, person: s.person > index ? s.person - 1 : s.person })),
  };
}

/** What an empty first and last year mean for an income of this kind (matches the planner's defaults). */
export function yearHints(kind: string): { start: string; end: string } {
  if (kind === "pension" || kind === "public_pension") return { start: "retirement", end: "for life" };
  if (kind === "military_pension" || kind === "other") return { start: "now", end: "for life" };
  return { start: "now", end: "retirement" };
}
