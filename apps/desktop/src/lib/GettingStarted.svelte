<script lang="ts">
  import { setupProgress, type Invoke, type Setup } from "./api";

  let { invoke, onnavigate }: { invoke?: Invoke; onnavigate?: (page: string) => void } = $props();
  let setup = $state<Setup | null>(null);

  $effect(() => {
    setupProgress(invoke)
      .then((s) => (setup = s))
      .catch(() => (setup = null));
  });
</script>

{#if setup && !setup.complete}
  <section class="card" data-testid="getting-started">
    <h2>Getting started · {setup.done} of {setup.steps.length}</h2>
    <ul>
      {#each setup.steps as s (s.key)}
        <li class:done={s.done}>
          <span class="tick" aria-hidden="true">{s.done ? "✓" : "○"}</span>
          {#if s.done}
            <span>{s.title}</span><span class="sr">done</span>
          {:else}
            <button type="button" class="link" onclick={() => onnavigate?.(s.page)}>{s.title}</button>
          {/if}
        </li>
      {/each}
    </ul>
  </section>
{/if}

<style>
  section { margin-bottom: 16px; }
  ul { list-style: none; margin: 0; padding: 0; }
  li { display: flex; gap: 10px; align-items: center; padding: 6px 0; border-bottom: 1px solid var(--line); }
  li.done { color: var(--muted); }
  .tick { width: 1.2em; }
  .link { border: none; background: none; padding: 0; text-align: left; text-decoration: underline; cursor: pointer; color: inherit; font: inherit; }
  .sr { position: absolute; left: -9999px; }
</style>
