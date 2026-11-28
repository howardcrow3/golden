<script lang="ts">
  import { linkReceipt, matchReceipts, unlinkReceipt, type Invoke, type ReceiptItem } from "./api";
  import { formatCents } from "./money";

  let { invoke, version = 0 }: { invoke?: Invoke; version?: number } = $props();
  let items = $state<ReceiptItem[]>([]);
  let auto = $state(0);
  let error = $state("");

  async function load() {
    try {
      const r = await matchReceipts(invoke);
      items = r?.items ?? [];
      auto = r?.autoLinked ?? 0;
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }
  $effect(() => {
    version;
    load();
  });

  async function act(fn: () => Promise<unknown>) {
    error = "";
    try {
      await fn();
      await load();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }

  const label: Record<string, string> = {
    linked: "Matched",
    suggested: "Needs a look",
    duplicate: "Duplicate",
    unmatched: "Waiting",
  };
</script>

{#if items.length > 0}
  <section class="receipts" aria-label="Receipts">
    <h3>Receipts</h3>
    <p class="hint">
      A receipt is matched to the charge that paid for it and never adds an expense of its own, so nothing is counted twice, whether the receipt or the statement arrives first.
    </p>
    {#if auto > 0}<p class="ok" role="status">Matched {auto} new {auto === 1 ? "receipt" : "receipts"}.</p>{/if}
    {#if error}<p class="err" role="alert">{error}</p>{/if}
    <ul>
      {#each items as it (it.documentId)}
        <li data-testid="receipt-row">
          <strong>{it.merchant || it.name}</strong>
          <span class="meta">
            {it.totalCents !== null ? formatCents(it.totalCents) : "no total found"}{it.day ? ` · ${it.day}` : ""}
          </span>
          <span class="badge {it.status}">{label[it.status] ?? it.status}</span>
          {#if it.status === "linked" && it.linked}
            <p class="hint">Paid by {it.linked.payee} on {it.linked.day} ({formatCents(it.linked.cents)}).</p>
            <button type="button" onclick={() => act(() => unlinkReceipt(it.documentId, invoke))}>Not this charge</button>
          {:else if it.status === "suggested"}
            <p class="hint">More than one charge could be this receipt. Pick the right one:</p>
            {#each it.candidates as c (c.id)}
              <div class="cand">
                {c.payee} · {c.day} · {formatCents(c.cents)}
                <button type="button" onclick={() => act(() => linkReceipt(it.documentId, c.id, invoke))}>This one</button>
              </div>
            {/each}
          {:else if it.status === "duplicate"}
            <p class="hint">The same receipt is already matched to this purchase; this copy isn't counted again.</p>
          {:else}
            <p class="hint">No matching charge yet. It will match when the statement arrives.</p>
          {/if}
        </li>
      {/each}
    </ul>
  </section>
{/if}

<style>
  .receipts { margin-top: 16px; }
  h3 { margin: 0 0 6px; }
  .hint { color: var(--muted); font-size: 13px; margin: 4px 0; }
  ul { list-style: none; padding: 0; }
  li { padding: 8px 0; border-bottom: 1px solid var(--line); }
  .meta { color: var(--muted); font-size: 12px; margin: 0 8px; }
  .badge { font-size: 11px; padding: 1px 6px; border-radius: 8px; border: 1px solid var(--line); }
  .badge.linked { color: var(--ok, #2a7); }
  .badge.suggested { color: var(--warn, #b70); }
  .cand { font-size: 13px; padding: 2px 0; }
  .err { color: var(--danger); }
</style>
