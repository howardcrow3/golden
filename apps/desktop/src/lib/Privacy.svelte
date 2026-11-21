<script lang="ts">
  import { connections, type Connections, type Invoke } from "./api";
  import AiActivity from "./AiActivity.svelte";

  let { invoke }: { invoke?: Invoke } = $props();
  let data = $state<Connections | null>(null);
  let error = $state("");
  $effect(() => {
    connections(invoke)
      .then((d) => (data = d))
      .catch((e) => (error = e instanceof Error ? e.message : String(e)));
  });
</script>

<h1>Privacy</h1>
{#if error}<p class="err" role="alert">{error}</p>{/if}
{#if data}
  <p class="status" data-testid="private-status" class:ok={data.private}>
    {data.private ? "Private: nothing leaves this computer." : "Something is connected to an outside service. See below."}
  </p>
  <p class="muted">Every outside service Golden can reach, what it sends, and when. Your statements and balances are stored only on this computer.</p>
  <ul class="list">
    {#each data.items as c}
      <li class="card" data-testid="connection">
        <div class="head"><b>{c.name}</b><span class="pill" class:on={c.inUse}>{c.inUse ? "In use" : "Off"}</span></div>
        <div class="muted">{c.host} · {c.when}</div>
        <p>{c.sends}</p>
      </li>
    {/each}
  </ul>
{/if}
<AiActivity {invoke} />

<style>
  .list { list-style: none; padding: 0; display: grid; gap: 12px; }
  .head { display: flex; justify-content: space-between; align-items: center; }
  .muted { color: var(--muted); }
  .status { font-weight: 600; }
  .pill { font-size: 12px; padding: 2px 8px; border-radius: 10px; border: 1px solid var(--line); color: var(--muted); }
  .pill.on { color: inherit; border-color: currentColor; }
</style>
