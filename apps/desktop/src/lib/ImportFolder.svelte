<script lang="ts">
  import { importFolder, type FolderReport, type ImportReport, type Invoke } from "./api";

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
  <p class="muted">Imports every new CSV, OFX or QFX file in a folder, such as your Downloads. Files already imported are skipped. OFX and QFX files go to the account they name; CSV files go to the account chosen above.</p>
  <label>Folder (full path)
    <input bind:value={path} aria-label="Folder to import" placeholder="/Users/you/Downloads" />
  </label>
  <button type="button" onclick={run} disabled={busy || !path.trim()}>{busy ? "Importing…" : "Import new files"}</button>
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
