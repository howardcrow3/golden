<script lang="ts">
  import { GRAINS, GRAIN_LABEL, type Grain } from "./grain";
  import ThisPeriod from "./ThisPeriod.svelte";
  import GettingStarted from "./GettingStarted.svelte";
  import CashFlow from "./CashFlow.svelte";
  import Budgets from "./Budgets.svelte";
  import Forecast from "./Forecast.svelte";
  import BalanceSheet from "./BalanceSheet.svelte";
  import RecurringList from "./RecurringList.svelte";
  import Debts from "./Debts.svelte";
  import Insights from "./Insights.svelte";
  import Goals from "./Goals.svelte";

  let { onnavigate }: { onnavigate?: (page: string) => void } = $props();
  let grain = $state<Grain>("month");
</script>

<h1>Overview</h1>
<p class="sub">Import a statement to see your own numbers here.</p>

<div class="toggle" role="group" aria-label="Time grain">
  {#each GRAINS as g}
    <button aria-pressed={grain === g} onclick={() => (grain = g)}>{GRAIN_LABEL[g]}</button>
  {/each}
</div>

<GettingStarted {onnavigate} />
<ThisPeriod {grain} />

<Insights />
<BalanceSheet />
<RecurringList />
<Goals />
<Debts />
<Forecast />

<Budgets {grain} />

<CashFlow {grain} />

<style>
  .sub { color: var(--muted); margin: 0 0 20px; }
  .toggle { display: flex; gap: 6px; margin-bottom: 20px; }
</style>
