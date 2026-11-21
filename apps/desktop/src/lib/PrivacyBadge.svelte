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
  .mode { margin-top: 12px; font-size: 12px; color: var(--muted); text-align: left; border-color: transparent; background: transparent; padding: 0; cursor: pointer; }
</style>
