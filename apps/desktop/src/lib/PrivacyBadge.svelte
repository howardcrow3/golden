<script lang="ts">
  import { connections, type Invoke } from "./api";

  let { invoke, onclick }: { invoke?: Invoke; onclick?: () => void } = $props();
  /** null while loading or outside the desktop app. */
  let isPrivate = $state<boolean | null>(null);
  $effect(() => {
    connections(invoke)
      .then((c) => (isPrivate = c.private))
      .catch(() => (isPrivate = null));
  });
</script>

<button type="button" class="mode" data-testid="privacy-badge" {onclick} aria-label="Privacy details">
  {#if isPrivate === false}
    Connected to an outside service
  {:else}
    Private mode · nothing leaves this computer
  {/if}
</button>

<style>
  .mode { margin: 4px 0 0; font-size: 11px; color: var(--muted); text-align: left; border-color: transparent; background: transparent; padding: 4px 8px; height: auto; line-height: 1.35; font-weight: 400; }
</style>
