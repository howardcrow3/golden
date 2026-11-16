<script lang="ts">
  import { importStatement, type ImportReport, type Invoke } from "./api";
  import { formatCents } from "./money";

  let { invoke }: { invoke?: Invoke } = $props();
  let account = $state("Checking");
  let kind = $state("checking");
  let reports = $state<ImportReport[]>([]);
  let error = $state("");
  let busy = $state(false);

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
        reports = [...reports, await importStatement(bytes, account, kind, invoke)];
      } catch (err) {
        error = `${f.name}: ${err instanceof Error ? err.message : String(err)}`;
      }
    }
    busy = false;
    input.value = "";
  }
</script>

<h1>Import statements</h1>
<p class="sub">CSV, OFX or QFX files from your bank. Files stay on this computer. Importing a file twice adds nothing.</p>

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
  <label>Files <input type="file" multiple accept=".csv,.ofx,.qfx" aria-label="Statement files" onchange={onFiles} disabled={busy} /></label>
</section>

{#if error}<p class="err" role="alert">{error}</p>{/if}

{#each reports as r}
  <section class="card report" data-testid="report">
    <b>{r.account}</b>: {r.added} added, {r.skippedDuplicates} already imported
    ({r.parsed} lines, net {formatCents(r.totalCents)}){#if r.uncategorized > 0}; {r.uncategorized} need a category{/if}.
  </section>
{/each}

<style>
  .sub { color: var(--muted); margin: 0 0 20px; }
  .form { display: flex; gap: 16px; align-items: end; flex-wrap: wrap; margin-bottom: 16px; }
  label { display: flex; flex-direction: column; gap: 4px; font-size: 12px; color: var(--muted); }
  .report { margin-bottom: 8px; }
  .err { color: var(--bad); }
</style>
