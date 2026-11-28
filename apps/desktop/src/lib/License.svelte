<script lang="ts">
  import { activateLicense, DEFAULT_PRICING, getPricing, licenseStatus, openCheckout, type Invoke, type LicenseStatus, type Pricing } from "./api";
  import { formatCents } from "./money";

  let { invoke, notice = "" }: { invoke?: Invoke; notice?: string } = $props();
  let status = $state<LicenseStatus | null>(null);
  let token = $state("");
  let error = $state("");
  let note = $state("");
  let pricing = $state<Pricing>(DEFAULT_PRICING);

  export function describe(s: LicenseStatus): string {
    if (s.state === "licensed") return s.plaidActive ? "Licensed, with bank connections." : "Licensed. Thank you.";
    if (s.state === "trial") return `Trial: ${s.daysLeft} ${s.daysLeft === 1 ? "day" : "days"} left.`;
    return "Trial ended. Your data is safe and exports still work.";
  }

  export function priceLine(p: Pricing): string {
    const core = formatCents(p.coreCents);
    const plaid = formatCents(p.plaidCents);
    const per = p.plaidInterval === "year" ? "a year" : "a month";
    const yearly = p.plaidYearlyCents > 0 ? `, or ${formatCents(p.plaidYearlyCents)} a year` : "";
    return `Golden is ${core} once per major version. Automatic bank updates are a separate add-on, not included in that price: ${plaid} ${per}${yearly}, on top of Golden.`;
  }

  $effect(() => {
    getPricing(invoke).then((p) => (pricing = p));
    licenseStatus(invoke).then((s) => (status = s)).catch(() => (status = null));
  });

  async function buy(plan: "core" | "plaid") {
    error = "";
    try {
      await openCheckout(plan, invoke);
      note = "Checkout opened in your browser. After you pay, open the link in your licence email and Golden activates itself. Or paste the key below.";
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }

  async function activate() {
    error = "";
    note = "";
    try {
      status = await activateLicense(token, invoke);
      token = "";
      note = "Licence accepted.";
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }
</script>

<h1>Licence</h1>
<p class="sub" data-testid="prices">{priceLine(pricing)}</p>
{#if pricing.warning}<p class="err" role="alert">{pricing.warning}</p>{/if}

{#if notice}<p class="note" role="status" data-testid="link-notice">{notice}</p>{/if}

<section class="card buy" data-testid="buy">
  <button type="button" disabled={!pricing.coreCheckoutUrl} onclick={() => buy("core")}>Buy Golden, {formatCents(pricing.coreCents)}</button>
  <button type="button" disabled={!pricing.plaidCheckoutUrl} onclick={() => buy("plaid")}>Add bank connections, {formatCents(pricing.plaidCents)} a {pricing.plaidInterval}</button>
  {#if !pricing.coreCheckoutUrl}<span class="hint">Purchasing opens soon.</span>{/if}
</section>

{#if status}<section class="card" data-testid="status">{describe(status)}</section>{/if}

<form class="card" onsubmit={(e) => { e.preventDefault(); activate(); }}>
  <label>Licence key <textarea bind:value={token} rows="3" aria-label="Licence key"></textarea></label>
  <button type="submit" disabled={!token.trim()}>Activate</button>
</form>
{#if error}<p class="err" role="alert">{error}</p>{/if}
{#if note}<p role="status">{note}</p>{/if}

<style>
  .sub { color: var(--muted); margin: 0 0 20px; }
  form { margin-top: 16px; display: flex; flex-direction: column; gap: 12px; align-items: start; }
  label { display: flex; flex-direction: column; gap: 4px; font-size: 12px; color: var(--muted); width: 100%; }
  textarea { font: inherit; width: 100%; border: 1px solid var(--line); background: var(--paper); padding: 8px; }
  .err { color: var(--bad); }
  .buy { display: flex; gap: 12px; align-items: center; flex-wrap: wrap; margin-bottom: 16px; }
  .hint { color: var(--muted); font-size: 12px; }
</style>
