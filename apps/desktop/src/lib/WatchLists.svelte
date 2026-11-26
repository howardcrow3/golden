<script lang="ts">
  import { localToday } from "./localDate";
  import { getWatchLists, listTransactions, saveWatchLists, type Invoke, type TxnRow, type WatchList } from "./api";
  import { formatCents } from "./money";
  import { watchTotals } from "./watchTotals";

  let { invoke, today = localToday() }: { invoke?: Invoke; today?: Date } = $props();
  let lists = $state<WatchList[]>([]);
  let rows = $state<TxnRow[]>([]);
  let available = $state(false);
  let name = $state("");
  let query = $state("");
  let error = $state("");

  const iso = (d: Date) => d.toISOString().slice(0, 10);

  async function load() {
    try {
      lists = await getWatchLists(invoke);
      const from = new Date(Date.UTC(today.getUTCFullYear(), today.getUTCMonth() - 1, 1));
      rows = await listTransactions(iso(from), iso(today), false, invoke);
      available = true;
    } catch {
      available = false;
    }
  }
  $effect(() => {
    load();
  });

  async function persist(next: WatchList[]) {
    error = "";
    try {
      lists = await saveWatchLists(next, invoke);
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }
  async function add() {
    await persist([...lists, { name, query }]);
    if (!error) {
      name = "";
      query = "";
    }
  }
</script>

{#if available}
  <section class="tb-section" data-testid="watch-lists">
    <h2>Watch lists</h2>
    {#if lists.length === 0}
      <p class="muted">Track what a kind of spending adds up to, such as coffee or a trip. Match a payee, category or tag.</p>
    {/if}
    {#each lists as l (l.name)}
      {@const t = watchTotals(rows, l.query, today)}
      <div class="row" data-testid="watch-row">
        <span><b>{l.name}</b> <span class="muted">· {l.query}</span></span>
        <span class="num">{formatCents(t.thisMonthCents, { whole: true })} this month <span class="muted">({formatCents(t.lastMonthCents, { whole: true })} last month)</span></span>
        <button type="button" aria-label={`Remove ${l.name}`} onclick={() => persist(lists.filter((x) => x.name !== l.name))}>Remove</button>
      </div>
    {/each}
    <form
      class="add"
      onsubmit={(e) => {
        e.preventDefault();
        void add();
      }}
    >
      <input bind:value={name} placeholder="Name, e.g. Coffee" aria-label="Watch list name" />
      <input bind:value={query} placeholder="Words to match, e.g. cafe" aria-label="Watch list search" />
      <button type="submit">Add</button>
    </form>
    {#if error}<p class="err" role="alert">{error}</p>{/if}
  </section>
{/if}

<style>
  section { margin-top: 16px; }
  .row { display: flex; justify-content: space-between; gap: 12px; align-items: center; padding: 6px 0; border-bottom: 1px solid var(--line); }
  .muted { color: var(--muted); }
  .add { display: flex; gap: 8px; margin-top: 8px; flex-wrap: wrap; }
  .err { color: var(--bad); }
</style>
