<script lang="ts">
  import { connections, eraseAllData, type Connections, type Invoke } from "./api";
  import AiActivity from "./AiActivity.svelte";

  let { invoke }: { invoke?: Invoke } = $props();
  let data = $state<Connections | null>(null);
  let error = $state("");
  let word = $state("");
  let erased = $state(false);
  async function erase() {
    error = "";
    try {
      await eraseAllData(word, invoke);
      erased = true;
      word = "";
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }
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

<section class="card erase">
  <h2>Delete everything</h2>
  <p class="muted">Removes every account, transaction, household member, budget, rule and setting from this computer in one step. It cannot be undone, so export a backup first if you might want it. Your trial or licence is kept. Statement files already filed in your statement folder are not touched; delete that folder yourself if you want them gone.</p>
  {#if erased}<p role="status" data-testid="erased">Everything has been deleted. Restart Golden to start fresh.</p>{/if}
  <label>Type DELETE to confirm <input aria-label="Confirm deletion" bind:value={word} /></label>
  <button type="button" onclick={erase} disabled={word.trim() !== "DELETE"}>Delete everything</button>
</section>

<style>
  .erase { margin-top: 24px; }
  .erase label { display: flex; flex-direction: column; gap: 4px; margin: 8px 0; max-width: 240px; }
  .list { list-style: none; padding: 0; display: grid; gap: 12px; }
  .head { display: flex; justify-content: space-between; align-items: center; }
  .muted { color: var(--muted); }
  .status { font-weight: 600; }
  .pill { font-size: 12px; padding: 2px 8px; border-radius: 10px; border: 1px solid var(--line); color: var(--muted); }
  .pill.on { color: inherit; border-color: currentColor; }
</style>
