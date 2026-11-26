<script lang="ts">
  import { vaultAdd, vaultDelete, vaultExport, vaultList, type Invoke, type VaultDoc } from "./api";
  import { formatCents } from "./money";

  let { invoke }: { invoke?: Invoke } = $props();
  let docs = $state<VaultDoc[]>([]);
  let query = $state("");
  let note = $state("");
  let error = $state("");
  let busy = $state(false);

  async function load() {
    try {
      docs = await vaultList(query, invoke);
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }
  $effect(() => {
    load();
  });

  function readBytes(f: File): Promise<Uint8Array> {
    return new Promise((resolve, reject) => {
      const r = new FileReader();
      r.onload = () => resolve(new Uint8Array(r.result as ArrayBuffer));
      r.onerror = () => reject(new Error("Couldn't read that file."));
      r.readAsArrayBuffer(f);
    });
  }

  async function onFiles(e: Event) {
    const input = e.currentTarget as HTMLInputElement;
    const files = Array.from(input.files ?? []);
    if (files.length === 0) return;
    busy = true;
    note = "";
    error = "";
    let added = 0;
    for (const f of files) {
      try {
        await vaultAdd(f.name, await readBytes(f), invoke);
        added += 1;
      } catch (err) {
        error = `${f.name}: ${err instanceof Error ? err.message : String(err)}`;
      }
    }
    if (added > 0) note = `Added ${added} ${added === 1 ? "document" : "documents"}.`;
    input.value = "";
    busy = false;
    await load();
  }

  async function act(fn: () => Promise<unknown>, done: (r: unknown) => string) {
    note = "";
    error = "";
    try {
      note = done(await fn());
      await load();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }

  const size = (n: number) => (n >= 1_048_576 ? `${(n / 1_048_576).toFixed(1)} MB` : `${Math.max(1, Math.round(n / 1024))} KB`);
</script>

<section class="docs" aria-label="Documents">
  <h2>Documents</h2>
  <p class="hint">
    Tax forms, statements and other papers, kept inside Golden's encrypted data on this computer. Golden reads what it can from each one here; nothing is uploaded.
    Photos and scans are read with your computer's own text recognition; if that fails the file is still kept.
  </p>
  <div class="row">
    <label>Add files <input type="file" multiple aria-label="Document files" onchange={onFiles} disabled={busy} /></label>
    <label>Search <input bind:value={query} oninput={load} aria-label="Search documents" placeholder="name, type or words inside" /></label>
  </div>
  {#if note}<p class="ok" role="status">{note}</p>{/if}
  {#if error}<p class="err" role="alert">{error}</p>{/if}
  {#if docs.length === 0}
    <p class="hint" data-testid="docs-empty">{query ? "Nothing matches." : "No documents yet."}</p>
  {:else}
    <ul>
      {#each docs as d (d.id)}
        <li data-testid="doc-row">
          <strong>{d.name}</strong>
          <span class="meta">{d.kind}{d.year ? ` ${d.year}` : ""} · {size(d.size)} · added {d.added}</span>
          {#if d.hasText === false}
            <p class="hint" data-testid="doc-unread">Couldn't read any text in this file. It's kept as is, but search won't find words inside it.</p>
          {/if}
          {#if d.fields.length > 0}
            <ul class="fields">
              {#each d.fields as f}<li>{f.label}: {formatCents(f.cents)}</li>{/each}
            </ul>
          {/if}
          <button type="button" onclick={() => act(() => vaultExport(d.id, invoke), (p) => `Copied to ${p}`)}>Save a copy</button>
          <button type="button" aria-label={`Delete ${d.name}`} onclick={() => act(() => vaultDelete(d.id, invoke), () => `Deleted ${d.name}.`)}>Delete</button>
        </li>
      {/each}
    </ul>
  {/if}
</section>

<style>
  .docs { padding: 16px; }
  h2 { margin: 0 0 6px; }
  .hint { color: var(--muted); font-size: 13px; }
  .row { display: flex; gap: 16px; flex-wrap: wrap; margin: 8px 0; }
  label { display: flex; flex-direction: column; gap: 4px; font-size: 12px; color: var(--muted); }
  ul { list-style: none; padding: 0; }
  li { padding: 8px 0; border-bottom: 1px solid var(--line); }
  .meta { color: var(--muted); font-size: 12px; margin-left: 8px; }
  .fields { font-size: 13px; margin: 4px 0; }
  .fields li { border: 0; padding: 0; }
  .err { color: var(--danger); }
</style>
