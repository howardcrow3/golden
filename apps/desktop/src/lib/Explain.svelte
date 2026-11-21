<script lang="ts">
  import { explainResult, type ExplainFacts, type Explanation, type Invoke } from "./api";

  let { facts, invoke }: { facts: ExplainFacts; invoke?: Invoke } = $props();
  let result = $state<Explanation | null>(null);
  let busy = $state(false);
  let error = $state("");

  async function explain() {
    busy = true;
    error = "";
    try {
      result = await explainResult(facts, invoke);
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      busy = false;
    }
  }
</script>

<div class="explain">
  <button type="button" onclick={explain} disabled={busy}>{busy ? "Thinking…" : "Explain in plain English"}</button>
  {#if result}
    <p data-testid="explanation">{result.text}</p>
    <p class="muted">{result.source === "model" ? "Written by the model on this computer from the numbers above only." : "Written by Golden from the numbers above."}</p>
  {/if}
  {#if error}<p class="err" role="alert">{error}</p>{/if}
</div>

<style>
  .explain { margin-top: 12px; }
  .muted { color: var(--muted); font-size: 12px; }
  .err { color: var(--bad); }
</style>
