<script lang="ts">
  import type { Household } from "./api";
  import { plannerScore } from "./plannerScore";

  let { household }: { household: Household } = $props();
  let s = $derived(plannerScore(household));
  let open = $state(false);
  let missing = $derived(s.items.filter((i) => !i.done));
</script>

<section class="tb-section" aria-label="Planner score">
  <h2>Plan completeness <span class="num" data-testid="score">{s.score}</span><span class="of">/100</span></h2>
  <div class="bar" role="progressbar" aria-valuemin="0" aria-valuemax="100" aria-valuenow={s.score}><div style:width={`${s.score}%`}></div></div>
  {#if missing.length === 0}
    <p class="hint">Everything the plan needs is filled in.</p>
  {:else}
    <button type="button" class="link" onclick={() => (open = !open)} aria-expanded={open}>
      {open ? "Hide" : "Show"} what is missing ({missing.length})
    </button>
    {#if open}
      <ul>
        {#each missing as i}<li><strong>{i.label}.</strong> {i.hint}</li>{/each}
      </ul>
    {/if}
  {/if}
  {#if s.warnings.length}
    <ul class="warn" aria-label="Warnings">
      {#each s.warnings as w}<li>{w}</li>{/each}
    </ul>
  {/if}
</section>

<style>
  h2 { margin: 0 0 8px; }
  .of { color: var(--muted); font-size: 14px; margin-left: 2px; }
  .bar { height: 8px; background: var(--line); border-radius: 4px; overflow: hidden; margin-bottom: 8px; }
  .bar div { height: 100%; background: var(--gold); }
  .hint { color: var(--muted); font-size: 13px; margin: 0; }
  .link { background: none; border: none; padding: 0; color: inherit; text-decoration: underline; cursor: pointer; font-size: 13px; }
  ul { margin: 8px 0 0; padding-left: 18px; font-size: 13px; }
  .warn { color: var(--bad); }
</style>
