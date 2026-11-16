<script lang="ts">
  import { categoryChoices, exportTransactions, listTransactions, setCategory, type Invoke, type TxnRow } from "./api";
  import { formatCents } from "./money";

  let { invoke, today = new Date() }: { invoke?: Invoke; today?: Date } = $props();
  let rows = $state<TxnRow[]>([]);
  let choices = $state<string[]>([]);
  let onlyTodo = $state(false);
  let remember = $state(true);
  let error = $state("");
  let note = $state("");
  let loaded = $state(false);

  const iso = (d: Date) => d.toISOString().slice(0, 10);
  function lastYear(): string {
    const from = new Date(today);
    from.setUTCFullYear(from.getUTCFullYear() - 1);
    return iso(from);
  }
  // Defaults to the last twelve months; both ends can be changed.
  // svelte-ignore state_referenced_locally
  let fromDay = $state(lastYear());
  // svelte-ignore state_referenced_locally
  let toDay = $state(iso(today));
  const DAY = /^\d{4}-\d{2}-\d{2}$/;
  function range(): [string, string] {
    if (!DAY.test(fromDay) || !DAY.test(toDay)) throw new Error("Pick a start and end date.");
    if (fromDay > toDay) throw new Error("The start date must be on or before the end date.");
    return [fromDay, toDay];
  }

  async function load() {
    try {
      const [from, to] = range();
      rows = await listTransactions(from, to, onlyTodo, invoke);
      choices = await categoryChoices(invoke);
      loaded = true;
      error = "";
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }
  $effect(() => {
    onlyTodo; // reload when the filter or dates change
    fromDay;
    toDay;
    load();
  });

  async function exportCsv() {
    note = "";
    try {
      const [from, to] = range();
      const path = await exportTransactions(from, to, invoke);
      note = `Exported to ${path}`;
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }

  async function change(row: TxnRow, value: string) {
    const v = value.trim();
    if (!v || v === row.category) return;
    note = "";
    try {
      const r = await setCategory(row.id, v, remember, invoke);
      note = r.alsoUpdated > 0 ? `Filed under ${r.category}, and ${r.alsoUpdated} more from the same merchant.` : `Filed under ${r.category}.`;
      await load();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }
</script>

<h1>Transactions</h1>
<p class="sub">Newest first; the last twelve months unless you change the dates. Pick a category to fix a line.</p>

<div class="bar">
  <label class="check">From <input type="date" bind:value={fromDay} aria-label="From date" /></label>
  <label class="check">To <input type="date" bind:value={toDay} aria-label="To date" /></label>
  <label class="check"><input type="checkbox" bind:checked={onlyTodo} /> Only lines that need a category</label>
  <label class="check"><input type="checkbox" bind:checked={remember} /> Remember this merchant for next time</label>
  <button type="button" onclick={exportCsv}>Export CSV</button>
</div>

{#if error}<p class="err" role="alert">{error}</p>{/if}
{#if note}<p role="status">{note}</p>{/if}

<datalist id="categories">{#each choices as c}<option value={c}></option>{/each}</datalist>

{#if loaded && rows.length === 0}
  <p class="empty">{onlyTodo ? "Everything has a category." : "No transactions yet. Import a statement first."}</p>
{:else if rows.length > 0}
  <table class="card">
    <thead><tr><th>Date</th><th>Payee</th><th>Account</th><th class="r">Amount</th><th>Category</th></tr></thead>
    <tbody>
      {#each rows as r (r.id)}
        <tr>
          <td class="num">{r.day}</td>
          <td>{r.payee}</td>
          <td class="muted">{r.account}</td>
          <td class="r num" class:neg={r.amountCents < 0}>{formatCents(r.amountCents)}</td>
          <td>
            <input list="categories" value={r.category ?? ""} placeholder="Choose…" aria-label={`Category for ${r.payee} on ${r.day}`} onchange={(e) => change(r, e.currentTarget.value)} />
          </td>
        </tr>
      {/each}
    </tbody>
  </table>
{/if}

<style>
  .sub { color: var(--muted); margin: 0 0 16px; }
  .bar { display: flex; gap: 24px; margin-bottom: 12px; flex-wrap: wrap; }
  .check { display: flex; gap: 6px; align-items: center; font-size: 13px; }
  .check input { padding: 0; }
  table { width: 100%; border-collapse: collapse; padding: 0; }
  th, td { padding: 8px 10px; border-bottom: 1px solid var(--line); text-align: left; font-weight: normal; }
  th { color: var(--muted); font-size: 12px; }
  .r { text-align: right; }
  .neg { color: var(--bad); }
  .muted { color: var(--muted); }
  .empty { color: var(--muted); }
  .err { color: var(--bad); }
  td input { width: 100%; min-width: 150px; }
</style>
