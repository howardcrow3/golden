<script lang="ts">
  import { categoryChoices, exportBackup, exportTransactions, listTransactions, restoreBackup, setCategory, setNoteTags, setSplits, type Invoke, type TxnRow } from "./api";
  import CategorySuggest from "./CategorySuggest.svelte";
  import { formatCents } from "./money";
  import { splitLeftCents, splitToParts, type SplitDraft } from "./splitMath";
  import { filterTxns, groupByCategory, sumTxns } from "./txnFilter";

  let { invoke, today = new Date() }: { invoke?: Invoke; today?: Date } = $props();
  let rows = $state<TxnRow[]>([]);
  let choices = $state<string[]>([]);
  let onlyTodo = $state(false);
  let remember = $state(true);
  let error = $state("");
  let note = $state("");
  let loaded = $state(false);
  let query = $state("");
  let shown = $derived(filterTxns(rows, query));
  let totals = $derived(sumTxns(shown));
  let view = $state<"list" | "board">("list");
  let lanes = $derived(groupByCategory(shown));

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

  async function backup() {
    note = "";
    try {
      note = `Backup saved to ${await exportBackup(invoke)}`;
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }

  function readBytes(f: File): Promise<Uint8Array> {
    return new Promise((resolve, reject) => {
      const r = new FileReader();
      r.onload = () => resolve(new Uint8Array(r.result as ArrayBuffer));
      r.onerror = () => reject(new Error("Couldn't read that file."));
      r.readAsArrayBuffer(f);
    });
  }

  async function restore(e: Event) {
    const input = e.currentTarget as HTMLInputElement;
    const f = input.files?.[0];
    if (!f) return;
    note = "";
    error = "";
    try {
      const r = await restoreBackup(await readBytes(f), invoke);
      note = `Restored ${r.added} transactions (${r.skippedDuplicates} were already here)` +
        (r.settingsRestored.length ? ` and your ${r.settingsRestored.join(", ").replace("user_rules", "category rules")}.` : ".");
      await load();
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    } finally {
      input.value = "";
    }
  }

  // Bulk select: checkbox per row, shift-click for a range, then file the whole selection under one category.
  let selected = $state<Set<number>>(new Set());
  let lastIdx = $state<number | null>(null);
  let bulkCategory = $state("");
  let allShown = $derived(shown.length > 0 && shown.every((r) => selected.has(r.id)));

  function toggle(i: number, on: boolean, range: boolean) {
    const next = new Set(selected);
    const [a, b] = range && lastIdx !== null ? [Math.min(lastIdx, i), Math.max(lastIdx, i)] : [i, i];
    for (let k = a; k <= b; k++) (on ? next.add(shown[k].id) : next.delete(shown[k].id));
    selected = next;
    lastIdx = i;
  }
  function toggleAll(on: boolean) {
    selected = on ? new Set(shown.map((r) => r.id)) : new Set();
    lastIdx = null;
  }
  async function applyBulk() {
    const v = bulkCategory.trim();
    if (!v || selected.size === 0) return;
    note = "";
    error = "";
    try {
      const ids = [...selected];
      for (const id of ids) await setCategory(id, v, remember, invoke);
      note = `Filed ${ids.length} ${ids.length === 1 ? "line" : "lines"} under ${v}.`;
      selected = new Set();
      bulkCategory = "";
      lastIdx = null;
      await load();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }

  // Split editor: one line at a time.
  let splitting = $state<number | null>(null);
  let drafts = $state<SplitDraft[]>([]);
  let splitError = $state("");
  function openSplit(r: TxnRow) {
    splitting = r.id;
    splitError = "";
    drafts = r.splits.length
      ? r.splits.map((p) => ({ category: p.category, amount: (Math.abs(p.cents) / 100).toFixed(2) }))
      : [{ category: r.category ?? "", amount: (Math.abs(r.amountCents) / 100).toFixed(2) }, { category: "", amount: "" }];
  }
  async function saveSplit(r: TxnRow) {
    const res = splitToParts(r.amountCents, drafts);
    if (!res.ok) {
      splitError = res.error;
      return;
    }
    try {
      await setSplits(r.id, res.parts, invoke);
      splitting = null;
      await load();
    } catch (e) {
      splitError = e instanceof Error ? e.message : String(e);
    }
  }
  async function clearSplit(r: TxnRow) {
    try {
      await setSplits(r.id, [], invoke);
      splitting = null;
      await load();
    } catch (e) {
      splitError = e instanceof Error ? e.message : String(e);
    }
  }

  async function saveExtras(row: TxnRow, noteText: string, tagText: string) {
    const tags = tagText.split(",").map((t) => t.trim().toLowerCase()).filter(Boolean);
    if (noteText.trim() === row.note && tags.join(",") === row.tags.join(",")) return;
    error = "";
    try {
      await setNoteTags(row.id, noteText, tagText, invoke);
      await load();
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
  <label class="check">Search <input type="search" bind:value={query} placeholder="payee, category, amount…" aria-label="Search transactions" /></label>
  <button type="button" onclick={exportCsv}>Export CSV</button>
  <button type="button" onclick={backup}>Back up everything</button>
  <label class="check">Restore from backup <input type="file" accept=".json" aria-label="Backup file" onchange={restore} /></label>
</div>

{#if onlyTodo && rows.length > 0}<CategorySuggest {invoke} ondone={load} />{/if}

{#if error}<p class="err" role="alert">{error}</p>{/if}
{#if note}<p role="status">{note}</p>{/if}

<datalist id="categories">{#each choices as c}<option value={c}></option>{/each}</datalist>

{#if loaded && rows.length === 0}
  <p class="empty">{onlyTodo ? "Everything has a category." : "No transactions yet. Import a statement first."}</p>
{:else if rows.length > 0}
  <p class="muted" data-testid="shown">{shown.length} of {rows.length} lines · in {formatCents(totals.inCents)} · out {formatCents(totals.outCents)}</p>
  {#if shown.length === 0}<p class="empty">Nothing matches that search.</p>{/if}
  <div class="tb-seg" role="group" aria-label="View">
    <button type="button" aria-pressed={view === "list"} onclick={() => (view = "list")}>List</button>
    <button type="button" aria-pressed={view === "board"} onclick={() => (view = "board")}>By category</button>
  </div>
  {#if view === "board"}
    <div class="board" data-testid="board">
      {#each lanes as lane (lane.category ?? "")}
        <section class="lane" aria-label={lane.category ?? "Needs a category"}>
          <header><b>{lane.category ?? "Needs a category"}</b><span class="muted">{lane.rows.length} · {formatCents(lane.totalCents)}</span></header>
          {#each lane.rows as r (r.id)}
            <div class="tile"><span>{r.payee}</span><span class="num" class:neg={r.amountCents < 0}>{formatCents(r.amountCents)}</span><small class="muted">{r.day}</small></div>
          {/each}
        </section>
      {/each}
    </div>
  {:else}
  {#if selected.size > 0}
    <div class="bulk" role="group" aria-label="Selected transactions" data-testid="bulk-bar">
      <b>{selected.size} selected</b>
      <input list="categories" bind:value={bulkCategory} placeholder="Category for all…" aria-label="Category for selected" />
      <button type="button" onclick={applyBulk} disabled={!bulkCategory.trim()}>File under this</button>
      <button type="button" onclick={() => toggleAll(false)}>Clear</button>
    </div>
  {/if}
  <table class="card">
    <thead><tr><th class="sel"><input type="checkbox" aria-label="Select all shown" checked={allShown} onchange={(e) => toggleAll(e.currentTarget.checked)} /></th><th>Date</th><th>Payee</th><th>Account</th><th class="r">Amount</th><th>Category</th><th>Note and tags</th><th>Split</th></tr></thead>
    <tbody>
      {#each shown as r, i (r.id)}
        <tr class:sel={selected.has(r.id)}>
          <td class="sel"><input type="checkbox" aria-label={`Select ${r.payee} on ${r.day}`} checked={selected.has(r.id)} onclick={(e) => toggle(i, e.currentTarget.checked, e.shiftKey)} /></td>
          <td class="num">{r.day}</td>
          <td>{r.payee}</td>
          <td class="muted">{r.account}</td>
          <td class="r num" class:neg={r.amountCents < 0}>{formatCents(r.amountCents)}</td>
          <td>
            <input list="categories" value={r.category ?? ""} placeholder="Choose…" aria-label={`Category for ${r.payee} on ${r.day}`} onchange={(e) => change(r, e.currentTarget.value)} />
          </td>
          <td class="extras">
            <input value={r.note ?? ""} placeholder="Note" aria-label={`Note for ${r.payee} on ${r.day}`} onchange={(e) => saveExtras(r, e.currentTarget.value, (r.tags ?? []).join(","))} />
            <input value={(r.tags ?? []).join(", ")} placeholder="tags, comma separated" aria-label={`Tags for ${r.payee} on ${r.day}`} onchange={(e) => saveExtras(r, r.note ?? "", e.currentTarget.value)} />
          </td>
          <td>
            <button type="button" aria-label={`Split ${r.payee} on ${r.day}`} onclick={() => openSplit(r)}>{(r.splits ?? []).length ? `Split ${r.splits.length} ways` : "Split"}</button>
          </td>
        </tr>
        {#if splitting === r.id}
          <tr data-testid="split-editor">
            <td></td>
            <td colspan="7">
              {#each drafts as p, i}
                <div class="part">
                  <input list="categories" bind:value={p.category} placeholder="Category" aria-label={`Part ${i + 1} category`} />
                  <input bind:value={p.amount} placeholder="0.00" inputmode="decimal" aria-label={`Part ${i + 1} amount`} />
                </div>
              {/each}
              <div class="part">
                <button type="button" onclick={() => (drafts = [...drafts, { category: "", amount: "" }])}>Add a part</button>
                <span class="muted" data-testid="split-left">{formatCents(splitLeftCents(r.amountCents, drafts))} left</span>
                <button type="button" onclick={() => saveSplit(r)}>Save split</button>
                {#if (r.splits ?? []).length}<button type="button" onclick={() => clearSplit(r)}>Remove split</button>{/if}
                <button type="button" onclick={() => (splitting = null)}>Cancel</button>
              </div>
              {#if splitError}<p class="err" role="alert">{splitError}</p>{/if}
            </td>
          </tr>
        {/if}
      {/each}
    </tbody>
  </table>
  {/if}
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
  td input:not([type="checkbox"]) { width: 100%; min-width: 150px; }
  .part { display: flex; gap: 8px; align-items: center; margin: 4px 0; }
  .part input:not([type="checkbox"]) { max-width: 200px; min-width: 100px; }
  .extras { display: flex; flex-direction: column; gap: 4px; }
  .sel { width: 28px; padding-right: 0; }
  tr.sel td { background: var(--accent-soft); }
  .bulk { display: flex; gap: 8px; align-items: center; margin: 0 0 8px; padding: 6px 10px; background: var(--accent-soft); border: 1px solid var(--accent); border-radius: var(--r-sm); font-size: 12.5px; color: var(--ink); }
  .bulk input { width: 220px; }
  .tb-seg { margin: 0 0 10px; }
  .board { display: flex; gap: 12px; overflow-x: auto; align-items: flex-start; padding-bottom: 8px; }
  .lane { flex: 0 0 240px; background: var(--surface); border: 1px solid var(--line); border-radius: var(--r-md); padding: 8px; display: grid; gap: 6px; }
  .lane header { display: flex; justify-content: space-between; gap: 8px; font-size: 12.5px; }
  .tile { display: grid; grid-template-columns: 1fr auto; gap: 0 8px; padding: 6px 8px; border: 1px solid var(--line); border-radius: var(--r-sm); font-size: 12.5px; }
  .tile small { grid-column: 1 / -1; }
</style>
