<script lang="ts">
  import { importFolder, stopWatching, watchFolder, watchedFolder, type FolderReport, type ImportReport, type Invoke, type Watch } from "./api";

  let {
    invoke,
    account,
    kind,
    onreport,
  }: { invoke?: Invoke; account: string; kind: string; onreport?: (r: ImportReport[]) => void } = $props();
  let path = $state("");
  let busy = $state(false);
  let error = $state("");
  let result = $state<FolderReport | null>(null);

  let watching = $state<Watch | null>(null);
  $effect(() => {
    watchedFolder(invoke)
      .then((w) => {
        watching = w;
        if (w && !path) path = w.path;
      })
      .catch(() => (watching = null));
  });
  async function watch() {
    error = "";
    try {
      watching = await watchFolder(path, account, kind, invoke);
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }
  async function unwatch() {
    error = "";
    try {
      await stopWatching(invoke);
      watching = null;
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }

  async function run() {
    busy = true;
    error = "";
    result = null;
    try {
      result = await importFolder(path, account, kind, invoke);
      if (result.imported.length) onreport?.(result.imported);
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      busy = false;
    }
  }
</script>

<section class="card folder">
  <b>Import a whole folder</b>
  <p class="muted">Imports every new CSV, OFX, QFX or PDF statement in a folder, such as your Downloads. Files already imported are skipped. OFX and QFX files go to the account they name; CSV files go to the account chosen above.</p>
  <label>Folder (full path)
    <input bind:value={path} aria-label="Folder to import" placeholder="/Users/you/Downloads" />
  </label>
  <button type="button" onclick={run} disabled={busy || !path.trim()}>{busy ? "Importing…" : "Import new files"}</button>
  {#if watching}
    <p role="status" data-testid="watching">Watching {watching.path}. New statements are imported every few minutes while Golden is open.</p>
    <button type="button" onclick={unwatch}>Stop watching</button>
  {:else}
    <button type="button" onclick={watch} disabled={!path.trim()}>Keep watching this folder</button>
  {/if}
  {#if error}<p class="err" role="alert">{error}</p>{/if}
  {#if result}
    <p role="status" data-testid="folder-result">
      Found {result.found} statement {result.found === 1 ? "file" : "files"}: {result.imported.length} imported,
      {result.alreadyImported} already imported{#if result.failed.length}, {result.failed.length} could not be read{/if}.
    </p>
    {#if result.failed.length}
      <ul>
        {#each result.failed as f}<li data-testid="folder-failure">{f.name}: {f.error}</li>{/each}
      </ul>
    {/if}
  {/if}
</section>

<style>
  .folder { display: grid; gap: 8px; margin-bottom: 16px; }
  .muted { color: var(--muted); margin: 0; }
  label { display: flex; flex-direction: column; gap: 4px; font-size: 12px; color: var(--muted); }
</style>
