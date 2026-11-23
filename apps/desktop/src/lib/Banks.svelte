<script lang="ts">
  import {
    bankConnect,
    bankFinishLink,
    bankItems,
    bankSetPaused,
    bankSyncAll,
    bankUnlink,
    licenseStatus,
    onBankLink,
    type BankItem,
    type BankLinkReturn,
    type BankSyncReport,
    type Invoke,
  } from "./api";

  let { invoke, listen }: { invoke?: Invoke; listen?: Parameters<typeof onBankLink>[1] } = $props();

  let items = $state<BankItem[]>([]);
  let allowed = $state<boolean | null>(null);
  let busy = $state(false);
  let error = $state("");
  let note = $state("");
  let confirming = $state("");

  const STATUS: Record<string, string> = { active: "Syncing", paused: "Paused", login_required: "Sign in again" };

  function say(r: BankSyncReport): string {
    const parts = [`${r.added} new`, `${r.updated} updated`, `${r.removed} removed`];
    if (r.reconciled) parts.push(`${r.reconciled} replaced by statements`);
    if (r.skipped) parts.push(`${r.skipped} skipped`);
    return parts.join(", ");
  }

  async function load() {
    try {
      items = await bankItems(invoke);
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }

  async function run<T>(f: () => Promise<T>): Promise<T | undefined> {
    busy = true;
    error = "";
    note = "";
    try {
      return await f();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      busy = false;
    }
  }

  async function connect() {
    await run(async () => {
      await bankConnect(invoke);
      note = "Your browser opened your bank's sign-in. Come back here when you're done.";
    });
  }

  async function finish(r: BankLinkReturn) {
    const rep = await run(() => bankFinishLink(r.publicToken, r.institution, invoke));
    if (rep) note = `${r.institution} connected: ${say(rep)}.`;
    await load();
  }

  async function syncNow() {
    const out = await run(() => bankSyncAll(invoke));
    if (!out) return;
    const bad = out.filter((o) => o.error);
    if (bad.length) error = bad.map((o) => `${o.bank}: ${o.error}`).join(" ");
    const good = out.filter((o) => o.report);
    if (good.length) note = good.map((o) => `${o.bank}: ${say(o.report!)}`).join(". ") + ".";
    await load();
  }

  async function toggle(i: BankItem) {
    await run(() => bankSetPaused(i.itemId, i.status !== "paused", invoke));
    await load();
  }

  async function unlink(i: BankItem) {
    if (confirming !== i.itemId) {
      confirming = i.itemId;
      return;
    }
    confirming = "";
    const n = await run(() => bankUnlink(i.itemId, invoke));
    if (n !== undefined) note = `${i.institution} unlinked. ${n} synced lines removed; your statements are untouched.`;
    await load();
  }

  $effect(() => {
    licenseStatus(invoke)
      .then((s) => (allowed = s.plaidActive))
      .catch(() => (allowed = false));
    load();
    let stop = () => {};
    onBankLink((r) => void finish(r), listen).then((s) => (stop = s));
    return () => stop();
  });
</script>

<h1>Banks</h1>
<p class="muted">Optional. Linked banks bring in transactions automatically, so you can skip downloading statements. Statement files keep working, and when one covers a month, it replaces the bank's lines for it. Private mode stays the default: nothing is sent until you connect a bank.</p>

{#if error}<p class="err" role="alert">{error}</p>{/if}
{#if note}<p role="status" data-testid="bank-note">{note}</p>{/if}

{#if allowed === false}
  <p class="card" data-testid="bank-locked">Bank connections are part of the Plaid add-on. Add it from the Licence page to use them.</p>
{:else if allowed}
  <div class="row">
    <button type="button" onclick={connect} disabled={busy}>Connect a bank</button>
    <button type="button" onclick={syncNow} disabled={busy || items.length === 0}>Sync now</button>
  </div>
  <p class="muted">Connecting opens your bank's sign-in in your browser. Golden never sees your bank password. It keeps only a sign-in token, in your computer's keychain.</p>
  {#if items.length === 0}
    <p data-testid="bank-empty">No banks connected.</p>
  {:else}
    <ul class="list">
      {#each items as i (i.itemId)}
        <li class="card" data-testid="bank-item">
          <div class="head"><b>{i.institution}</b><span class="pill">{STATUS[i.status] ?? i.status}</span></div>
          {#if i.status === "login_required"}<p class="err">This bank needs you to sign in again. Unlink it and connect it again.</p>{/if}
          <div class="row">
            {#if i.status !== "login_required"}
              <button type="button" onclick={() => toggle(i)} disabled={busy}>{i.status === "paused" ? "Resume" : "Pause"}</button>
            {/if}
            <button type="button" onclick={() => unlink(i)} disabled={busy}>{confirming === i.itemId ? "Click again to unlink" : "Unlink"}</button>
          </div>
        </li>
      {/each}
    </ul>
  {/if}
{/if}

<style>
  .list { list-style: none; padding: 0; display: grid; gap: 12px; }
  .head { display: flex; justify-content: space-between; align-items: center; }
  .row { display: flex; gap: 8px; margin: 8px 0; }
  .muted { color: var(--muted); }
  .pill { font-size: 12px; padding: 2px 8px; border-radius: 10px; border: 1px solid var(--line); color: var(--muted); }
</style>
