<script lang="ts">
  import { getProgress, startProgress, stopProgress, type Invoke, type ProgressReport } from "./api";
  import { pairPoints } from "./chart";
  import { formatCents } from "./money";

  let { invoke }: { invoke?: Invoke } = $props();
  let report = $state<ProgressReport | null>(null);
  let loaded = $state(false);
  let error = $state("");
  const W = 560;
  const H = 120;

  async function load() {
    try {
      report = await getProgress(invoke);
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loaded = true;
    }
  }
  async function act(fn: () => Promise<void>) {
    error = "";
    try {
      await fn();
      await load();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }
  $effect(() => {
    load();
  });
  const lines = $derived(report ? pairPoints(report.points.map((p) => p.actualChangeCents), report.points.map((p) => p.projectedChangeCents), W, H) : ["", ""]);
</script>

{#if loaded}
  <section class="progress" aria-label="Progress against the plan" data-testid="progress">
    <h3>Progress against the plan</h3>
    {#if !report}
      <p class="hint">Start tracking to see, month by month, how your real net worth moves against what the plan projected from today. It uses the statements you import.</p>
      <button type="button" onclick={() => act(() => startProgress(invoke))}>Start tracking from today</button>
    {:else}
      <p data-testid="progress-summary">
        Since {report.started}, your net worth has {report.aheadCents >= 0 ? "beaten" : "trailed"} the plan's projected savings growth by
        <strong>{formatCents(Math.abs(report.aheadCents), { whole: true })}</strong>.
      </p>
      {#if report.points.length > 1}
        <svg viewBox={`0 0 ${W} ${H}`} role="img" aria-label="Change in net worth against the projected change">
          <polyline points={lines[1]} fill="none" stroke="var(--muted)" stroke-width="2" stroke-dasharray="4 3" />
          <polyline points={lines[0]} fill="none" stroke="var(--gold)" stroke-width="2" data-testid="actual-line" />
        </svg>
        <p class="hint">Gold is your actual net worth change, dashed is the plan. Net worth also counts a home and debts, so this compares change, not level.</p>
      {:else}
        <p class="hint">Come back next month: one more month of statements gives a line to compare.</p>
      {/if}
      <button type="button" onclick={() => act(() => stopProgress(invoke))}>Stop tracking</button>
      <button type="button" onclick={() => act(() => startProgress(invoke))}>Restart from today</button>
    {/if}
    {#if error}<p class="err" role="alert">{error}</p>{/if}
  </section>
{/if}

<style>
  .progress { margin-top: 16px; }
  h3 { margin: 0 0 6px; font-size: 15px; }
  svg { width: 100%; max-width: 640px; height: auto; display: block; margin: 6px 0; }
  .hint { color: var(--muted); font-size: 12px; margin: 4px 0; }
  .err { color: var(--bad); }
</style>
