<script lang="ts">
  import { osNotify, type Invoke, type QuickPlanResult } from "./api";
  import { newlyUrgent } from "./alertNotify";

  let { result, invoke }: { result: QuickPlanResult; invoke?: Invoke } = $props();
  let items = $derived(result.alerts ?? []);
  // Tell the person once per heads-up, even when the window is in the background.
  $effect(() => {
    const fresh = newlyUrgent(items);
    if (fresh > 0) osNotify(fresh, invoke).catch(() => {});
  });
  const icon = { good: "✓", info: "•", warn: "!", danger: "!!" } as const;
</script>

{#if items.length > 0}
  <section class="alerts" aria-label="Heads-ups">
    <ul>
      {#each items as a (a.key)}
        <li class={a.severity} data-testid={`alert-${a.key}`}>
          <span class="icon" aria-hidden="true">{icon[a.severity]}</span>
          <span>{a.message}</span>
        </li>
      {/each}
    </ul>
  </section>
{/if}

<style>
  .alerts { margin-top: 12px; }
  ul { list-style: none; margin: 0; padding: 0; display: grid; gap: 4px; }
  li { display: flex; gap: 8px; font-size: 13px; padding: 6px 10px; border-radius: 6px; background: var(--surface); }
  .icon { font-weight: 700; width: 1.6em; }
  .good { color: var(--ok); }
  .warn { color: var(--ink); border-left: 3px solid var(--accent); padding-left: 6px; }
  .danger { color: var(--danger); }
</style>
