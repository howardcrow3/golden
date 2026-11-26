export type SplitDraft = { category: string; amount: string };

/** "12.34" -> 1234 cents, or null when it is not a plain amount. */
export function dollarsToCents(text: string): number | null {
  const t = text.trim().replace(/^\$/, "");
  if (!/^\d+(\.\d{1,2})?$/.test(t)) return null;
  return Math.round(parseFloat(t) * 100);
}

/** Cents of the line still to be given to a part, in the line's own direction (always >= 0 when under-filled). */
export function splitLeftCents(lineCents: number, parts: SplitDraft[]): number {
  const total = Math.abs(lineCents);
  const used = parts.reduce((n, p) => n + (dollarsToCents(p.amount) ?? 0), 0);
  return total - used;
}

/** Parts as signed cents matching the line, or an error to show. */
export function splitToParts(lineCents: number, parts: SplitDraft[]): { ok: true; parts: { category: string; cents: number }[] } | { ok: false; error: string } {
  if (parts.length < 2) return { ok: false, error: "A split needs at least two parts." };
  const sign = lineCents < 0 ? -1 : 1;
  const out: { category: string; cents: number }[] = [];
  for (const p of parts) {
    const c = dollarsToCents(p.amount);
    if (!p.category.trim()) return { ok: false, error: "Give every part a category." };
    if (c === null || c === 0) return { ok: false, error: "Enter an amount like 12.50 for every part." };
    out.push({ category: p.category.trim(), cents: sign * c });
  }
  if (splitLeftCents(lineCents, parts) !== 0) return { ok: false, error: "The parts must add up to the full amount." };
  return { ok: true, parts: out };
}
