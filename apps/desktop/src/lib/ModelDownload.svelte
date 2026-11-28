<script lang="ts">
  import { removeModel, type Invoke } from "./api";
  import { modelDownload, view } from "./downloads.svelte";
  import DownloadRow from "./DownloadRow.svelte";

  let { invoke, pollMs = 500 }: { invoke?: Invoke; pollMs?: number } = $props();
  // svelte-ignore state_referenced_locally
  const md = modelDownload(invoke, pollMs);
  let removeError = $state("");
  let item = $derived(view.items.find((i) => i.id === `model:${md.status?.id}`));

  $effect(() => {
    md.load();
  });

  async function remove() {
    if (!md.status) return;
    try {
      await removeModel(md.status.id, invoke);
      await md.refresh();
    } catch (e) {
      removeError = e instanceof Error ? e.message : String(e);
    }
  }
  const gb = (mb: number) => (mb / 1000).toFixed(1);
  let error = $derived(md.error || removeError);
</script>

<section class="card dl" aria-label="Model on this computer">
  <h2>Model on this computer</h2>
  {#if md.status}
    {#if md.status.installed}
      <p>{md.status.name} is installed. Everything runs on this computer and nothing is sent anywhere.</p>
      <button type="button" onclick={remove}>Remove</button>
    {:else if item && item.state !== "done" && item.state !== "canceled" && item.state !== "failed"}
      <p>Downloading {md.status.name}. Progress is also under the download button at the top right.</p>
      <div class="tb-dl-card"><DownloadRow {item} /></div>
    {:else}
      <p>Get {md.status.name} ({gb(md.status.sizeMb)} GB), chosen for this computer's {md.status.ramGb} GB of memory. It downloads once, can resume if interrupted, and is checked before use.</p>
      <button type="button" onclick={() => md.start()}>Download</button>
    {/if}
    {#if md.progress.error}<p class="err" role="alert">{md.progress.error}</p>{/if}
  {/if}
  {#if error}<p class="err" role="alert">{error}</p>{/if}
</section>

<style>
  .dl { margin: 12px 0; }
  .tb-dl-card { margin: 8px 0; }
  .err { color: var(--bad); }
</style>
