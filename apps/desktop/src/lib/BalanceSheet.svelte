<script lang="ts">
  import { getBalanceSheet, type BalanceSheet, type Invoke } from "./api";
  import { formatCents } from "./money";

  let { invoke }: { invoke?: Invoke } = $props();
  let data = $state<BalanceSheet | null>(null);
  let available = $state(false);

  $effect(() => {
    getBalanceSheet(invoke)
      .then((s) => {
        data = s;
        available = true;
      })
      .catch(() => {
        data = null;
        available = false;
      });
  });
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
    <p class="muted">Based on activity you've imported; add opening balances for the full picture.</p>
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
</style>
