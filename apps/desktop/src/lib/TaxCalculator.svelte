<script lang="ts">
  import { estimateTax, type TaxEstimate, type TaxRequest } from "./api";
  import { formatCents, formatPercent, parseDollars } from "./money";

  let wages = $state("150,000");
  let filing = $state<TaxRequest["filing"]>("married_joint");
  let usState = $state("TX");
  let result = $state<TaxEstimate | null>(null);
  let error = $state("");

  async function run() {
    error = "";
    const cents = parseDollars(wages);
    if (cents === null) {
      error = "Enter wages as a dollar amount.";
      return;
    }
    try {
      result = await estimateTax({ wagesCents: cents, filing, state: usState });
    } catch (e) {
      result = null;
      error = e instanceof Error ? e.message : String(e);
    }
  }
</script>

<h1>Tax estimate</h1>
<p class="sub">2026 federal and state income tax plus FICA on wages.</p>

<form class="card" onsubmit={(e) => { e.preventDefault(); run(); }}>
  <label>Wages <input bind:value={wages} aria-label="Wages" /></label>
  <label>Filing
    <select bind:value={filing} aria-label="Filing status">
      <option value="single">Single</option>
      <option value="married_joint">Married filing jointly</option>
      <option value="married_separate">Married filing separately</option>
      <option value="head_of_household">Head of household</option>
    </select>
  </label>
  <label>State <input bind:value={usState} maxlength="2" size="3" aria-label="State" /></label>
  <button type="submit">Estimate</button>
</form>

{#if error}<p class="err" role="alert">{error}</p>{/if}
{#if result}
  <section class="card out">
    <div><span>Federal income tax</span><b class="num">{formatCents(result.federalCents, { whole: true })}</b></div>
    <div><span>State income tax{result.stateIsEstimate ? " (estimate)" : ""}</span><b class="num">{formatCents(result.stateCents, { whole: true })}</b></div>
    <div><span>Social Security and Medicare</span><b class="num">{formatCents(result.ficaCents, { whole: true })}</b></div>
    <div class="total"><span>Total</span><b class="num">{formatCents(result.totalCents, { whole: true })} · {formatPercent(result.effectiveRate)} of wages</b></div>
  </section>
{/if}

<style>
  .sub { color: var(--muted); margin: 0 0 20px; }
  form { display: flex; gap: 16px; align-items: end; flex-wrap: wrap; margin-bottom: 16px; }
  label { display: flex; flex-direction: column; gap: 4px; font-size: 12px; color: var(--muted); }
  .out div { display: flex; justify-content: space-between; padding: 8px 0; border-bottom: 1px solid var(--line); }
  .out .total { border-bottom: 0; font-size: 18px; }
  .err { color: var(--bad); }
</style>
