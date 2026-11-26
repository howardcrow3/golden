<script lang="ts">
  import { retireWhen, type Invoke, type RetireWhen } from "./api";
  import { formatPercent } from "./money";

  let { invoke, refresh = null }: { invoke?: Invoke; refresh?: unknown } = $props();
  let target = $state(0.85);
  let result = $state<RetireWhen | null>(null);
  let error = $state("");
  let busy = $state(false);

  $effect(() => {
    refresh;
    result = null;
  });

  async function solve() {
    busy = true;
    error = "";
    try {
      result = await retireWhen(target, invoke);
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      busy = false;
    }
  }
</script>

<div class="when" data-testid="retire-when">
  <h3>When could you retire?</h3>
  <p class="hint">Tries each retirement age for the first person and finds the earliest one where the plan works in the share of trials you pick.</p>
  <label>
    Comfort level
    <select bind:value={target} onchange={() => (result = null)} aria-label="Retire-when comfort level">
      <option value={0.7}>70% of trials</option>
      <option value={0.8}>80%</option>
      <option value={0.85}>85%</option>
      <option value={0.9}>90%</option>
      <option value={0.95}>95%</option>
    </select>
  </label>
  <button type="button" disabled={busy} onclick={solve}>{busy ? "Working…" : "Find my earliest age"}</button>
  {#if error}<p class="err" role="alert">{error}</p>{/if}
  {#if result}
    {#if result.age === null}
      <p data-testid="when-answer">No retirement age up to 75 reaches {formatPercent(result.target, 0)}. More saving, income or lower spending would change that.</p>
    {:else}
      <p data-testid="when-answer">
        You could retire at <strong>{result.age}</strong> and still have the plan work in {formatPercent(result.target, 0)} of trials.
        {#if result.age < result.currentAge}That is {result.currentAge - result.age} years earlier than the {result.currentAge} you have set.
        {:else if result.age > result.currentAge}That is {result.age - result.currentAge} years later than the {result.currentAge} you have set.
        {:else}That matches the {result.currentAge} you have set.{/if}
      </p>
    {/if}
    <ul class="tries">
      {#each result.tries.slice(-8) as t}
        <li data-testid="when-try">Age {t.age}: {formatPercent(t.successRate, 0)}</li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .when { margin-top: 16px; border-top: 1px solid var(--line); padding-top: 12px; }
  h3 { margin: 0 0 4px; font-size: 15px; }
  .hint { color: var(--muted); font-size: 13px; margin: 0 0 8px; }
  label { display: flex; flex-direction: column; gap: 4px; font-size: 12px; color: var(--muted); margin-bottom: 8px; max-width: 220px; }
  .tries { list-style: none; padding: 0; margin: 6px 0 0; font-size: 12px; color: var(--muted); display: flex; gap: 12px; flex-wrap: wrap; }
  .err { color: var(--danger); }
</style>
