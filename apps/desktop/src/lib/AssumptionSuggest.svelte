<script lang="ts">
  import { setPick, suggestAssumptions, type AssumptionSuggestion, type Invoke } from "./api";
  import { assumptionName } from "./assumptions";

  let { invoke, onchange }: { invoke?: Invoke; onchange?: () => void } = $props();
  let suggestions = $state<AssumptionSuggestion[] | null>(null);
  let busy = $state(false);
  let error = $state("");

  async function ask() {
    busy = true;
    error = "";
    try {
      suggestions = await suggestAssumptions(invoke);
    } catch (e) {
      suggestions = null;
      error = e instanceof Error ? e.message : String(e);
    } finally {
      busy = false;
    }
  }
  async function accept(s: AssumptionSuggestion) {
    try {
      await setPick(s.key, s.label, invoke);
      suggestions = (suggestions ?? []).filter((x) => x.key !== s.key);
      onchange?.();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }
  function reject(s: AssumptionSuggestion) {
    suggestions = (suggestions ?? []).filter((x) => x.key !== s.key);
  }
</script>

<div class="suggest">
  <button type="button" onclick={ask} disabled={busy}>{busy ? "Thinking…" : "Suggest assumptions"}</button>
  <span class="muted">Picks among the options above that may suit you better. Nothing changes until you accept.</span>
  {#if error}<p class="err" role="alert">{error}</p>{/if}
  {#if suggestions && suggestions.length === 0}<p role="status">Nothing to suggest: the current assumptions look right for you.</p>{/if}
  {#each suggestions ?? [] as s (s.key)}
    <div class="item" data-testid="assumption-suggestion">
      <strong>{assumptionName(s.key)}: {s.label}</strong>
      <span class="muted">{s.source === "model" ? "(suggested by the model on this computer)" : "(suggested by Golden)"}</span>
      <p>{s.reason}</p>
      <button type="button" onclick={() => accept(s)} aria-label={`Accept ${s.label} for ${assumptionName(s.key)}`}>Accept</button>
      <button type="button" onclick={() => reject(s)} aria-label={`Reject ${s.label} for ${assumptionName(s.key)}`}>Reject</button>
    </div>
  {/each}
</div>

<style>
  .suggest { margin-top: 12px; }
  .muted { color: var(--muted); font-size: 12px; }
  .item { margin: 10px 0; }
  .err { color: var(--bad); }
</style>
