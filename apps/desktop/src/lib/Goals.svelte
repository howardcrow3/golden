<script lang="ts">
  import { getGoals, removeGoal, setGoal, type GoalRow, type Invoke } from "./api";
  import { formatCents, parseDollars } from "./money";

  let { invoke }: { invoke?: Invoke } = $props();
  let rows = $state<GoalRow[]>([]);
  let available = $state(false);
  let name = $state("");
  let target = $state("");
  let saved = $state("0");
  let by = $state("");
  let error = $state("");

  async function load() {
    try {
      rows = await getGoals(invoke);
      available = true;
    } catch {
      rows = [];
      available = false;
    }
  }
  $effect(() => {
    load();
  });

  async function add() {
    error = "";
    const t = parseDollars(target);
    const s = parseDollars(saved);
    if (!name.trim()) return void (error = "Give the goal a name.");
    if (t === null || t <= 0) return void (error = "Enter the target as a number, like 10000.");
    if (s === null || s < 0) return void (error = "Enter what's saved so far as a number, like 0.");
    if (!/^\d{4}-\d{2}-\d{2}$/.test(by)) return void (error = "Enter the date as YYYY-MM-DD.");
    try {
      await setGoal({ name: name.trim(), targetCents: t, savedCents: s, byDate: by }, invoke);
      name = target = by = "";
      saved = "0";
      await load();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }

  async function drop(n: string) {
    try {
      await removeGoal(n, invoke);
      await load();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }

  const words: Record<string, string> = {
    done: "Reached",
    on_track: "Within reach",
    behind: "Needs more than you've been keeping",
    past_due: "Past its date",
    unknown: "Import a few months to compare",
  };
</script>

{#if available}
  <section class="card" data-testid="goals">
    <h2>Goals</h2>
    {#each rows as g}
      <div class="goal" data-testid="goal-row">
        <b>{g.name}</b> <span class="num">{formatCents(g.savedCents, { whole: true })} of {formatCents(g.targetCents, { whole: true })}</span> by {g.byDate}
        <div class="bar" role="progressbar" aria-valuenow={g.percent} aria-valuemin="0" aria-valuemax="100"><span style={`width:${g.percent}%`}></span></div>
        <span class="muted">
          {#if g.status !== "done"}About <span class="num">{formatCents(g.monthlyNeededCents, { whole: true })}</span> a month. {/if}<span data-status={g.status}>{words[g.status]}.</span>
        </span>
        <button class="link" onclick={() => drop(g.name)} aria-label={`Remove ${g.name}`}>Remove</button>
      </div>
    {/each}
    <form onsubmit={(e) => { e.preventDefault(); add(); }}>
      <label>Goal <input aria-label="Goal name" bind:value={name} /></label>
      <label>Target $ <input aria-label="Goal target" bind:value={target} inputmode="decimal" /></label>
      <label>Saved so far $ <input aria-label="Goal saved so far" bind:value={saved} inputmode="decimal" /></label>
      <label>By (YYYY-MM-DD) <input aria-label="Goal date" bind:value={by} /></label>
      <button type="submit">Add goal</button>
    </form>
    {#if error}<p class="err" role="alert">{error}</p>{/if}
  </section>
{/if}

<style>
  section { margin-top: 16px; }
  .goal { margin-bottom: 12px; }
  .bar { height: 6px; background: var(--line); margin: 4px 0; max-width: 360px; }
  .bar span { display: block; height: 100%; background: var(--accent); }
  form { display: flex; gap: 12px; align-items: end; flex-wrap: wrap; }
  label { display: inline-flex; flex-direction: column; gap: 4px; font-size: 12px; color: var(--muted); }
  input { width: 120px; }
  .muted { color: var(--muted); }
  .err { color: var(--bad); }
  .link { background: none; border: none; padding: 0 0 0 8px; color: inherit; text-decoration: underline; cursor: pointer; }
</style>
