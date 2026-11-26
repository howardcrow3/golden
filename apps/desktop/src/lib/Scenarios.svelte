<script lang="ts">
  import {
    compareScenarios,
    deleteScenario,
    duplicateScenario,
    moveScenario,
    renameScenario,
    listScenarios,
    makePlan,
    undoablePlan,
    undoMakePlan,
    type CompareColumn,
    type Invoke,
    type SavedScenario,
  } from "./api";
  import { formatCents, formatPercent } from "./money";
  import { linePoints } from "./chart";

  let { invoke, refresh = 0, onchange }: { invoke?: Invoke; refresh?: number; onchange?: () => void } = $props();
  let list = $state<SavedScenario[]>([]);
  let chosen = $state<number[]>([]);
  let columns = $state<CompareColumn[] | null>(null);
  let undo = $state<string | null>(null);
  let note = $state("");
  let error = $state("");
  let busy = $state(false);
  let renaming = $state<number | null>(null);
  let draft = $state("");

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
  async function act(f: () => Promise<unknown>) {
    error = "";
    note = "";
    try {
      await f();
      columns = null;
      await load();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }
  function startRename(s: SavedScenario) {
    renaming = s.id;
    draft = s.name;
  }
  async function saveRename(s: SavedScenario) {
    await act(() => renameScenario(s.id, draft, invoke));
    if (!error) renaming = null;
  }
  const colours = ["var(--ink)", "var(--s1)", "var(--s2)", "var(--s3)"];
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
            {#if renaming === s.id}
              <input bind:value={draft} aria-label={`New name for ${s.name}`} />
            {:else}
              <b>{s.name}</b>
            {/if}
          </label>
          {#if renaming === s.id}
            <button type="button" onclick={() => saveRename(s)}>Save name</button>
            <button type="button" onclick={() => (renaming = null)}>Cancel</button>
          {:else}
            <button type="button" onclick={() => startRename(s)} aria-label={`Rename ${s.name}`}>Rename</button>
            <button type="button" onclick={() => act(() => duplicateScenario(s.id, invoke))} aria-label={`Duplicate ${s.name}`}>Duplicate</button>
            <button type="button" onclick={() => act(() => moveScenario(s.id, -1, invoke))} aria-label={`Move ${s.name} up`}>↑</button>
            <button type="button" onclick={() => act(() => moveScenario(s.id, 1, invoke))} aria-label={`Move ${s.name} down`}>↓</button>
          {/if}
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
    {#if columns.some((c) => c.result.years.length > 1)}
      <figure>
        <svg viewBox="0 0 640 180" role="img" aria-label="Median balance by year, plans overlaid" data-testid="overlay">
          {#each columns as c, i}
            <polyline fill="none" stroke={colours[i % colours.length]} stroke-width="2" points={linePoints(c.result.years.map((y) => y.p50Cents ?? y.balanceCents), 640, 180)} />
          {/each}
        </svg>
        <figcaption>
          {#each columns as c, i}<span class="key" style:color={colours[i % colours.length]}>━ {c.name}</span>{/each}
        </figcaption>
      </figure>
    {/if}
  {/if}
</section>

<style>
  .scen { display: grid; gap: 8px; margin: 16px 0; }
  ul { list-style: none; padding: 0; display: grid; gap: 8px; }
  li { display: flex; gap: 10px; align-items: center; flex-wrap: wrap; }
  .muted { color: var(--muted); }
  .key { margin-right: 12px; font-size: 12px; }
  figure { margin: 8px 0; }
  svg { width: 100%; height: 180px; border: 1px solid var(--line); background: var(--surface); border-radius: var(--r-md); }
  table { border-collapse: collapse; }
  th, td { text-align: left; padding: 4px 12px 4px 0; border-bottom: 1px solid var(--line); }
</style>
