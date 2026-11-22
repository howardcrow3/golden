import type { YearPoint } from "./api";

export interface FlowNode {
  key: string;
  label: string;
  cents: number;
}

export interface YearFlow {
  year: number;
  age: number;
  sources: FlowNode[];
  sinks: FlowNode[];
  total: number;
}

/**
 * One year of money in and money out. Everything coming in is pooled, so each source is split
 * across the uses in proportion; that keeps the diagram honest without per-dollar tracking.
 * What is not spent or paid in tax is shown as saved (401(k) saving and money added to the
 * portfolio).
 */
export function yearFlow(y: YearPoint): YearFlow {
  const pos = (n: number | undefined) => Math.max(0, Math.round(n ?? 0));
  const sources: FlowNode[] = [
    { key: "earned", label: "Wages and business income", cents: pos(y.earnedCents) },
    { key: "ss", label: "Social Security", cents: pos(y.socialSecurityCents) },
    { key: "rmd", label: "Required withdrawals", cents: pos(y.rmdCents) },
    { key: "withdrawals", label: "Withdrawals from savings", cents: pos(y.withdrawalsCents) },
  ].filter((n) => n.cents > 0);
  const inflow = sources.reduce((s, n) => s + n.cents, 0);
  const taxes = pos(y.taxesCents);
  const medicare = Math.min(pos(y.medicareCents), pos(y.spendingCents));
  const living = Math.max(0, pos(y.spendingCents) - medicare);
  const saved = Math.max(0, inflow - taxes - living - medicare);
  const sinks: FlowNode[] = [
    { key: "living", label: "Living costs", cents: living },
    { key: "medicare", label: "Medicare premiums", cents: medicare },
    { key: "taxes", label: "Taxes", cents: taxes },
    { key: "saved", label: "Saved", cents: saved },
  ].filter((n) => n.cents > 0);
  // If a year spends more than it brings in (rounding, a shortfall), scale to the larger side.
  const outflow = sinks.reduce((s, n) => s + n.cents, 0);
  return { year: y.year, age: y.age, sources, sinks, total: Math.max(inflow, outflow) };
}

export interface Box {
  key: string;
  label: string;
  cents: number;
  x: number;
  y: number;
  w: number;
  h: number;
}
export interface Ribbon {
  from: string;
  to: string;
  cents: number;
  path: string;
}

/** Position the boxes and ribbons inside a width x height area. */
export function layoutFlow(f: YearFlow, width: number, height: number, gap = 8, bar = 14) {
  const stack = (nodes: FlowNode[], x: number): Box[] => {
    const usable = height - gap * Math.max(0, nodes.length - 1);
    const sum = nodes.reduce((s, n) => s + n.cents, 0) || 1;
    let y = 0;
    return nodes.map((n) => {
      const h = Math.max(2, (n.cents / sum) * usable);
      const b = { key: n.key, label: n.label, cents: n.cents, x, y, w: bar, h };
      y += h + gap;
      return b;
    });
  };
  const left = stack(f.sources, 0);
  const right = stack(f.sinks, width - bar);
  const outflow = f.sinks.reduce((s, n) => s + n.cents, 0) || 1;
  const leftUsed = new Map(left.map((b) => [b.key, 0]));
  const rightUsed = new Map(right.map((b) => [b.key, 0]));
  const ribbons: Ribbon[] = [];
  const inflow = f.sources.reduce((s, n) => s + n.cents, 0);
  for (const s of left) {
    for (const k of right) {
      // Share of this source that goes to this use.
      const cents = inflow > 0 ? (s.cents * k.cents) / outflow : 0;
      if (cents <= 0) continue;
      const th = (cents / s.cents) * s.h;
      const kh = (cents / k.cents) * k.h;
      const y0 = s.y + (leftUsed.get(s.key) ?? 0);
      const y1 = k.y + (rightUsed.get(k.key) ?? 0);
      leftUsed.set(s.key, (leftUsed.get(s.key) ?? 0) + th);
      rightUsed.set(k.key, (rightUsed.get(k.key) ?? 0) + kh);
      const x0 = s.x + s.w;
      const x1 = k.x;
      const mx = (x0 + x1) / 2;
      const f2 = (n: number) => n.toFixed(1);
      const path =
        `M${f2(x0)},${f2(y0)} C${f2(mx)},${f2(y0)} ${f2(mx)},${f2(y1)} ${f2(x1)},${f2(y1)} ` +
        `L${f2(x1)},${f2(y1 + kh)} C${f2(mx)},${f2(y1 + kh)} ${f2(mx)},${f2(y0 + th)} ${f2(x0)},${f2(y0 + th)} Z`;
      ribbons.push({ from: s.key, to: k.key, cents: Math.round(cents), path });
    }
  }
  return { left, right, ribbons };
}
