<script lang="ts">
  import { getBalanceSheet, listAccounts, setOpeningBalance, type AccountRow, type BalanceSheet, type Invoke } from "./api";
  import { formatCents, parseDollars } from "./money";

  let { invoke }: { invoke?: Invoke } = $props();
  let data = $state<BalanceSheet | null>(null);
  let available = $state(false);

  let accounts = $state<AccountRow[]>([]);
  let editing = $state(false);
  let drafts = $state<Record<number, string>>({});
  let error = $state("");

  async function load() {
    try {
      data = await getBalanceSheet(invoke);
      accounts = await listAccounts(invoke);
      available = true;
    } catch {
      data = null;
      available = false;
    }
  }
  $effect(() => {
    load();
  });

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
  .err { color: var(--bad); }
  .link { background: none; border: none; padding: 0; color: inherit; text-decoration: underline; cursor: pointer; }
</style>
