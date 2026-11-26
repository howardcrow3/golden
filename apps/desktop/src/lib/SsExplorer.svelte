<script lang="ts">
  import { ssExplore, type Invoke, type SsExplore } from "./api";
  import { formatCents, formatPercent } from "./money";

  let { invoke, refresh = null }: { invoke?: Invoke; refresh?: unknown } = $props();
  let data = $state<SsExplore | null>(null);
  let error = $state("");
  let busy = $state(false);

  $effect(() => {
    refresh;
    data = null;
  });

  async function run() {
    busy = true;
    error = "";
    try {
      data = await ssExplore(invoke);
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      busy = false;
    }
  }

  const best = $derived.by(() => {
    if (!data || !data.rows.length) return null;
    const top = Math.max(...data.rows.map((r) => r.legacyCents));
    // The earliest age that gets within 1% of the best legacy, so a tie does not push people to wait.
    return data.rows.find((r) => r.legacyCents >= top * 0.99) ?? null;
  });
  const when = (a: number | null) => (a === null ? "never" : `about ${a.toFixed(1)}`);
</script>

<div class="ss" data-testid="ss-explorer">
  <h3>When to claim Social Security</h3>
  <p class="hint">
    Enter each person's benefit at full retirement age (from your Social Security statement). Claiming earlier pays less each year, later pays more, up to 70.
    Spousal and survivor benefits are not included.
  </p>
  <button type="button" disabled={busy} onclick={run}>{busy ? "Working…" : "Compare claiming ages"}</button>
  {#if error}<p class="err" role="alert">{error}</p>{/if}
  {#if data}
    <table>
      <thead><tr><th>Claim at</th><th>Per year</th><th>Total received</th><th>Left for heirs, after tax</th><th>Chance it works</th></tr></thead>
      <tbody>
        {#each data.rows as r}
          <tr class:best={best === r} data-testid="ss-row">
            <th scope="row">{r.claimAge}</th>
            <td class="num">{r.annualCents.map((c) => formatCents(c, { whole: true })).join(" + ")}</td>
            <td class="num">{formatCents(r.lifetimeCents, { whole: true })}</td>
            <td class="num">{formatCents(r.legacyCents, { whole: true })}</td>
            <td class="num">{formatPercent(r.successRate, 0)}</td>
          </tr>
        {/each}
      </tbody>
    </table>
    {#if best}<p data-testid="ss-best">Claiming at {best.claimAge} leaves the most for heirs after tax, on the average path.</p>{/if}
    <ul class="be">
      {#each data.breakEvens as b}
        <li data-testid="ss-breakeven">Claiming at {b.later} instead of {b.earlier} pays off in total {when(b.age)}{b.age === null ? "" : " years old"}.</li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .ss { margin-top: 16px; border-top: 1px solid var(--line); padding-top: 12px; }
  h3 { margin: 0 0 4px; font-size: 15px; }
  .hint { color: var(--muted); font-size: 13px; margin: 0 0 8px; }
  table { border-collapse: collapse; width: 100%; margin-top: 10px; font-size: 14px; }
  th, td { text-align: right; padding: 6px 8px; border-bottom: 1px solid var(--line); }
  th:first-child, tbody th { text-align: left; font-weight: 400; }
  tr.best { background: var(--accent-soft); }
  .be { margin: 8px 0 0; padding-left: 18px; font-size: 13px; color: var(--muted); }
  .err { color: var(--bad); }
</style>
