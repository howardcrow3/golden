<script lang="ts">
  import type { QuickPlanResult } from "./api";
  import { columnsChart, lineChart } from "./tbchart";
  import { formatCents } from "./money";

  let { result }: { result: QuickPlanResult } = $props();
  let banded = $derived(result.years.length > 0 && result.years.every((y) => y.p10Cents !== undefined));
  const dollars = (c: number | undefined) => (c ?? 0) / 100;
  let fan = $derived({
    cats: result.years.map((y) => String(y.year)),
    heads: result.years.map((y) => `${y.year}, age ${y.age}`),
    series: [{ name: "Median", color: "var(--s1)", values: result.years.map((y) => dollars(y.p50Cents)) }],
    bands: [{ lo: result.years.map((y) => dollars(y.p10Cents)), hi: result.years.map((y) => dollars(y.p90Cents)), opacity: 0.15 }],
    height: 220,
    zero: true,
    label: "Range of possible balances by year",
  });
  let bins = $derived(result.histogram ?? []);
  let hist = $derived({
    cats: bins.map((b) => formatCents(b.fromCents, { whole: true })),
    heads: bins.map((b) => `${formatCents(b.fromCents, { whole: true })} to ${formatCents(b.toCents, { whole: true })}`),
    series: [{ name: "Futures", color: "var(--s1)", values: bins.map((b) => b.count) }],
    height: 160,
    label: "How many simulated futures end at each balance",
  });
</script>

{#if banded}
  <figure>
    <div use:lineChart={fan} data-testid="fan-chart"></div>
    <figcaption>
      Middle line is the median future; the shaded area covers the 10th to 90th percentile
      ({result.years[0].year} to {result.years[result.years.length - 1].year}).
      {#if result.runs}{result.runs.toLocaleString()} simulated futures, seed {result.seed}; the same inputs always give the same result.{/if}
    </figcaption>
  </figure>
{/if}
{#if bins.length}
  <figure>
    <div use:columnsChart={hist} data-testid="hist-chart"></div>
    <figcaption>Final balances, from {formatCents(bins[0].fromCents, { whole: true })} to {formatCents(bins[bins.length - 1].toCents, { whole: true })}. A tall first bar means many futures run out of money.</figcaption>
  </figure>
{/if}

<style>
  figure { margin: 12px 0; }
  figcaption { color: var(--muted); font-size: 12px; margin-top: 4px; }
</style>
