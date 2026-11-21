<script lang="ts">
  import { cancelModelDownload, modelProgress, modelStatus, removeModel, startModelDownload, type Invoke, type ModelProgress, type ModelStatus } from "./api";

  let { invoke, pollMs = 500 }: { invoke?: Invoke; pollMs?: number } = $props();
  let status = $state<ModelStatus | null>(null);
  let progress = $state<ModelProgress>({ running: false, downloadedMb: 0, totalMb: 0, error: "" });
  let error = $state("");
  let timer: ReturnType<typeof setInterval> | undefined;

  async function refresh() {
    try {
      status = await modelStatus(invoke);
      error = "";
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }

  async function poll() {
    try {
      const was = progress.running;
      progress = await modelProgress(invoke);
      if (!progress.running) {
        stop();
        if (was) await refresh();
      }
    } catch {
      stop();
    }
  }

  function watch() {
    stop();
    timer = setInterval(poll, pollMs);
  }
  function stop() {
    if (timer) clearInterval(timer);
    timer = undefined;
  }

  $effect(() => {
    refresh();
    modelProgress(invoke)
      .then((p) => {
        progress = p;
        if (p.running) watch();
      })
      .catch(() => {});
    return stop;
  });

  async function download() {
    if (!status) return;
    error = "";
    try {
      await startModelDownload(status.id, invoke);
      progress = { running: true, downloadedMb: 0, totalMb: status.sizeMb, error: "" };
      watch();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }
  async function cancel() {
    await cancelModelDownload(invoke).catch(() => {});
  }
  async function remove() {
    if (!status) return;
    try {
      await removeModel(status.id, invoke);
      await refresh();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }

  const pct = $derived(progress.totalMb > 0 ? Math.min(100, Math.round((progress.downloadedMb / progress.totalMb) * 100)) : 0);
  const gb = (mb: number) => (mb / 1000).toFixed(1);
</script>

<section class="card dl" aria-label="Model on this computer">
  <h2>Model on this computer</h2>
  {#if status}
    {#if status.installed}
      <p>{status.name} is installed. Everything runs on this computer and nothing is sent anywhere.</p>
      <button type="button" onclick={remove}>Remove</button>
    {:else if progress.running}
      <p>Downloading {status.name}: {gb(progress.downloadedMb)} of {gb(progress.totalMb)} GB</p>
      <div class="bar" role="progressbar" aria-valuemin="0" aria-valuemax="100" aria-valuenow={pct} aria-label="Download progress"><span style="width:{pct}%"></span></div>
      <button type="button" onclick={cancel}>Cancel</button>
    {:else}
      <p>Get {status.name} ({gb(status.sizeMb)} GB), chosen for this computer's {status.ramGb} GB of memory. It downloads once, can resume if interrupted, and is checked before use.</p>
      <button type="button" onclick={download}>Download</button>
    {/if}
    {#if progress.error}<p class="err" role="alert">{progress.error}</p>{/if}
  {/if}
  {#if error}<p class="err" role="alert">{error}</p>{/if}
</section>

<style>
  .dl { margin: 12px 0; }
  .bar { height: 8px; background: var(--border, #ddd); border-radius: 4px; overflow: hidden; margin: 8px 0; }
  .bar span { display: block; height: 100%; background: var(--accent, #c9a227); }
  .err { color: var(--bad); }
</style>
