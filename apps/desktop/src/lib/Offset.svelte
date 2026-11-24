<script lang="ts">
  import { getMortgageOffset, type Invoke, type OffsetEffect } from "./api";
  import { formatCents, parseDollars } from "./money";

  let { invoke }: { invoke?: Invoke } = $props();
  let balance = $state("");
  let rate = $state("");
  let years = $state("");
  let offset = $state("");
  let result = $state<OffsetEffect | null>(null);
  let error = $state("");

  async function run() {
    error = "";
    result = null;
    const b = parseDollars(balance);
    const o = parseDollars(offset || "0");
    const r = parseFloat(rate);
    const y = parseInt(years, 10);
    if (b === null || b <= 0 || o === null || o < 0 || !Number.isFinite(r) || !Number.isFinite(y)) {
      error = "Enter what you owe, the yearly rate (like 4.5), the years left and the savings you would hold in the offset account.";
      return;
    }
    try {
      result = await getMortgageOffset(b, Math.round(r * 100), y, o, invoke);
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }
  const span = (m: number) => `${Math.floor(m / 12)} years ${m % 12} months`;
</script>

<section class="card" data-testid="offset">
  <h2>Mortgage offset account</h2>
  <p class="muted">Savings held in an offset account cut the interest on your loan while you keep the same payment.</p>
  <div class="row">
    <label>Owed $ <input aria-label="Mortgage owed" bind:value={balance} inputmode="decimal" /></label>
    <label>Rate % <input aria-label="Mortgage rate" bind:value={rate} inputmode="decimal" /></label>
    <label>Years left <input aria-label="Years left" bind:value={years} inputmode="numeric" /></label>
    <label>Offset savings $ <input aria-label="Offset savings" bind:value={offset} inputmode="decimal" /></label>
    <button onclick={run}>Compare</button>
  </div>
  {#if error}<p class="err" role="alert">{error}</p>{/if}
  {#if result}
    <p data-testid="offset-result">
      Paid off in <b>{span(result.monthsWith)}</b> instead of {span(result.monthsWithout)}, saving
      <span class="num">{formatCents(result.interestSavedCents, { whole: true })}</span> in interest.
    </p>
  {/if}
</section>

<style>
  section { margin-top: 16px; }
  .row { display: flex; gap: 12px; align-items: end; flex-wrap: wrap; }
  label { display: inline-flex; flex-direction: column; gap: 4px; font-size: 12px; color: var(--muted); }
  input { width: 100px; }
  .muted { color: var(--muted); }
  .err { color: var(--bad); }
</style>
