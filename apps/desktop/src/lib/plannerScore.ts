import type { Household } from "./api";

export interface ScoreItem {
  label: string;
  done: boolean;
  points: number;
  /** What to do when it is not done. */
  hint: string;
}

export interface PlannerScore {
  /** 0 to 100: how much of what the plan needs has been filled in. */
  score: number;
  items: ScoreItem[];
  /** Things that make the numbers less trustworthy even though everything is filled in. */
  warnings: string[];
}

/** States whose income tax Golden only approximates (see the tax data notes). */
const APPROXIMATE_STATES = new Set(["AR", "CT", "MD", "MN", "NY", "VT"]);

function ageOn(birth: string, today: Date): number | null {
  const m = /^(\d{4})-(\d{2})-(\d{2})$/.exec(birth.trim());
  if (!m) return null;
  const [y, mo, d] = [Number(m[1]), Number(m[2]), Number(m[3])];
  if (mo < 1 || mo > 12 || d < 1 || d > 31) return null;
  let age = today.getFullYear() - y;
  if (today.getMonth() + 1 < mo || (today.getMonth() + 1 === mo && today.getDate() < d)) age -= 1;
  return age;
}

/** How complete the plan is and what could make it misleading. Pure: same household, same answer. */
export function plannerScore(h: Household, today: Date = new Date()): PlannerScore {
  const ages = h.people.map((p) => ageOn(p.birthDate, today));
  const total = h.cashCents + h.taxableCents + h.traditionalCents + h.rothCents;
  const retired = h.people.every((p, i) => ages[i] !== null && ages[i]! >= p.retireAge);
  const items: ScoreItem[] = [
    {
      label: "Everyone has a birth date",
      done: h.people.length > 0 && ages.every((a) => a !== null && a >= 0 && a <= 110),
      points: 15,
      hint: "Birth dates set Social Security, Medicare and withdrawal-penalty ages.",
    },
    {
      label: "Income is entered",
      done: h.streams.some((s) => s.annualCents > 0) || retired,
      points: 10,
      hint: "Add a salary, pension or other income, or set a retirement age that has already passed.",
    },
    {
      label: "Spending is entered",
      done: h.spendingCents > 0,
      points: 15,
      hint: "Enter what the household spends in a year.",
    },
    {
      label: "Account balances are entered",
      done: total > 0,
      points: 15,
      hint: "Add cash, taxable, traditional and Roth balances.",
    },
    {
      label: "Retirement accounts are split into pre-tax and Roth",
      done: h.traditionalCents > 0 || h.rothCents > 0,
      points: 5,
      hint: "Traditional and Roth money is taxed differently when it comes out.",
    },
    {
      label: "Social Security is entered for each person",
      done: h.people.length > 0 && h.people.every((p) => p.ssAnnualCents > 0),
      points: 10,
      hint: "Use the estimate from your Social Security statement.",
    },
    {
      label: "A state is chosen",
      done: /^[A-Za-z]{2}$/.test(h.state),
      points: 5,
      hint: "State income tax depends on where you live.",
    },
    {
      label: "Assumptions have been reviewed",
      done: Object.keys(h.picks).length > 0,
      points: 10,
      hint: "Look at the return, inflation and lifespan options and pick the ones you believe.",
    },
    {
      label: "Cash covers three months of spending",
      done: h.spendingCents > 0 && h.cashCents >= h.spendingCents / 4,
      points: 5,
      hint: "Keeping a few months in cash avoids selling investments in a downturn.",
    },
    {
      label: "Planned big costs are entered",
      done: (h.events ?? []).length > 0,
      points: 10,
      hint: "A roof, tuition or a wedding changes the plan; add them in the what-if chat.",
    },
  ];
  const score = items.reduce((s, i) => s + (i.done ? i.points : 0), 0);

  const warnings: string[] = [];
  h.people.forEach((p, i) => {
    const a = ages[i];
    if (a !== null && a >= 50 && p.ssAnnualCents === 0) {
      warnings.push(`${p.name || `Person ${i + 1}`} has no Social Security entered, so the plan assumes none.`);
    }
    if (a !== null && p.retireAge < a && a < 59 && h.traditionalCents > h.cashCents + h.taxableCents + h.rothCents) {
      warnings.push(
        `${p.name || `Person ${i + 1}`} is retired before 59½ with most money in pre-tax accounts; withdrawals carry a 10% penalty.`,
      );
    }
  });
  if (total > 0 && h.spendingCents / total > 0.1) {
    warnings.push("Spending is more than 10% of everything saved each year, which few plans sustain.");
  }
  if (APPROXIMATE_STATES.has(h.state.toUpperCase())) {
    warnings.push(`State tax for ${h.state.toUpperCase()} is an estimate.`);
  }
  if (h.people.some((p, i) => ages[i] !== null && p.retireAge < 40)) {
    warnings.push("A retirement age under 40 looks like a typo.");
  }
  return { score, items, warnings };
}
