<script lang="ts">
  import { budgetStatus, getRecurring, type Invoke } from "./api";
  import { formatCents } from "./money";
  import { monthEnd, safeToSpend, type SafeToSpend } from "./spendToday";
  import { upcoming } from "./upcoming";

  let { invoke, today = new Date() }: { invoke?: Invoke; today?: Date } = $props();
  const iso = (d: Date) => d.toISOString().slice(0, 10);

  let result = $state<SafeToSpend | null>(null);
  let noBudget = $state(false);

  $effect(() => {
    const day = iso(today);
    const from = new Date(today);
    from.setFullYear(from.getFullYear() - 1);
    Promise.all([budgetStatus(day, "month", invoke), getRecurring(iso(from), day, invoke)])
      .then(([rows, recurring]) => {
        const total = rows.find((r) => r.category === "Total");
        if (!total) {
          noBudget = true;
          result = null;
          return;
        }
        noBudget = false;
        const end = monthEnd(day);
        const bills = upcoming(recurring, day, 6)
          .filter((i) => i.cents < 0 && i.date <= end)
          .reduce((s, i) => s - i.cents, 0);
        result = safeToSpend(total.remainingCents, bills, day);
      })
      .catch(() => {
        result = null;
        noBudget = false;
      });
  });
</script>

{#if result}
  <section class="tb-section" data-testid="safe-to-spend">
    <h2>Safe to spend today</h2>
    {#if result.over}
      <p data-testid="safe-to-spend-over">
        Bills still due this month are <b class="num">{formatCents(-result.leftCents, { whole: true })}</b> more than what is left of your budget.
      </p>
    {:else}
      <p class="big num" data-testid="safe-to-spend-today">{formatCents(result.perDayCents, { whole: true })}</p>
      <p class="muted">
        a day for the next {result.daysLeft} {result.daysLeft === 1 ? "day" : "days"}: <span class="num">{formatCents(result.leftCents, { whole: true })}</span>
        left of this month's budget after <span class="num">{formatCents(result.billsCents, { whole: true })}</span> of bills still to come.
      </p>
    {/if}
  </section>
{:else if noBudget}
  <section class="tb-section" data-testid="safe-to-spend-hint">
    <h2>Safe to spend today</h2>
    <p class="muted">Set an overall monthly budget on the Budgets screen and Golden will show what you can spend each day.</p>
  </section>
{/if}

<style>
  section { margin-top: 16px; }
  .big { font-size: 28px; margin: 4px 0; }
  .muted { color: var(--muted); margin: 0 0 8px; }
</style>
