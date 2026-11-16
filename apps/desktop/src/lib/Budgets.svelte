<script lang="ts">
  import { budgetStatus, categoryChoices, setBudget, TOTAL_BUDGET, type BudgetRow, type Invoke } from "./api";
  import { type Grain } from "./grain";
  import { formatCents, parseDollars } from "./money";

  let { grain, today = new Date(), invoke }: { grain: Grain; today?: Date; invoke?: Invoke } = $props();
  let rows = $state<BudgetRow[]>([]);
  let choices = $state<string[]>([]);
  let available = $state(false);
  let category = $state(TOTAL_BUDGET);
  let amount = $state("");
  let error = $state("");
  let version = $state(0);

  const iso = (d: Date) => d.toISOString().slice(0, 10);

  $effect(() => {
    const g = grain;
    void version;
    (async () => {
      try {
        rows = await budgetStatus(iso(today), g, invoke);
        choices = await categoryChoices(invoke);
        available = true;
      } catch {
        rows = [];
        available = false;
      }
    })();
  });

  /** Share of the budget used, capped for the bar. */
  export function usedPercent(r: BudgetRow): number {
    if (r.budgetCents <= 0) return 0;
    return Math.min(100, Math.round((r.spentCents / r.budgetCents) * 100));
  }

  async function save() {
    error = "";
    const dollars = parseDollars(amount);
    if (dollars === null || dollars < 0) {
      error = "Enter the monthly budget as a dollar amount, or 0 to remove it.";
      return;
    }
    // The box is entered per month; show it per month regardless of the grain toggle.
    try {
      await setBudget(category, dollars, invoke);
      amount = "";
      version += 1;
    } catch (e) {
      error = String(e);
    }
  }
</script>

{#if available}
  <section class="card" data-testid="budgets">
    <h2>Budgets this {grain}</h2>
    {#if rows.length === 0}
      <p class="muted">No budgets yet. Set one below, per month.</p>
    {/if}
    {#each rows as r}
      <div class="row" data-testid="budget-row">
        <div class="head">
          <span>{r.category}</span>
          <span class="num" class:over={r.remainingCents < 0}>
            {formatCents(r.spentCents, { whole: true })} of {formatCents(r.budgetCents, { whole: true })}
          </span>
        </div>
        <div class="bar" role="progressbar" aria-label={`${r.category} used`} aria-valuenow={usedPercent(r)} aria-valuemin="0" aria-valuemax="100">
          <div class="fill" class:over={r.remainingCents < 0} style:width={`${usedPercent(r)}%`}></div>
        </div>
        <div class="muted small">
          {r.remainingCents < 0
            ? `${formatCents(-r.remainingCents, { whole: true })} over`
            : `${formatCents(r.remainingCents, { whole: true })} left`}
        </div>
      </div>
    {/each}
    <form
      class="set"
      onsubmit={(e) => {
        e.preventDefault();
        void save();
      }}
    >
      <label>
        Category
        <select bind:value={category}>
          <option value={TOTAL_BUDGET}>Everything (overall)</option>
          {#each choices as c}<option value={c}>{c}</option>{/each}
        </select>
      </label>
      <label>
        Per month
        <input bind:value={amount} inputmode="decimal" placeholder="e.g. 400" />
      </label>
      <button type="submit">Set budget</button>
    </form>
    {#if error}<p role="alert" class="err">{error}</p>{/if}
  </section>
{/if}

<style>
  section { margin-top: 16px; }
  .row { padding: 8px 0; border-bottom: 1px solid var(--line); }
  .head { display: flex; justify-content: space-between; }
  .bar { height: 8px; background: var(--line); border-radius: 4px; margin: 6px 0 2px; overflow: hidden; }
  .fill { height: 100%; background: var(--accent, #c9a227); }
  .fill.over { background: var(--bad); }
  .over { color: var(--bad); }
  .muted { color: var(--muted); }
  .small { font-size: 12px; }
  .set { display: flex; gap: 12px; align-items: end; margin-top: 14px; flex-wrap: wrap; }
  .set label { display: flex; flex-direction: column; gap: 4px; font-size: 12px; color: var(--muted); }
  .err { color: var(--bad); }
</style>
