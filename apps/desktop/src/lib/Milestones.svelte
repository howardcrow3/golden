<script lang="ts">
  import type { QuickPlanResult } from "./api";

  let { result }: { result: QuickPlanResult } = $props();
  let items = $derived(result.milestones ?? []);
</script>

{#if items.length > 0}
  <section class="ms" aria-label="Milestones">
    <h3>Milestones</h3>
    <ol>
      {#each items as m (m.key)}
        <li data-testid={`ms-${m.key}`} class:warn={m.key === "depleted"}>
          <strong>{m.year}</strong> <span class="age">age {m.age}</span> {m.label}
        </li>
      {/each}
    </ol>
  </section>
{/if}

<style>
  .ms { margin-top: 16px; }
  h3 { margin: 0 0 6px; font-size: 15px; }
  ol { margin: 0; padding-left: 18px; font-size: 13px; }
  li { margin: 2px 0; }
  .age { color: var(--muted); }
  .warn { color: var(--danger); }
</style>
