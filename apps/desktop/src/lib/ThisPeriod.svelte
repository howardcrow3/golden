<script lang="ts">
  import { cashFlow, type Invoke, type PeriodRow } from "./api";
  import type { Grain } from "./grain";
  import { formatCents } from "./money";

  let { grain, today = new Date(), invoke }: { grain: Grain; today?: Date; invoke?: Invoke } = $props();
  let latest = $state<PeriodRow | null>(null);

  const iso = (d: Date) => d.toISOString().slice(0, 10);

  $effect(() => {
    const g = grain;
    (async () => {
      try {
        const from = new Date(today);
        from.setUTCDate(from.getUTCDate() - { week: 14, month: 62, quarter: 190, year: 750 }[g]);
        const rows = await cashFlow(iso(from), iso(today), g, invoke);
        latest = [...rows].reverse().find((r) => r.count > 0) ?? null;
      } catch {
        latest = null;
      }
    })();
  });

  /** Share of income kept, as a whole percent; null when there was no income. */
  export function savingsRate(r: PeriodRow): number | null {
    return r.incomeCents > 0 ? Math.round((r.netCents / r.incomeCents) * 100) : null;
  }
  let rate = $derived(latest ? savingsRate(latest) : null);
</script>

{#if latest}
  <section class="card" data-testid="this-period">
    <h2>{latest.period}</h2>
    <div class="row">
      <div><span class="label">Income</span><b class="num" data-testid="tp-income">{formatCents(latest.incomeCents, { whole: true })}</b></div>
      <div><span class="label">Spending</span><b class="num" data-testid="tp-spending">{formatCents(latest.spendingCents, { whole: true })}</b></div>
      <div><span class="label">Kept</span><b class="num" class:neg={latest.netCents < 0} data-testid="tp-net">{formatCents(latest.netCents, { whole: true })}</b></div>
    </div>
    {#if rate !== null}
      <p class="rate" data-testid="tp-rate">{rate >= 0 ? `You kept ${rate}% of what came in.` : `You spent ${-rate}% more than came in.`}</p>
    {/if}
  </section>
{/if}

<style>
  .row { display: flex; gap: 32px; flex-wrap: wrap; }
  .label { display: block; color: var(--muted); font-size: 12px; }
  b { font-size: 24px; font-weight: 600; }
  .neg { color: var(--bad); }
  .rate { color: var(--muted); margin: 12px 0 0; }
</style>
