<script lang="ts">
  import { activateLicense, DEFAULT_PRICING, getPricing, licenseStatus, type Invoke, type LicenseStatus, type Pricing } from "./api";
  import { formatCents } from "./money";

  let { invoke }: { invoke?: Invoke } = $props();
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
    return `Golden is ${core} once per major version. Bank connections are an optional ${plaid} ${per}.`;
  }

  $effect(() => {
    getPricing(invoke).then((p) => (pricing = p));
    licenseStatus(invoke).then((s) => (status = s)).catch(() => (status = null));
  });

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
</style>
