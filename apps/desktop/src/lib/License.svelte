<script lang="ts">
  import { activateLicense, licenseStatus, type Invoke, type LicenseStatus } from "./api";

  let { invoke }: { invoke?: Invoke } = $props();
  let status = $state<LicenseStatus | null>(null);
  let token = $state("");
  let error = $state("");
  let note = $state("");

  export function describe(s: LicenseStatus): string {
    if (s.state === "licensed") return s.plaidActive ? "Licensed, with bank connections." : "Licensed. Thank you.";
    if (s.state === "trial") return `Trial: ${s.daysLeft} ${s.daysLeft === 1 ? "day" : "days"} left.`;
    return "Trial ended. Your data is safe and exports still work.";
  }

  $effect(() => {
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
<p class="sub">Golden is $25 once per major version. Bank connections are an optional $20 a year.</p>

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
