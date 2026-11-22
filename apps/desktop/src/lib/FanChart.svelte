<script lang="ts">
  import type { QuickPlanResult } from "./api";
  import { bandPath, linePoints } from "./chart";
  import { formatCents } from "./money";

  let { result }: { result: QuickPlanResult } = $props();
  const W = 640;
  const H = 200;
  let banded = $derived(result.years.length > 0 && result.years.every((y) => y.p10Cents !== undefined));
  let p10 = $derived(result.years.map((y) => y.p10Cents ?? 0));
  let p50 = $derived(result.years.map((y) => y.p50Cents ?? 0));
  let p90 = $derived(result.years.map((y) => y.p90Cents ?? 0));
  let bins = $derived(result.histogram ?? []);
  let tallest = $derived(Math.max(1, ...bins.map((b) => b.count)));
</script>

{#if banded}
  <figure>
    <svg viewBox={`0 0 ${W} ${H}`} role="img" aria-label="Range of possible balances by year">
      <path d={bandPath(p10, p90, W, H)} fill="var(--gold)" opacity="0.25" data-testid="band" />
      <polyline fill="none" stroke="var(--gold)" stroke-width="2" points={linePoints(p50, W, H)} />
    </svg>
    <figcaption>
      Middle line is the median future; the shaded area covers the 10th to 90th percentile
      ({result.years[0].year} to {result.years[result.years.length - 1].year}).
      {#if result.runs}{result.runs.toLocaleString()} simulated futures, seed {result.seed}; the same inputs always give the same result.{/if}
    </figcaption>
  </figure>
{/if}
{#if bins.length}
  <figure>
    <div class="hist" role="img" aria-label="How many simulated futures end at each balance">
      {#each bins as b}
        <div class="col" title={`${formatCents(b.fromCents, { whole: true })} to ${formatCents(b.toCents, { whole: true })}: ${b.count} runs`}>
          <div class="bar" style:height={`${(b.count / tallest) * 100}%`}></div>
        </div>
      {/each}
    </div>
    <figcaption>Final balances, from {formatCents(bins[0].fromCents, { whole: true })} to {formatCents(bins[bins.length - 1].toCents, { whole: true })}. A tall first bar means many futures run out of money.</figcaption>
  </figure>
{/if}

<style>
  figure { margin: 12px 0; }
  svg { width: 100%; height: 200px; border: 1px solid var(--line); background: var(--ivory); }
  figcaption { color: var(--muted); font-size: 12px; margin-top: 4px; }
  .hist { display: flex; align-items: flex-end; gap: 3px; height: 80px; border-bottom: 1px solid var(--line); }
  .col { flex: 1; height: 100%; display: flex; align-items: flex-end; }
  .bar { width: 100%; background: var(--gold); min-height: 1px; }
</style>
