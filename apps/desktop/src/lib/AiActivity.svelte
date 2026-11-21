<script lang="ts">
  import { aiActivity, clearAiActivity, type AiActivityEntry, type Invoke } from "./api";

  let { invoke }: { invoke?: Invoke } = $props();
  let entries = $state<AiActivityEntry[] | null>(null);
  let error = $state("");

  const TASK: Record<string, string> = {
    whatif: "What-if question",
    categorize: "Categorize transactions",
    name_statement: "Name a statement",
    explain: "Explain results",
    suggest_assumptions: "Suggest assumptions",
  };

  async function load() {
    error = "";
    try {
      entries = await aiActivity(invoke);
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }
  async function clear() {
    try {
      await clearAiActivity(invoke);
      entries = [];
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }
  function ontoggle(e: Event) {
    if ((e.currentTarget as HTMLDetailsElement).open) void load();
  }
</script>

<details class="activity" {ontoggle}>
  <summary>What the AI did</summary>
  <p class="muted">Every time the model ran this session. Only when, what for, how long and how big are kept, never the text. Closing Golden clears this.</p>
  {#if error}<p class="err" role="alert">{error}</p>{/if}
  {#if entries && entries.length === 0}<p role="status">The model hasn't run yet.</p>{/if}
  {#if entries && entries.length > 0}
    <ul>
      {#each entries as e}
        <li data-testid="activity-row">
          {e.at}: {TASK[e.task] ?? e.task}, {e.ok ? "worked" : "failed"}, {(e.millis / 1000).toFixed(1)}s, {e.promptChars} characters in, {e.replyChars} out{#if e.error} ({e.error}){/if}
        </li>
      {/each}
    </ul>
    <button type="button" onclick={clear}>Clear</button>
  {/if}
</details>

<style>
  .activity { margin: 16px 0; }
  .muted { color: var(--muted); }
  ul { padding-left: 18px; }
</style>
