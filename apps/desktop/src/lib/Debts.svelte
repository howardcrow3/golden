<script lang="ts">
  import { getDebtPlan, setDebtTerms, type DebtPlan, type Invoke, type Payoff } from "./api";
  import { formatCents, parseDollars } from "./money";

  let { invoke }: { invoke?: Invoke } = $props();
  let plan = $state<DebtPlan | null>(null);
  let extra = $state("0");
  let rates = $state<Record<string, string>>({});
  let mins = $state<Record<string, string>>({});
  let error = $state("");

  async function load() {
    const cents = parseDollars(extra);
    if (cents === null || cents < 0) {
      error = "Enter the extra monthly payment as a number, like 200.";
      return;
    }
    error = "";
    try {
      const p = await getDebtPlan(cents, invoke);
      plan = p;
      for (const d of p.debts) {
        rates[d.name] ??= d.hasTerms ? (d.aprBp / 100).toFixed(2) : "";
        mins[d.name] ??= d.hasTerms ? (d.minCents / 100).toFixed(2) : "";
      }
    } catch {
      plan = null;
    }
  }
  $effect(() => {
    load();
  });

  async function save(name: string) {
    error = "";
    const rate = parseFloat(rates[name] ?? "");
    const min = parseDollars(mins[name] ?? "");
    if (!Number.isFinite(rate) || rate < 0 || min === null || min < 0) {
      error = `Enter ${name}'s yearly interest rate (like 22.99) and monthly minimum (like 90).`;
      return;
    }
    try {
      await setDebtTerms(name, Math.round(rate * 100), min, invoke);
      await load();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }

  const years = (p: Payoff) => (p.feasible ? `${p.months} months` : "never at these payments");
</script>

{#if plan && plan.debts.length}
  <section class="card" data-testid="debts">
    <h2>Paying off what you owe</h2>
    {#each plan.debts as d}
      <div class="row" data-testid="debt-terms">
        <span>{d.name} <span class="num">{formatCents(d.balanceCents, { whole: true })}</span></span>
        <label>Rate % <input aria-label={`${d.name} interest rate`} bind:value={rates[d.name]} inputmode="decimal" /></label>
        <label>Minimum $ <input aria-label={`${d.name} minimum payment`} bind:value={mins[d.name]} inputmode="decimal" /></label>
        <button onclick={() => save(d.name)}>Save</button>
      </div>
    {/each}
    <label>Extra per month $ <input aria-label="Extra monthly payment" bind:value={extra} onchange={load} inputmode="decimal" /></label>
    {#if error}<p class="err" role="alert">{error}</p>{/if}
    {#if plan.avalanche && plan.snowball && plan.minimumsOnly}
      <div data-testid="payoff">
        <p>Highest rate first: debt-free in <b>{years(plan.avalanche)}</b>, interest <span class="num">{formatCents(plan.avalanche.totalInterestCents, { whole: true })}</span>.</p>
        <p>Smallest balance first: <b>{years(plan.snowball)}</b>, interest <span class="num">{formatCents(plan.snowball.totalInterestCents, { whole: true })}</span>.</p>
        {#if plan.extraCents > 0 && plan.minimumsOnly.feasible && plan.avalanche.feasible}
          <p class="muted" data-testid="saved">Minimums only would take {plan.minimumsOnly.months} months; the extra saves <span class="num">{formatCents(plan.minimumsOnly.totalInterestCents - plan.avalanche.totalInterestCents, { whole: true })}</span> in interest.</p>
        {/if}
      </div>
    {:else}
      <p class="muted">Add each debt's rate and minimum payment to see how fast you can clear them.</p>
    {/if}
  </section>
{/if}

<style>
  section { margin-top: 16px; }
  .row { display: flex; gap: 12px; align-items: end; flex-wrap: wrap; margin-bottom: 8px; }
  label { display: inline-flex; flex-direction: column; gap: 4px; font-size: 12px; color: var(--muted); }
  input { width: 90px; }
  .muted { color: var(--muted); }
  .err { color: var(--bad); }
</style>
