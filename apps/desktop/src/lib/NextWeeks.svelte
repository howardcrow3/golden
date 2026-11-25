<script lang="ts">
  import { getRecurring, type Invoke } from "./api";
  import { formatCents } from "./money";
  import { biggest, upcoming, type Upcoming } from "./upcoming";

  let { invoke, today = new Date() }: { invoke?: Invoke; today?: Date } = $props();
  let items = $state<Upcoming[]>([]);

  const iso = (d: Date) => d.toISOString().slice(0, 10);

  $effect(() => {
    const from = new Date(today);
    from.setFullYear(from.getFullYear() - 1);
    getRecurring(iso(from), iso(today), invoke)
      .then((r) => (items = biggest(upcoming(r, iso(today), 13), 12)))
      .catch(() => (items = []));
  });

  let inflow = $derived(items.filter((i) => i.cents > 0).reduce((s, i) => s + i.cents, 0));
  let outflow = $derived(items.filter((i) => i.cents < 0).reduce((s, i) => s - i.cents, 0));
</script>

{#if items.length}
  <section class="card" data-testid="next-weeks">
    <h2>Next 13 weeks</h2>
    <p class="muted" data-testid="next-weeks-total">
      The biggest dated items coming in: <b class="num">{formatCents(inflow, { whole: true })}</b> in, <b class="num">{formatCents(outflow, { whole: true })}</b> out.
      Everyday spending is not listed.
    </p>
    <table>
      <tbody>
        {#each items as i}
          <tr data-testid="next-weeks-row">
            <td>{i.date}</td>
            <td>{i.payee}</td>
            <td class="r num" class:neg={i.cents < 0}>{formatCents(i.cents)}</td>
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
  td { padding: 6px 0; border-bottom: 1px solid var(--line); }
  .r { text-align: right; }
  .neg { color: var(--bad); }
</style>
