<script lang="ts">
  import { localToday } from "./localDate";
  import { getRecurring, type Invoke } from "./api";
  import { formatCents } from "./money";
  import { addMonths, calendarMonth, monthTitle } from "./monthGrid";
  import { upcoming, type Upcoming } from "./upcoming";

  let { invoke, today = localToday() }: { invoke?: Invoke; today?: Date } = $props();
  const iso = (d: Date) => d.toISOString().slice(0, 10);

  let items = $state<Upcoming[]>([]);
  let offset = $state(0);
  let selected = $state("");

  $effect(() => {
    const day = iso(today);
    const from = new Date(today);
    from.setFullYear(from.getFullYear() - 1);
    getRecurring(iso(from), day, invoke)
      .then((r) => (items = upcoming(r, day, 26)))
      .catch(() => (items = []));
  });

  let shown = $derived(addMonths(today.getUTCFullYear(), today.getUTCMonth() + 1, offset));
  let weeks = $derived(calendarMonth(items, shown.year, shown.month));
  let picked = $derived(weeks.flat().find((d) => d.date === selected));
  const todayIso = $derived(iso(today));
</script>

{#if items.length}
  <section class="tb-section" data-testid="bills-calendar">
    <h2>Bills calendar</h2>
    <div class="nav">
      <button aria-label="Previous month" onclick={() => { offset -= 1; selected = ""; }} disabled={offset <= 0}>‹</button>
      <b data-testid="calendar-title">{monthTitle(shown.year, shown.month)}</b>
      <button aria-label="Next month" onclick={() => { offset += 1; selected = ""; }} disabled={offset >= 5}>›</button>
    </div>
    <table>
      <thead>
        <tr>{#each ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"] as d}<th>{d}</th>{/each}</tr>
      </thead>
      <tbody>
        {#each weeks as week}
          <tr>
            {#each week as d}
              <td class:today={d.date === todayIso} class:has={d.items.length > 0}>
                {#if d.day}
                  <button class="cell" data-testid={d.items.length ? "calendar-busy" : undefined} onclick={() => (selected = d.date)} aria-label={`${d.date}${d.items.length ? `, ${d.items.length} items` : ""}`}>
                    <span class="n">{d.day}</span>
                    {#if d.items.length}<span class="net num" class:neg={d.netCents < 0}>{formatCents(d.netCents, { whole: true })}</span>{/if}
                  </button>
                {/if}
              </td>
            {/each}
          </tr>
        {/each}
      </tbody>
    </table>
    {#if picked && picked.items.length}
      <ul data-testid="calendar-detail">
        {#each picked.items as i}
          <li>{i.payee}: <span class="num" class:neg={i.cents < 0}>{formatCents(i.cents)}</span></li>
        {/each}
      </ul>
    {/if}
  </section>
{/if}

<style>
  section { margin-top: 16px; }
  .nav { display: flex; gap: 12px; align-items: center; margin-bottom: 8px; }
  table { width: 100%; border-collapse: collapse; table-layout: fixed; }
  th { font-size: 12px; color: var(--muted); text-align: left; padding: 4px; }
  td { height: 52px; vertical-align: top; border: 1px solid var(--line); padding: 0; }
  td.today { outline: 2px solid var(--accent, currentColor); outline-offset: -2px; }
  td.has { background: var(--surface-2, transparent); }
  .cell { all: unset; display: flex; flex-direction: column; gap: 2px; width: 100%; height: 100%; box-sizing: border-box; padding: 4px; cursor: pointer; }
  .n { font-size: 12px; color: var(--muted); }
  .net { font-size: 12px; }
  .neg { color: var(--bad); }
</style>
