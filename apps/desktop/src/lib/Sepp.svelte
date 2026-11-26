<script lang="ts">
  import { sepp, type Invoke, type SeppRow } from "./api";
  import { formatCents } from "./money";

  let { invoke }: { invoke?: Invoke } = $props();
  let balance = $state(500000);
  let age = $state(50);
  let rate = $state(5);
  let rows = $state<SeppRow[] | null>(null);
  let error = $state("");

  const names = { required_minimum: "Required minimum", fixed_amortization: "Fixed amortization" } as const;

  async function run() {
    error = "";
    rows = null;
    try {
      rows = await sepp(Math.round(Number(balance) * 100), Number(age), Math.round(Number(rate) * 100), invoke);
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }
</script>

<div class="sepp" data-testid="sepp">
  <h3>Early IRA withdrawals (72(t))</h3>
  <p class="hint">
    Substantially equal payments let you take money from an IRA before 59½ without the 10% penalty. You must keep taking them for
    five years or until 59½, whichever is longer, and changing them early brings the penalty back on everything taken.
  </p>
  <div class="row">
    <label>IRA balance <input type="number" min="0" bind:value={balance} aria-label="IRA balance" /></label>
    <label>Your age <input type="number" min="40" max="70" bind:value={age} aria-label="Your age" /></label>
    <label>Interest rate % <input type="number" min="0.1" max="20" step="0.1" bind:value={rate} aria-label="Interest rate" /></label>
  </div>
  <button type="button" onclick={run}>Work out payments</button>
  {#if error}<p class="err" role="alert">{error}</p>{/if}
  {#if rows}
    <table>
      <thead><tr><th>Method</th><th>Each year</th><th>Same every year?</th><th>Earliest you can stop</th></tr></thead>
      <tbody>
        {#each rows as r}
          <tr data-testid="sepp-row">
            <th scope="row">{names[r.method]}</th>
            <td class="num">{formatCents(r.yearlyCents, { whole: true })}</td>
            <td>{r.level ? "Yes" : "No, recalculated yearly"}</td>
            <td>Age {r.earliestStopAge}</td>
          </tr>
        {/each}
      </tbody>
    </table>
    <p class="hint">The rate ceiling is the greater of 5% and 120% of the IRS mid-term rate. Estimates, not tax advice.</p>
  {/if}
</div>

<style>
  .sepp { margin-top: 16px; border-top: 1px solid var(--line); padding-top: 12px; }
  h3 { margin: 0 0 4px; font-size: 15px; }
  .hint { color: var(--muted); font-size: 13px; margin: 0 0 8px; }
  .row { display: flex; gap: 12px; flex-wrap: wrap; }
  label { display: flex; flex-direction: column; gap: 4px; font-size: 12px; color: var(--muted); margin-bottom: 8px; max-width: 160px; }
  table { border-collapse: collapse; width: 100%; margin-top: 10px; font-size: 14px; }
  th, td { text-align: left; padding: 4px 8px; }
  .num { text-align: right; }
  .err { color: var(--danger); }
</style>
