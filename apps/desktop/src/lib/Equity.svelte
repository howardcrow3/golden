<script lang="ts">
  import { getEsppPurchase, getRsuVest, type EsppResult, type Invoke, type VestResult } from "./api";
  import { formatCents, parseDollars } from "./money";

  let { invoke }: { invoke?: Invoke } = $props();

  let shares = $state("");
  let price = $state("");
  let fed = $state("22");
  let other = $state("12.65");
  let vest = $state<VestResult | null>(null);
  let vestError = $state("");

  let saved = $state("");
  let offering = $state("");
  let purchase = $state("");
  let discount = $state("15");
  let lookback = $state(true);
  let espp = $state<EsppResult | null>(null);
  let esppError = $state("");

  async function runVest() {
    vestError = "";
    vest = null;
    const n = parseInt(shares, 10);
    const p = parseDollars(price);
    const f = parseFloat(fed);
    const o = parseFloat(other);
    if (!Number.isFinite(n) || n < 0 || p === null || p <= 0 || !Number.isFinite(f) || !Number.isFinite(o)) {
      vestError = "Enter the shares vesting, the share price and your tax rates (like 22).";
      return;
    }
    try {
      vest = await getRsuVest(n, p, f / 100, o / 100, invoke);
    } catch (e) {
      vestError = e instanceof Error ? e.message : String(e);
    }
  }

  async function runEspp() {
    esppError = "";
    espp = null;
    const c = parseDollars(saved);
    const a = parseDollars(offering);
    const b = parseDollars(purchase);
    const d = parseFloat(discount);
    if (c === null || a === null || b === null || !Number.isFinite(d)) {
      esppError = "Enter what you saved, both share prices and the discount (like 15).";
      return;
    }
    try {
      espp = await getEsppPurchase(c, a, b, d / 100, lookback, invoke);
    } catch (e) {
      esppError = e instanceof Error ? e.message : String(e);
    }
  }
</script>

<section class="card" data-testid="equity">
  <h2>Stock pay: RSU vests and ESPP</h2>
  <p class="muted">Estimates only. Your employer's withholding and your own tax bill can differ.</p>

  <h3>RSU vest</h3>
  <div class="row">
    <label>Shares <input aria-label="Shares vesting" bind:value={shares} inputmode="numeric" /></label>
    <label>Price $ <input aria-label="Vest price" bind:value={price} inputmode="decimal" /></label>
    <label>Federal % <input aria-label="Federal rate" bind:value={fed} inputmode="decimal" /></label>
    <label>Other taxes % <input aria-label="Other tax rate" bind:value={other} inputmode="decimal" /></label>
    <button onclick={runVest}>Work it out</button>
  </div>
  {#if vestError}<p class="err" role="alert">{vestError}</p>{/if}
  {#if vest}
    <p data-testid="vest-result">
      The vest is worth <span class="num">{formatCents(vest.valueCents)}</span> and about
      <span class="num">{formatCents(vest.withheldCents)}</span> is withheld. Selling <b>{vest.sharesSold}</b> shares covers it, leaving
      <b>{vest.sharesKept}</b> shares{vest.cashBackCents > 0 ? ` and ${formatCents(vest.cashBackCents)} cash back` : ""}.
    </p>
  {/if}

  <h3>ESPP purchase</h3>
  <div class="row">
    <label>Saved $ <input aria-label="ESPP saved" bind:value={saved} inputmode="decimal" /></label>
    <label>Price at start $ <input aria-label="Offering price" bind:value={offering} inputmode="decimal" /></label>
    <label>Price at purchase $ <input aria-label="Purchase price" bind:value={purchase} inputmode="decimal" /></label>
    <label>Discount % <input aria-label="ESPP discount" bind:value={discount} inputmode="decimal" /></label>
    <label class="check"><input type="checkbox" aria-label="Lookback" bind:checked={lookback} /> Lookback</label>
    <button onclick={runEspp}>Work it out</button>
  </div>
  {#if esppError}<p class="err" role="alert">{esppError}</p>{/if}
  {#if espp}
    <p data-testid="espp-result">
      You pay <span class="num">{formatCents(espp.purchasePriceCents)}</span> a share and buy <b>{espp.shares}</b> shares
      (<span class="num">{formatCents(espp.leftoverCents)}</span> left over). The discount is worth
      <span class="num">{formatCents(espp.discountGainCents)}</span> the day you buy.
    </p>
  {/if}
</section>

<style>
  section { margin-top: 16px; }
  h3 { margin: 12px 0 4px; font-size: 14px; }
  .row { display: flex; gap: 12px; align-items: end; flex-wrap: wrap; }
  label { display: inline-flex; flex-direction: column; gap: 4px; font-size: 12px; color: var(--muted); }
  label.check { flex-direction: row; align-items: center; }
  input { width: 100px; }
  .muted { color: var(--muted); }
  .err { color: var(--bad); }
</style>
