import type { Household, PersonRow, StreamRow } from "./api";
import { formatCents, parseDollars } from "./money";

/** Editable text version of a household: money and years are strings while being typed. */
export interface Form {
  people: { name: string; birthDate: string; retireAge: number; ss: string }[];
  streams: { label: string; person: number; amount: string; start: string; end: string; contribution: string }[];
  filing: Household["filing"];
  state: string;
  spending: string;
  cash: string;
  taxable: string;
  traditional: string;
  roth: string;
  picks: Record<string, string>;
}

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
    })),
    filing: h.filing,
    state: h.state,
    spending: dollars(h.spendingCents),
    cash: dollars(h.cashCents),
    taxable: dollars(h.taxableCents),
    traditional: dollars(h.traditionalCents),
    roth: dollars(h.rothCents),
    picks: h.picks,
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
  };
}

export function newPerson(): Form["people"][number] {
  return { name: "", birthDate: "", retireAge: 65, ss: "0" };
}

export function newStream(oneTime: boolean, thisYear: number): Form["streams"][number] {
  return oneTime
    ? { label: "Bonus", person: 0, amount: "0", start: String(thisYear + 1), end: String(thisYear + 1), contribution: "0" }
    : { label: "Income", person: 0, amount: "0", start: "", end: "", contribution: "0" };
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
