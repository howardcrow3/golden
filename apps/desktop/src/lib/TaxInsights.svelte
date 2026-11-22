<script lang="ts">
  import type { Household, QuickPlanResult } from "./api";
  import { formatCents, formatPercent } from "./money";
  import { diversification, rateColour, taxSummary } from "./taxInsights";

  let { result, household }: { result: QuickPlanResult; household?: Household | null } = $props();
  let s = $derived(taxSummary(result.years));
  let d = $derived(household ? diversification(household) : null);
  let has = $derived(result.years.some((y) => y.marginalRate !== undefined));
</script>

{#if has}
  <section class="tax" aria-label="Tax insights">
    <h3>Tax insights</h3>
    <div class="strip" role="img" aria-label="Federal rate on the next dollar, year by year">
      {#each result.years as y}
        <div class="cell" style:background={rateColour(y.marginalRate ?? 0)} title={`${y.year} (age ${y.age}): ${formatPercent(y.marginalRate ?? 0, 0)} on the next dollar`}></div>
      {/each}
    </div>
    <p class="hint">Darker means a higher rate on the next dollar of income that year. {result.years[0].year} to {result.years[result.years.length - 1].year}.</p>
    <ul>
      {#if s.effectiveRate !== null}
        <li data-testid="effective">Across the plan you pay {formatPercent(s.effectiveRate, 1)} of your income in income tax ({formatCents(s.lifetimeIncomeTaxCents, { whole: true })} in total).</li>
      {/if}
      {#if s.peakYear}<li>Your highest rate on the next dollar is {formatPercent(s.peakYear.rate, 0)}, first in {s.peakYear.year}.</li>{/if}
      {#each s.bands as b}<li>{b.years} {b.years === 1 ? "year" : "years"} at {formatPercent(b.rate, 0)}</li>{/each}
    </ul>
    {#if d && d.total > 0}
      <p data-testid="diversification">
        Your savings today: {formatPercent(d.preTax, 0)} pre-tax, {formatPercent(d.roth, 0)} Roth, {formatPercent(d.taxable, 0)} taxable, {formatPercent(d.cash, 0)} cash.
        {#if d.preTax > 0.75}Mostly pre-tax money means large taxable withdrawals later; Roth conversions can spread that out.{/if}
      </p>
    {/if}
  </section>
{/if}

<style>
  .tax { margin-top: 16px; }
  h3 { margin: 0 0 6px; font-size: 15px; }
  .strip { display: flex; height: 22px; border: 1px solid var(--line); }
  .cell { flex: 1; }
  .hint { color: var(--muted); font-size: 12px; margin: 4px 0 8px; }
  ul { margin: 0; padding-left: 18px; font-size: 13px; }
</style>
