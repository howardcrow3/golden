<script lang="ts">
  import { getRecurring, type Invoke, type Recurring } from "./api";
  import { formatCents } from "./money";

  let { invoke, today = new Date() }: { invoke?: Invoke; today?: Date } = $props();
  let rows = $state<Recurring[]>([]);

  const iso = (d: Date) => d.toISOString().slice(0, 10);

  $effect(() => {
    const from = new Date(today);
    from.setFullYear(from.getFullYear() - 1);
    getRecurring(iso(from), iso(today), invoke)
      .then((r) => (rows = r))
      .catch(() => (rows = []));
  });

  let out = $derived(rows.filter((r) => r.typicalCents < 0));
  let yearlyOut = $derived(out.reduce((s, r) => s + -r.yearlyCents, 0));
</script>

{#if rows.length}
  <section class="tb-section" data-testid="recurring">
    <h2>What repeats</h2>
    <p class="muted" data-testid="recurring-total">
      Regular payments add up to <b class="num">{formatCents(yearlyOut, { whole: true })}</b> a year, from the last 12 months of statements.
    </p>
    <table>
      <thead><tr><th>Who</th><th>How often</th><th class="r">Amount</th><th class="r">Next</th></tr></thead>
      <tbody>
        {#each rows as r}
          <tr data-testid="recurring-row">
            <td>{r.payee}</td>
            <td>{r.cadence}</td>
            <td class="r num" class:neg={r.typicalCents < 0}>{formatCents(r.typicalCents)}</td>
            <td class="r">{r.nextExpected}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  </section>
{/if}

<style>
  section { margin-top: 16px; }
  .muted { color: var(--muted); margin: 0 0 8px; }
  table { width: 100%; border-collapse: collapse; }
  th, td { padding: 6px 0; border-bottom: 1px solid var(--line); text-align: left; font-weight: normal; }
  th { color: var(--muted); font-size: 12px; }
  .r { text-align: right; }
  .neg { color: var(--bad); }
</style>
