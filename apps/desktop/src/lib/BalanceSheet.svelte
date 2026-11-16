<script lang="ts">
  import { getBalanceSheet, getNetWorthHistory, listAccounts, setOpeningBalance, type AccountRow, type BalanceSheet, type NetWorthPoint, type Invoke } from "./api";
  import { formatCents, parseDollars } from "./money";

  let { invoke }: { invoke?: Invoke } = $props();
  let data = $state<BalanceSheet | null>(null);
  let available = $state(false);

  let accounts = $state<AccountRow[]>([]);
  let history = $state<NetWorthPoint[]>([]);
  let editing = $state(false);
  let drafts = $state<Record<number, string>>({});
  let error = $state("");

  async function load() {
    try {
      data = await getBalanceSheet(invoke);
      accounts = await listAccounts(invoke);
      history = await getNetWorthHistory(12, invoke).catch(() => []);
      available = true;
    } catch {
      data = null;
      available = false;
    }
  }
  $effect(() => {
    load();
  });

  /** Points for a 300 x 50 trend line of month-end net worth. */
  export function trend(points: NetWorthPoint[]): string {
    const v = points.map((p) => p.netWorthCents);
    const lo = Math.min(...v);
    const span = Math.max(...v) - lo || 1;
    return v.map((x, i) => `${Math.round((i / (v.length - 1 || 1)) * 300)},${Math.round(48 - ((x - lo) / span) * 46)}`).join(" ");
  }

  function startEditing() {
    drafts = Object.fromEntries(accounts.map((a) => [a.id, (a.openingCents / 100).toFixed(2)]));
    error = "";
    editing = true;
  }

  async function save() {
    error = "";
    try {
      for (const a of accounts) {
        const cents = parseDollars(drafts[a.id] ?? "");
        if (cents === null) throw new Error(`Enter ${a.name}'s starting balance as a number, like 1250.00.`);
        if (cents !== a.openingCents) await setOpeningBalance(a.id, cents, invoke);
      }
      editing = false;
      await load();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }
</script>

{#if available && data && (data.assets.length || data.debts.length)}
  <section class="card" data-testid="balance-sheet">
    <h2>What you're worth</h2>
    <p class="big num" class:neg={data.netWorthCents < 0} data-testid="net-worth">{formatCents(data.netWorthCents, { whole: true })}</p>
    {#if history.length > 1}
      <svg viewBox="0 0 300 50" class="trend" role="img" aria-label={`Net worth by month, ${history[0].month} to ${history[history.length - 1].month}`} data-testid="net-worth-trend">
        <polyline points={trend(history)} fill="none" stroke="currentColor" stroke-width="2" />
      </svg>
    {/if}
    <div class="cols">
      <div>
        <h3>You have <span class="num">{formatCents(data.assetsCents, { whole: true })}</span></h3>
        {#each data.assets as l}
          <div class="row" data-testid="asset-row"><span>{l.name}</span><span class="num">{formatCents(l.cents, { whole: true })}</span></div>
        {/each}
      </div>
      <div>
        <h3>You owe <span class="num">{formatCents(data.debtsCents, { whole: true })}</span></h3>
        {#each data.debts as l}
          <div class="row" data-testid="debt-row"><span>{l.name}</span><span class="num">{formatCents(l.cents, { whole: true })}</span></div>
        {/each}
        {#if !data.debts.length}<p class="muted">Nothing owed.</p>{/if}
      </div>
    </div>
    {#if editing}
      <div data-testid="opening-editor">
        <p class="muted">What each account held before your first imported statement (use a negative number for a card balance you owed).</p>
        {#each accounts as a}
          <label class="row">{a.name} <input aria-label={`Starting balance for ${a.name}`} bind:value={drafts[a.id]} inputmode="decimal" /></label>
        {/each}
        {#if error}<p class="err" role="alert">{error}</p>{/if}
        <button onclick={save}>Save starting balances</button>
        <button onclick={() => (editing = false)}>Cancel</button>
      </div>
    {:else}
      <p class="muted">Based on activity you've imported plus any starting balances. <button class="link" onclick={startEditing}>Set starting balances</button></p>
    {/if}
  </section>
{/if}

<style>
  section { margin-top: 16px; }
  .big { font-size: 28px; margin: 0 0 8px; }
  .cols { display: grid; grid-template-columns: 1fr 1fr; gap: 16px; }
  h3 { font-size: 14px; margin: 0 0 6px; color: var(--muted); font-weight: normal; }
  .row { display: flex; justify-content: space-between; padding: 4px 0; border-bottom: 1px solid var(--line); }
  .muted { color: var(--muted); margin: 8px 0 0; }
  .neg { color: var(--bad); }
  .trend { width: 100%; max-width: 420px; height: 50px; color: var(--accent, #c9a227); margin: 4px 0 12px; }
  .err { color: var(--bad); }
  .link { background: none; border: none; padding: 0; color: inherit; text-decoration: underline; cursor: pointer; }
</style>
