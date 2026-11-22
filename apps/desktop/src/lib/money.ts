let hidden = false;
/** Screen-sharing mode: every dollar amount the app formats shows as dots. */
export function setHideAmounts(on: boolean): void {
  hidden = on;
}
export function amountsHidden(): boolean {
  return hidden;
}

/** Format integer cents as US dollars: 123456 -> "$1,234.56". */
export function formatCents(cents: number, opts: { whole?: boolean } = {}): string {
  if (hidden) return "$•••••";
  const neg = cents < 0;
  const abs = Math.abs(Math.round(cents));
  const whole = opts.whole ? Math.round(abs / 100) : Math.floor(abs / 100);
  const dollars = whole.toString().replace(/\B(?=(\d{3})+(?!\d))/g, ",");
  const body = opts.whole ? dollars : `${dollars}.${(abs % 100).toString().padStart(2, "0")}`;
  return `${neg ? "-" : ""}$${body}`;
}

/** Parse a user-typed dollar amount into cents; returns null when it is not a number. */
export function parseDollars(input: string): number | null {
  const t = input.trim().replace(/[$,\s]/g, "");
  if (!/^-?\d*\.?\d{0,2}$/.test(t) || t === "" || t === "-" || t === ".") return null;
  return Math.round(parseFloat(t) * 100);
}

export function formatPercent(rate: number, digits = 1): string {
  return `${(rate * 100).toFixed(digits)}%`;
}
