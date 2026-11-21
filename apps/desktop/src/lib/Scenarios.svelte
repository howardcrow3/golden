<script lang="ts">
  import {
    compareScenarios,
    deleteScenario,
    listScenarios,
    makePlan,
    undoablePlan,
    undoMakePlan,
    type CompareColumn,
    type Invoke,
    type SavedScenario,
  } from "./api";
  import { formatCents, formatPercent } from "./money";

  let { invoke, refresh = 0, onchange }: { invoke?: Invoke; refresh?: number; onchange?: () => void } = $props();
  let list = $state<SavedScenario[]>([]);
  let chosen = $state<number[]>([]);
  let columns = $state<CompareColumn[] | null>(null);
  let undo = $state<string | null>(null);
  let note = $state("");
  let error = $state("");
  let busy = $state(false);

  async function load() {
    try {
      const got = await listScenarios(invoke);
      list = Array.isArray(got) ? got : [];
      chosen = chosen.filter((id) => list.some((s) => s.id === id));
      const u = await undoablePlan(invoke);
      undo = typeof u === "string" && u ? u : null;
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }
  $effect(() => {
    refresh;
    void load();
  });

  function toggle(id: number) {
    chosen = chosen.includes(id) ? chosen.filter((x) => x !== id) : chosen.length < 3 ? [...chosen, id] : chosen;
  }
  async function compare() {
    busy = true;
    error = "";
    try {
      columns = (await compareScenarios(chosen, 400, invoke)).columns;
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      busy = false;
    }
  }
  async function remove(s: SavedScenario) {
    error = "";
    try {
      await deleteScenario(s.id, invoke);
      columns = null;
      await load();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }
  async function adopt(s: SavedScenario) {
    error = "";
    note = "";
    try {
      await makePlan(s.id, invoke);
      note = `"${s.name}" is now your plan.`;
      columns = null;
      await load();
      onchange?.();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }
  async function revert() {
    error = "";
    try {
      await undoMakePlan(invoke);
      note = "Your plan is back the way it was.";
      columns = null;
      await load();
      onchange?.();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }
</script>

<section class="card scen" aria-label="Saved scenarios">
  <b>Saved scenarios</b>
  {#if error}<p class="err" role="alert">{error}</p>{/if}
  {#if note}<p role="status" data-testid="scenario-note">{note}</p>{/if}
  {#if undo}
    <p>
      You made "{undo}" your plan.
      <button type="button" onclick={revert}>Undo</button>
      <span class="muted">Undo puts your plan back as it was before, including anything you changed since.</span>
    </p>
  {/if}
  {#if list.length === 0}
    <p class="muted">Nothing saved yet. Ask a what-if, then choose "Save as scenario" under its answer.</p>
  {:else}
    <ul>
      {#each list as s}
        <li data-testid="scenario">
          <label>
            <input type="checkbox" checked={chosen.includes(s.id)} disabled={!chosen.includes(s.id) && chosen.length >= 3} onchange={() => toggle(s.id)} aria-label={`Compare ${s.name}`} />
            <b>{s.name}</b>
          </label>
          <span class="muted">{s.descriptions.join("; ")}</span>
          <button type="button" onclick={() => adopt(s)}>Make this my plan</button>
          <button type="button" onclick={() => remove(s)} aria-label={`Delete ${s.name}`}>Delete</button>
        </li>
      {/each}
    </ul>
    <button type="button" onclick={compare} disabled={busy || chosen.length === 0}>{busy ? "Comparing…" : "Compare with my current plan"}</button>
    <span class="muted">Pick up to three.</span>
  {/if}
  {#if columns}
    <table data-testid="compare">
      <thead>
        <tr><th></th>{#each columns as c}<th>{c.name}</th>{/each}</tr>
      </thead>
      <tbody>
        <tr><th>Chance the plan works</th>{#each columns as c}<td>{formatPercent(c.result.successRate, 0)}</td>{/each}</tr>
        <tr><th>Typical end balance</th>{#each columns as c}<td>{formatCents(c.result.p50Cents, { whole: true })}</td>{/each}</tr>
        <tr><th>Bad-luck end balance</th>{#each columns as c}<td>{formatCents(c.result.p10Cents, { whole: true })}</td>{/each}</tr>
        <tr><th>Money runs out</th>{#each columns as c}<td>{c.result.depletedYear ?? "Not in the plan"}</td>{/each}</tr>
      </tbody>
    </table>
  {/if}
</section>

<style>
  .scen { display: grid; gap: 8px; margin: 16px 0; }
  ul { list-style: none; padding: 0; display: grid; gap: 8px; }
  li { display: flex; gap: 10px; align-items: center; flex-wrap: wrap; }
  .muted { color: var(--muted); }
  table { border-collapse: collapse; }
  th, td { text-align: left; padding: 4px 12px 4px 0; border-bottom: 1px solid var(--line); }
</style>
