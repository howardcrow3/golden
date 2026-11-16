<script lang="ts">
  import { cashFlow, categories, type CategoryRow, type Invoke, type PeriodRow } from "./api";
  import type { Grain } from "./grain";
  import { formatCents } from "./money";

  let { grain, today = new Date(), invoke }: { grain: Grain; today?: Date; invoke?: Invoke } = $props();
  let rows = $state<PeriodRow[]>([]);
  let cats = $state<CategoryRow[]>([]);
  let ready = $state(false);

  const iso = (d: Date) => d.toISOString().slice(0, 10);

  /** Start of the window: roughly twelve periods back. */
  export function windowStart(t: Date, g: Grain): Date {
    const d = new Date(t);
    const back = { week: 12 * 7, month: 365, quarter: 3 * 365, year: 5 * 365 }[g];
    d.setUTCDate(d.getUTCDate() - back);
    return d;
  }

  $effect(() => {
    const g = grain;
    (async () => {
      try {
        rows = await cashFlow(iso(windowStart(today, g)), iso(today), g, invoke);
        // Categories for the latest period that has activity.
        const last = [...rows].reverse().find((r) => r.count > 0);
        cats = last ? await categories(iso(today), g, false, invoke) : [];
        ready = true;
      } catch {
        rows = [];
        ready = false;
      }
    })();
  });
  let active = $derived(rows.filter((r) => r.count > 0));
</script>

{#if ready && active.length > 0}
  <section class="card" data-testid="cashflow">
    <h2>Cash flow by {grain}</h2>
    <table>
      <thead><tr><th>Period</th><th class="r">Income</th><th class="r">Spending</th><th class="r">Net</th></tr></thead>
      <tbody>
        {#each rows as r}
          <tr><td>{r.period}</td><td class="r num">{formatCents(r.incomeCents, { whole: true })}</td><td class="r num">{formatCents(r.spendingCents, { whole: true })}</td><td class="r num" class:neg={r.netCents < 0}>{formatCents(r.netCents, { whole: true })}</td></tr>
        {/each}
      </tbody>
    </table>
  </section>
  {#if cats.length > 0}
    <section class="card" data-testid="categories">
      <h2>Spending by category this {grain}</h2>
      {#each cats as c}<div class="line"><span>{c.category}</span><b class="num">{formatCents(c.cents, { whole: true })}</b></div>{/each}
    </section>
  {/if}
{/if}

<style>
  section { margin-top: 16px; }
  table { width: 100%; border-collapse: collapse; }
  th, td { padding: 6px 0; border-bottom: 1px solid var(--line); text-align: left; font-weight: normal; }
  th { color: var(--muted); font-size: 12px; }
  .r { text-align: right; }
  .neg { color: var(--bad); }
  .line { display: flex; justify-content: space-between; padding: 6px 0; border-bottom: 1px solid var(--line); }
</style>
