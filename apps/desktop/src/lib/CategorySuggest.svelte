<script lang="ts">
  import { setCategory, suggestCategories, type CategorySuggestion, type Invoke } from "./api";

  let { invoke, ondone }: { invoke?: Invoke; ondone?: () => void } = $props();
  let suggestions = $state<CategorySuggestion[] | null>(null);
  let busy = $state(false);
  let error = $state("");
  let note = $state("");

  async function ask() {
    busy = true;
    error = "";
    note = "";
    try {
      suggestions = await suggestCategories(invoke);
      if (suggestions.length === 0) note = "The model had no confident suggestions for the lines that need a category.";
    } catch (e) {
      suggestions = null;
      error = e instanceof Error ? e.message : String(e);
    } finally {
      busy = false;
    }
  }

  async function accept(s: CategorySuggestion) {
    try {
      await setCategory(s.id, s.category, true, invoke);
      suggestions = (suggestions ?? []).filter((x) => x.id !== s.id);
      ondone?.();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }
  async function acceptAll() {
    for (const s of [...(suggestions ?? [])]) await accept(s);
  }
  function skip(s: CategorySuggestion) {
    suggestions = (suggestions ?? []).filter((x) => x.id !== s.id);
  }
</script>

<section class="card suggest" aria-label="Category suggestions">
  <button type="button" onclick={ask} disabled={busy}>{busy ? "Thinking…" : "Suggest categories with the model"}</button>
  <span class="muted">Uses the model on this computer. It sees only the merchant text, never amounts or dates. Nothing changes until you accept.</span>
  {#if error}<p class="err" role="alert">{error}</p>{/if}
  {#if note}<p role="status">{note}</p>{/if}
  {#if suggestions && suggestions.length > 0}
    <ul>
      {#each suggestions as s (s.id)}
        <li data-testid="suggestion">
          <span class="payee">{s.payee}</span> → <strong>{s.category}</strong>
          <button type="button" onclick={() => accept(s)} aria-label={`Accept ${s.category} for ${s.payee}`}>Accept</button>
          <button type="button" onclick={() => skip(s)} aria-label={`Skip ${s.payee}`}>Skip</button>
        </li>
      {/each}
    </ul>
    <button type="button" onclick={acceptAll}>Accept all</button>
  {/if}
</section>

<style>
  .suggest { margin: 12px 0; }
  .muted { color: var(--muted); margin-left: 8px; }
  .err { color: var(--bad); }
  ul { list-style: none; padding: 0; }
  li { margin: 6px 0; }
</style>
