<script lang="ts">
  import { suggestAllocation, type Allocation, type Invoke } from "./api";
  import { formatCents, parseDollars } from "./money";

  let { invoke }: { invoke?: Invoke } = $props();
  let amount = $state("");
  let result = $state<Allocation | null>(null);
  let error = $state("");

  async function suggest() {
    error = "";
    result = null;
    const cents = parseDollars(amount);
    if (cents === null || cents < 0) {
      error = "Enter how much you plan to save in a year, like 20000.";
      return;
    }
    try {
      result = await suggestAllocation(cents, invoke);
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }
</script>

<section class="card" data-testid="allocate">
  <h2>Where should new savings go?</h2>
  <p class="muted">A starting order, not advice: emergency cash, then your workplace plan, then an IRA, then a taxable account. It doesn't know about employer matches or income limits.</p>
  <label>Saved per year
    <input bind:value={amount} inputmode="decimal" aria-label="Yearly savings" placeholder="20,000" />
  </label>
  <button onclick={suggest}>Suggest</button>
  {#if error}<p class="err" role="alert">{error}</p>{/if}
  {#if result}
    {#if result.steps.length}
      <ol data-testid="allocation-steps">
        {#each result.steps as s}
          <li><b>{s.name}</b> <span class="num">{formatCents(s.cents, { whole: true })}</span><br /><span class="muted">{s.why}</span></li>
        {/each}
      </ol>
    {:else}
      <p class="muted">Nothing to place yet.</p>
    {/if}
  {/if}
</section>

<style>
  section { margin-top: 16px; }
  label { display: inline-flex; flex-direction: column; gap: 4px; font-size: 12px; color: var(--muted); margin-right: 12px; }
  .muted { color: var(--muted); }
  .err { color: var(--bad); }
  li { margin: 8px 0; }
</style>
