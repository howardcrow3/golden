/** Turn a series into an SVG polyline `points` string inside a width x height box. */
export function linePoints(values: number[], width: number, height: number, pad = 4): string {
  if (values.length === 0) return "";
  const max = Math.max(...values, 1);
  const min = Math.min(...values, 0);
  const span = max - min || 1;
  const x = (i: number) => pad + (values.length === 1 ? 0 : (i * (width - 2 * pad)) / (values.length - 1));
  const y = (v: number) => height - pad - ((v - min) / span) * (height - 2 * pad);
  return values.map((v, i) => `${x(i).toFixed(1)},${y(v).toFixed(1)}`).join(" ");
}

export type Verdict = { label: string; tone: "good" | "ok" | "bad" };

/** Plain-language read of a success rate; always paired with the range, never alone. */
export function verdict(successRate: number): Verdict {
  if (successRate >= 0.85) return { label: "On track", tone: "good" };
  if (successRate >= 0.65) return { label: "Close, worth a look", tone: "ok" };
  return { label: "At risk", tone: "bad" };
}

/** SVG path for the area between two series sharing one scale (a band of a fan chart). */
export function bandPath(lower: number[], upper: number[], width: number, height: number, pad = 4): string {
  const n = Math.min(lower.length, upper.length);
  if (n === 0) return "";
  const all = [...lower.slice(0, n), ...upper.slice(0, n)];
  const max = Math.max(...all, 1);
  const min = Math.min(...all, 0);
  const span = max - min || 1;
  const x = (i: number) => pad + (n === 1 ? 0 : (i * (width - 2 * pad)) / (n - 1));
  const y = (v: number) => height - pad - ((v - min) / span) * (height - 2 * pad);
  const top = upper.slice(0, n).map((v, i) => `${x(i).toFixed(1)},${y(v).toFixed(1)}`);
  const bottom = lower
    .slice(0, n)
    .map((v, i) => `${x(i).toFixed(1)},${y(v).toFixed(1)}`)
    .reverse();
  return `M${[...top, ...bottom].join(" L")} Z`;
}
