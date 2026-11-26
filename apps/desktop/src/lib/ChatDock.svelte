<script lang="ts">
  import Ask from "./Ask.svelte";
  import type { Invoke } from "./api";

  let { page, onhide, invoke }: { page: string; onhide: () => void; invoke?: Invoke } = $props();
</script>

<aside class="dock" aria-label="Chat">
  <div class="dh">
    <span>Chat</span>
    <button class="tb-xbtn" aria-label="Hide chat" onclick={onhide}>×</button>
  </div>
  <div class="ctx"><span class="tb-badge">Looking at {page}</span></div>
  <div class="dt"><Ask compact {invoke} /></div>
</aside>

<style>
  .dock { width: 300px; border-left: 1px solid var(--line); display: flex; flex-direction: column; min-height: 0; background: var(--paper); }
  .dh { height: 34px; display: flex; align-items: center; justify-content: space-between; padding: 0 10px 0 14px; font-weight: 500; color: var(--ink); border-bottom: 1px solid var(--line); }
  .ctx { padding: 8px 14px 0; }
  .dt { flex: 1; overflow-y: auto; padding: 10px 14px 14px; min-height: 0; }
</style>
