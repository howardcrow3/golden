<script lang="ts">
  import { safeSpend, type Invoke, type SafeSpend } from "./api";
  import { formatCents, formatPercent } from "./money";

  let { invoke, refresh = null }: { invoke?: Invoke; refresh?: unknown } = $props();
  let target = $state(0.85);
  let result = $state<SafeSpend | null>(null);
  let error = $state("");
  let busy = $state(false);

  // A new plan run makes any earlier answer stale.
  $effect(() => {
    refresh;
    result = null;
  });

  async function solve() {
    busy = true;
    error = "";
    try {
      result = await safeSpend(target, invoke);
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      busy = false;
    }
  }
  const diff = $derived(result ? result.spendingCents - result.currentCents : 0);
</script>

<div class="safe" data-testid="safe-spend">
  <h3>How much could you spend?</h3>
  <p class="hint">The most a year (in today's dollars) that still works in the share of trials you pick.</p>
  <label>
    Comfort level
    <select bind:value={target} onchange={() => (result = null)} aria-label="Comfort level">
      <option value={0.7}>70% of trials (more spending, more risk)</option>
      <option value={0.8}>80%</option>
      <option value={0.85}>85%</option>
      <option value={0.9}>90%</option>
      <option value={0.95}>95% (less spending, less risk)</option>
    </select>
  </label>
  <button type="button" disabled={busy} onclick={solve}>{busy ? "Working…" : "Find my safe spending"}</button>
  {#if error}<p class="err" role="alert">{error}</p>{/if}
  {#if result}
    {#if result.nothingSafe}
      <p data-testid="safe-answer">At {formatPercent(result.target, 0)} the plan does not work even with very little spending. Adding savings or income is the first lever.</p>
    {:else}
      <p data-testid="safe-answer" class="num">
        About <b>{formatCents(result.spendingCents, { whole: true })}</b> a year{result.capped ? " or more" : ""} works in {formatPercent(result.target, 0)} of trials.
        {#if diff >= 0}You plan {formatCents(result.currentCents, { whole: true })}, so there is {formatCents(diff, { whole: true })} of room.
        {:else}You plan {formatCents(result.currentCents, { whole: true })}, which is {formatCents(-diff, { whole: true })} more than that.{/if}
      </p>
    {/if}
  {/if}
</div>

<style>
  .safe { margin-top: 16px; border-top: 1px solid var(--line); padding-top: 12px; }
  h3 { margin: 0 0 4px; font-size: 15px; }
  .hint { color: var(--muted); font-size: 13px; margin: 0 0 8px; }
  label { display: flex; flex-direction: column; gap: 4px; font-size: 12px; color: var(--muted); margin-bottom: 8px; }
  .err { color: var(--bad); }
</style>
