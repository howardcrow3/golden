<script lang="ts">
  import { budgetStatus, categoryChoices, saveReport, setBudget, setBudgetRollover, TOTAL_BUDGET, type BudgetRow, type Invoke } from "./api";
  import { type Grain } from "./grain";
  import { formatCents, parseDollars } from "./money";
  import { budgetCsv } from "./report";

  let { grain, today = new Date(), invoke }: { grain: Grain; today?: Date; invoke?: Invoke } = $props();
  let rows = $state<BudgetRow[]>([]);
  let choices = $state<string[]>([]);
  let available = $state(false);
  let category = $state(TOTAL_BUDGET);
  let amount = $state("");
  let error = $state("");
  let version = $state(0);
  let savedTo = $state("");

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

  async function toggleRoll(r: BudgetRow, on: boolean) {
    error = "";
    try {
      await setBudgetRollover(r.category === "Total" ? TOTAL_BUDGET : r.category, on, invoke);
      version += 1;
    } catch (e) {
      error = String(e);
    }
  }

  async function exportCsv() {
    error = "";
    try {
      const day = iso(today);
      savedTo = await saveReport(`Golden budget ${grain} ${day}`, "csv", budgetCsv(rows, `${grain} of ${day}`), invoke);
    } catch (e) {
      error = String(e);
    }
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
  <section class="tb-section" data-testid="budgets">
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
          {#if (r.rolloverCents ?? 0) !== 0}
            · includes {formatCents(r.rolloverCents ?? 0, { whole: true })} rolled over
          {/if}
          {#if grain === "month"}
            <label class="roll"><input type="checkbox" checked={!!r.rolls} onchange={(e) => toggleRoll(r, e.currentTarget.checked)} aria-label={`Roll over ${r.category}`} /> Roll over</label>
          {/if}
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
    {#if rows.length > 0}
      <p><button type="button" onclick={exportCsv}>Export budget as CSV</button></p>
    {/if}
    {#if savedTo}<p class="muted small" data-testid="budget-saved">Saved to {savedTo}</p>{/if}
    {#if error}<p role="alert" class="err">{error}</p>{/if}
  </section>
{/if}

<style>
  section { margin-top: 16px; }
  .row { padding: 8px 0; border-bottom: 1px solid var(--line); }
  .roll { margin-left: 12px; }
  .head { display: flex; justify-content: space-between; }
  .bar { height: 8px; background: var(--line); border-radius: 4px; margin: 6px 0 2px; overflow: hidden; }
  .fill { height: 100%; background: var(--accent); }
  .fill.over { background: var(--bad); }
  .over { color: var(--bad); }
  .muted { color: var(--muted); }
  .small { font-size: 12px; }
  .set { display: flex; gap: 12px; align-items: end; margin-top: 14px; flex-wrap: wrap; }
  .set label { display: flex; flex-direction: column; gap: 4px; font-size: 12px; color: var(--muted); }
  .err { color: var(--bad); }
</style>
