<script lang="ts">
  import { importFile, libraryFolder, setLibraryFolder, type ImportReport, type Invoke } from "./api";
  import { formatCents } from "./money";
  import ImportFolder from "./ImportFolder.svelte";
  import Paystub from "./Paystub.svelte";
  import Withholding from "./Withholding.svelte";
  let bump = $state(0);

  let { invoke }: { invoke?: Invoke } = $props();
  let account = $state("Checking");
  let kind = $state("checking");
  let reports = $state<ImportReport[]>([]);
  let error = $state("");
  let busy = $state(false);
  let folder = $state<string | null>(null);
  $effect(() => {
    libraryFolder(invoke).then((p) => (folder = p)).catch(() => (folder = null));
  });

  let editing = $state(false);
  let draft = $state("");
  let folderNote = $state("");
  async function saveFolder(path: string) {
    error = "";
    folderNote = "";
    try {
      folder = await setLibraryFolder(path, invoke);
      editing = false;
      folderNote = path.trim() ? "New imports will be filed here. Files already filed stay where they are." : "Back to the default folder.";
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }

  function readBytes(f: File): Promise<Uint8Array> {
    return new Promise((resolve, reject) => {
      const r = new FileReader();
      r.onload = () => resolve(new Uint8Array(r.result as ArrayBuffer));
      r.onerror = () => reject(new Error("could not read the file"));
      r.readAsArrayBuffer(f);
    });
  }

  async function onFiles(e: Event) {
    const input = e.target as HTMLInputElement;
    const files = Array.from(input.files ?? []);
    error = "";
    busy = true;
    for (const f of files) {
      try {
        const bytes = await readBytes(f);
        reports = [...reports, await importFile(f.name, bytes, account, kind, invoke)];
      } catch (err) {
        error = `${f.name}: ${err instanceof Error ? err.message : String(err)}`;
      }
    }
    busy = false;
    input.value = "";
  }
</script>

<h1>Import statements</h1>
<p class="sub">CSV, OFX, QFX or text PDF statements from your bank. Files stay on this computer. Importing a file twice adds nothing.{#if folder} Originals are filed by year, type and month in <span data-testid="folder">{folder}</span>.{/if}</p>

{#if folder}
  <p class="sub">
    {#if !editing}
      <button type="button" class="link" onclick={() => { draft = folder ?? ""; editing = true; }}>Change statement folder</button>
    {:else}
      <label>Statement folder (full path)
        <input bind:value={draft} aria-label="Statement folder" />
      </label>
      <button type="button" onclick={() => saveFolder(draft)}>Use this folder</button>
      <button type="button" onclick={() => saveFolder("")}>Use the default</button>
      <button type="button" onclick={() => (editing = false)}>Cancel</button>
    {/if}
  </p>
  {#if folderNote}<p role="status">{folderNote}</p>{/if}
{/if}

<section class="card form">
  <label>Account name <input bind:value={account} aria-label="Account name" /></label>
  <label>Type
    <select bind:value={kind} aria-label="Account type">
      <option value="checking">Checking</option>
      <option value="savings">Savings</option>
      <option value="credit">Credit card</option>
      <option value="investment">Investment</option>
      <option value="retirement">Retirement</option>
      <option value="loan">Loan</option>
    </select>
  </label>
  <label>Files <input type="file" multiple accept=".csv,.ofx,.qfx,.pdf" aria-label="Statement files" onchange={onFiles} disabled={busy} /></label>
</section>

{#if error}<p class="err" role="alert">{error}</p>{/if}

<ImportFolder {invoke} {account} {kind} onreport={(r) => (reports = [...reports, ...r])} />

{#each reports as r}
  <section class="card report" data-testid="report">
    <b>{r.account}</b>: {r.added} added, {r.skippedDuplicates} already imported
    ({r.parsed} lines, net {formatCents(r.totalCents)}){#if r.uncategorized > 0}; {r.uncategorized} need a category{/if}.{#if r.filedAs} Filed as <span class="filed">{r.filedAs}</span>.{/if}
    {#if r.reconciled === true}
      <span class="ok" data-testid="recon">Balance matches the statement ({formatCents(r.statementBalanceCents ?? 0)}).</span>
    {:else if r.reconciled === false}
      <span class="warn" data-testid="recon" role="status">
        The statement says {formatCents(r.statementBalanceCents ?? 0)} but Golden has {formatCents(r.balanceAfterCents ?? 0)}
        (off by {formatCents(r.differenceCents ?? 0)}). If this is the account's first statement, set its opening balance to fix it; otherwise a statement may be missing.
      </span>
    {/if}
  </section>
{/each}

<Paystub {invoke} onapplied={() => bump++} />
{#key bump}<Withholding {invoke} />{/key}

<style>
  .ok { color: var(--ok, #2e7d32); }
  .warn { color: var(--warn, #b26a00); }
  .sub { color: var(--muted); margin: 0 0 20px; }
  .form { display: flex; gap: 16px; align-items: end; flex-wrap: wrap; margin-bottom: 16px; }
  label { display: flex; flex-direction: column; gap: 4px; font-size: 12px; color: var(--muted); }
  .report { margin-bottom: 8px; }
  .err { color: var(--bad); }
  .link { background: none; border: none; padding: 0; color: inherit; text-decoration: underline; cursor: pointer; }
</style>
