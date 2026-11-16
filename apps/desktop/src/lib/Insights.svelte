<script lang="ts">
  import { getInsights, type Insight, type Invoke } from "./api";

  let { invoke }: { invoke?: Invoke } = $props();
  let items = $state<Insight[]>([]);

  $effect(() => {
    getInsights(invoke)
      .then((r) => (items = r))
      .catch(() => (items = []));
  });
</script>

{#if items.length}
  <section class="card" data-testid="insights">
    <h2>What stands out</h2>
    <ul>
      {#each items as i}
        <li data-kind={i.kind}>{i.text}</li>
      {/each}
    </ul>
  </section>
{/if}

<style>
  section { margin-top: 16px; }
  ul { margin: 0; padding-left: 18px; }
  li { margin: 6px 0; }
</style>
