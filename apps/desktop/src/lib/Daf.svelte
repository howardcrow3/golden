<script lang="ts">
  import { dafBunching, type Bunching, type Invoke } from "./api";
  import { formatCents } from "./money";

  let { invoke }: { invoke?: Invoke } = $props();
  let filing = $state("married_joint");
  let income = $state(150000);
  let other = $state(10000);
  let giving = $state(8000);
  let years = $state(4);
  let out = $state<Bunching | null>(null);
  let error = $state("");

  async function run() {
    error = "";
    out = null;
    try {
      out = await dafBunching(
        filing,
        Math.round(Number(income) * 100),
        Math.round(Number(other) * 100),
        Math.round(Number(giving) * 100),
        Number(years),
        invoke,
      );
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }
</script>

<div class="daf" data-testid="daf">
  <h3>Bunching gifts in a donor-advised fund</h3>
  <p class="hint">
    Give several years of charity at once to a donor-advised fund, itemize that year, and take the standard deduction in the
    years between. The fund then sends the money to charities over time.
  </p>
  <div class="row">
    <label>Filing as
      <select bind:value={filing} aria-label="Filing as">
        <option value="single">Single</option>
        <option value="married_joint">Married filing jointly</option>
        <option value="head_of_household">Head of household</option>
      </select>
    </label>
    <label>Income before deductions <input type="number" min="0" bind:value={income} aria-label="Income before deductions" /></label>
    <label>Other itemized deductions <input type="number" min="0" bind:value={other} aria-label="Other itemized deductions" /></label>
    <label>Giving each year <input type="number" min="0" bind:value={giving} aria-label="Giving each year" /></label>
    <label>Years to bunch <input type="number" min="2" max="10" bind:value={years} aria-label="Years to bunch" /></label>
  </div>
  <button type="button" onclick={run}>Compare</button>
  {#if error}<p class="err" role="alert">{error}</p>{/if}
  {#if out}
    <p data-testid="daf-result">
      {#if out.savedCents > 0}
        Bunching saves about <strong>{formatCents(out.savedCents, { whole: true })}</strong> in federal tax over {out.years} years
        ({formatCents(out.taxEachYearCents, { whole: true })} giving evenly, {formatCents(out.taxBunchedCents, { whole: true })} bunched).
      {:else if out.itemizesAnyway}
        You already itemize every year, so bunching saves nothing here.
      {:else}
        Bunching would not lower your tax here.
      {/if}
    </p>
    <p class="hint">Cash gifts are deductible up to 60% of income. Federal tax only, 2026 rules.</p>
  {/if}
</div>

<style>
  .daf { margin-top: 16px; border-top: 1px solid var(--line, #ddd); padding-top: 12px; }
  h3 { margin: 0 0 4px; font-size: 15px; }
  .hint { color: var(--muted); font-size: 13px; margin: 0 0 8px; }
  .row { display: flex; gap: 12px; flex-wrap: wrap; }
  label { display: flex; flex-direction: column; gap: 4px; font-size: 12px; color: var(--muted); margin-bottom: 8px; max-width: 180px; }
  .err { color: var(--danger, #b3261e); }
</style>
