<script lang="ts">
  import { GRAINS, GRAIN_LABEL, convertBudget, type Grain } from "./grain";
  import { formatCents } from "./money";
  import CashFlow from "./CashFlow.svelte";
  import Budgets from "./Budgets.svelte";
  import Forecast from "./Forecast.svelte";

  // Sample household until real data is imported.
  const monthlyBudget = 650_000;
  let grain = $state<Grain>("month");
  let budget = $derived(convertBudget(monthlyBudget, "month", grain));
</script>

<h1>Overview</h1>
<p class="sub">Sample numbers until you import your first statement.</p>

<div class="toggle" role="group" aria-label="Time grain">
  {#each GRAINS as g}
    <button aria-pressed={grain === g} onclick={() => (grain = g)}>{GRAIN_LABEL[g]}</button>
  {/each}
</div>

<section class="card">
  <h2>Spending budget per {grain}</h2>
  <div class="big num" data-testid="budget">{formatCents(budget, { whole: true })}</div>
</section>

<Forecast />

<Budgets {grain} />

<CashFlow {grain} />

<style>
  .sub { color: var(--muted); margin: 0 0 20px; }
  .toggle { display: flex; gap: 6px; margin-bottom: 20px; }
  .big { font-size: 36px; font-weight: 600; }
</style>
