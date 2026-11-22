<script lang="ts">
  import { rothExplore, type Invoke, type RothRow } from "./api";
  import { formatCents, formatPercent } from "./money";

  let { invoke, refresh = null }: { invoke?: Invoke; refresh?: unknown } = $props();
  let untilAge = $state(72);
  let rows = $state<RothRow[] | null>(null);
  let error = $state("");
  let busy = $state(false);

  $effect(() => {
    refresh;
    rows = null;
  });

  async function run() {
    busy = true;
    error = "";
    try {
      rows = await rothExplore(Number(untilAge), invoke);
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      busy = false;
    }
  }

  // The option with the most left for heirs after tax, among those that still work as well as doing nothing.
  const best = $derived.by(() => {
    if (!rows || rows.length < 2) return null;
    const base = rows[0];
    const ok = rows.filter((r) => r.successRate >= base.successRate - 0.02);
    return ok.reduce((a, b) => (b.legacyCents > a.legacyCents ? b : a), ok[0]);
  });
</script>

<div class="roth" data-testid="roth-explorer">
  <h3>Roth conversions</h3>
  <p class="hint">
    Move money from traditional to Roth accounts each year, from your first year without a paycheck, and compare the result.
    The tax on a conversion is paid from your accounts in the order you chose. Estimates, not tax advice.
  </p>
  <label>
    Convert until age
    <input type="number" min="50" max="90" bind:value={untilAge} aria-label="Convert until age" />
  </label>
  <button type="button" disabled={busy} onclick={run}>{busy ? "Working…" : "Compare strategies"}</button>
  {#if error}<p class="err" role="alert">{error}</p>{/if}
  {#if rows}
    <table>
      <thead>
        <tr><th>Strategy</th><th>Converted</th><th>Tax over the plan</th><th>Left for heirs, after tax</th><th>Chance it works</th></tr>
      </thead>
      <tbody>
        {#each rows as r}
          <tr class:best={best === r} data-testid="roth-row">
            <th scope="row">{r.label}</th>
            <td class="num">{formatCents(r.convertedCents, { whole: true })}</td>
            <td class="num">{formatCents(r.lifetimeTaxCents, { whole: true })}</td>
            <td class="num">{formatCents(r.legacyCents, { whole: true })}</td>
            <td class="num">{formatPercent(r.successRate, 0)}</td>
          </tr>
        {/each}
      </tbody>
    </table>
    {#if best && best !== rows[0]}
      <p data-testid="roth-best">{best.label} leaves the most for heirs after tax without making the plan much less likely to work.</p>
    {:else}
      <p data-testid="roth-best">None of these beat doing nothing on what is left for heirs.</p>
    {/if}
    <p class="hint">"Left for heirs" counts traditional money after a 22% tax, so the rows are comparable.</p>
  {/if}
</div>

<style>
  .roth { margin-top: 16px; border-top: 1px solid var(--line, #ddd); padding-top: 12px; }
  h3 { margin: 0 0 4px; font-size: 15px; }
  .hint { color: var(--muted); font-size: 13px; margin: 0 0 8px; }
  label { display: flex; flex-direction: column; gap: 4px; font-size: 12px; color: var(--muted); margin-bottom: 8px; max-width: 160px; }
  table { border-collapse: collapse; width: 100%; margin-top: 10px; font-size: 14px; }
  th, td { text-align: right; padding: 6px 8px; border-bottom: 1px solid var(--line, #ddd); }
  th:first-child, tbody th { text-align: left; font-weight: 400; }
  tr.best { background: var(--ivory, #faf6ea); }
  .err { color: var(--bad, #b00020); }
</style>
