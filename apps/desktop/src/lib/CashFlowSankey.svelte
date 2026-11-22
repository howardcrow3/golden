<script lang="ts">
  import type { QuickPlanResult } from "./api";
  import { formatCents } from "./money";
  import { layoutFlow, yearFlow } from "./sankey";

  let { result }: { result: QuickPlanResult } = $props();
  const W = 560;
  const H = 220;
  let has = $derived(result.years.length > 0 && result.years.some((y) => y.earnedCents !== undefined));
  let index = $state(0);
  let i = $derived(Math.min(index, Math.max(0, result.years.length - 1)));
  let flow = $derived(has ? yearFlow(result.years[i]) : null);
  let layout = $derived(flow ? layoutFlow(flow, W, H) : null);
  const colours: Record<string, string> = {
    earned: "var(--good)", ss: "var(--gold)", rmd: "var(--muted)", withdrawals: "var(--muted)",
    living: "var(--charcoal)", medicare: "var(--gold)", taxes: "var(--bad)", saved: "var(--good)",
  };
</script>

{#if has && flow && layout}
  <section class="flow" aria-label="Where the money goes">
    <h3>Where the money goes</h3>
    <label class="year">Year
      <input type="range" min="0" max={result.years.length - 1} bind:value={index} aria-label="Year to show" />
      <strong data-testid="flow-year">{flow.year}</strong> <span class="age">age {flow.age}</span>
    </label>
    <svg viewBox={`-4 -4 ${W + 8} ${H + 8}`} role="img" aria-label={`Money in and out in ${flow.year}`}>
      {#each layout.ribbons as r}
        <path d={r.path} fill={colours[r.from]} opacity="0.28" />
      {/each}
      {#each [...layout.left, ...layout.right] as b}
        <rect x={b.x} y={b.y} width={b.w} height={b.h} fill={colours[b.key]} />
      {/each}
    </svg>
    <div class="cols">
      <ul data-testid="flow-in">
        <li class="head">Money in</li>
        {#each flow.sources as s}<li>{s.label} <strong>{formatCents(s.cents, { whole: true })}</strong></li>{/each}
      </ul>
      <ul data-testid="flow-out">
        <li class="head">Money out</li>
        {#each flow.sinks as s}<li>{s.label} <strong>{formatCents(s.cents, { whole: true })}</strong></li>{/each}
      </ul>
    </div>
    <p class="hint">Each source is shown split across the uses in proportion. Saved includes 401(k) saving and money added to the portfolio.</p>
  </section>
{/if}

<style>
  .flow { margin-top: 16px; }
  h3 { margin: 0 0 6px; font-size: 15px; }
  .year { display: flex; align-items: center; gap: 8px; font-size: 13px; }
  .year input { flex: 1; }
  .age { color: var(--muted); }
  svg { width: 100%; max-width: 640px; height: auto; margin-top: 8px; }
  .cols { display: flex; gap: 24px; flex-wrap: wrap; }
  ul { list-style: none; margin: 4px 0; padding: 0; font-size: 13px; min-width: 220px; }
  li { display: flex; justify-content: space-between; gap: 12px; padding: 1px 0; }
  .head { color: var(--muted); font-size: 12px; text-transform: uppercase; letter-spacing: 0.04em; }
  .hint { color: var(--muted); font-size: 12px; margin: 4px 0 0; }
</style>
