<script lang="ts">
  import { rentVsBuy, type Invoke, type RentBuyOutcome } from "./api";
  import { formatCents, parseDollars } from "./money";

  let { invoke }: { invoke?: Invoke } = $props();
  let price = $state("500,000");
  let down = $state("100,000");
  let rate = $state("6.5");
  let mortgageYears = $state("30");
  let carrying = $state("1.5");
  let upkeep = $state("1");
  let appreciation = $state("3.5");
  let selling = $state("6");
  let rent = $state("2,500");
  let rentGrowth = $state("3");
  let invest = $state("6");
  let years = $state("20");
  let out = $state<RentBuyOutcome | null>(null);
  let error = $state("");

  const bp = (t: string, label: string) => {
    const n = Number(t.trim());
    if (!Number.isFinite(n)) throw new Error(`${label}: enter a percentage.`);
    return Math.round(n * 100);
  };
  const cents = (t: string, label: string) => {
    const c = parseDollars(t);
    if (c === null) throw new Error(`${label}: enter a dollar amount.`);
    return c;
  };

  async function run() {
    error = "";
    out = null;
    try {
      out = await rentVsBuy(
        {
          priceCents: cents(price, "Price"),
          downPaymentCents: cents(down, "Down payment"),
          mortgageRateBp: bp(rate, "Mortgage rate"),
          mortgageYears: Math.round(Number(mortgageYears) || 0),
          carryingBp: bp(carrying, "Property tax and insurance"),
          upkeepBp: bp(upkeep, "Upkeep"),
          appreciationBp: bp(appreciation, "Home price growth"),
          sellingCostBp: bp(selling, "Selling costs"),
          monthlyRentCents: cents(rent, "Monthly rent"),
          rentGrowthBp: bp(rentGrowth, "Rent growth"),
          investReturnBp: bp(invest, "Investment return"),
          years: Math.round(Number(years) || 0),
        },
        invoke,
      );
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }
</script>

<div class="rb" data-testid="rent-buy">
  <h3>Rent or buy?</h3>
  <p class="hint">Compares what you would have after some years: buy with a mortgage, or rent and invest the down payment and the monthly difference. Taxes on the sale and the mortgage interest deduction are left out.</p>
  <div class="grid">
    <label>Price <input bind:value={price} aria-label="Price" inputmode="numeric" /></label>
    <label>Down payment <input bind:value={down} aria-label="Down payment" inputmode="numeric" /></label>
    <label>Mortgage rate % <input bind:value={rate} aria-label="Mortgage rate" /></label>
    <label>Mortgage years <input bind:value={mortgageYears} aria-label="Mortgage years" /></label>
    <label>Tax and insurance % <input bind:value={carrying} aria-label="Property tax and insurance" /></label>
    <label>Upkeep % <input bind:value={upkeep} aria-label="Upkeep" /></label>
    <label>Home price growth % <input bind:value={appreciation} aria-label="Home price growth" /></label>
    <label>Selling costs % <input bind:value={selling} aria-label="Selling costs" /></label>
    <label>Rent per month <input bind:value={rent} aria-label="Monthly rent" inputmode="numeric" /></label>
    <label>Rent growth % <input bind:value={rentGrowth} aria-label="Rent growth" /></label>
    <label>Investments earn % <input bind:value={invest} aria-label="Investment return" /></label>
    <label>Compare after (years) <input bind:value={years} aria-label="Years to compare" /></label>
  </div>
  <button type="button" onclick={run}>Compare</button>
  {#if error}<p class="err" role="alert">{error}</p>{/if}
  {#if out}
    <p data-testid="rb-answer">
      After {out.years} years, {out.buyAdvantageCents >= 0 ? "buying" : "renting"} leaves you ahead by
      <strong>{formatCents(Math.abs(out.buyAdvantageCents), { whole: true })}</strong>
      (buy {formatCents(out.buyNetWorthCents, { whole: true })}, rent {formatCents(out.rentNetWorthCents, { whole: true })}).
      The mortgage payment is {formatCents(out.monthlyPaymentCents, { whole: true })} a month.
      {#if out.breakEvenYear !== null}Buying pulls ahead after {out.breakEvenYear} years.{:else}Buying does not pull ahead within this time.{/if}
    </p>
  {/if}
</div>

<style>
  .rb { margin-top: 16px; border-top: 1px solid var(--line, #ddd); padding-top: 12px; }
  h3 { margin: 0 0 4px; font-size: 15px; }
  .hint { color: var(--muted); font-size: 13px; margin: 0 0 8px; }
  .grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(150px, 1fr)); gap: 8px; margin-bottom: 8px; }
  label { display: flex; flex-direction: column; gap: 4px; font-size: 12px; color: var(--muted); }
  .err { color: var(--danger, #b3261e); }
</style>
