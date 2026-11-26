<script lang="ts">
  import { applySsaStatement, readSsaStatement, type Invoke, type SsaSummary } from "./api";
  import { formatCents } from "./money";

  let { invoke }: { invoke?: Invoke } = $props();
  let text = $state("");
  let summary = $state<SsaSummary | null>(null);
  let claim = $state("fra");
  let person = $state("1");
  let error = $state("");
  let done = $state("");

  async function read() {
    error = "";
    done = "";
    summary = null;
    try {
      summary = await readSsaStatement(text, invoke);
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }

  async function use() {
    error = "";
    done = "";
    const n = parseInt(person, 10);
    if (!Number.isFinite(n) || n < 1) {
      error = "Enter 1 for the first person in your plan, 2 for the second.";
      return;
    }
    try {
      const yearly = await applySsaStatement(text, n - 1, claim, invoke);
      done = `Social Security set to ${formatCents(yearly, { whole: true })} a year for person ${n}.`;
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }
</script>

<section class="tb-section" data-testid="ssa">
  <h2>Read a Social Security Statement</h2>
  <p class="muted">Paste the text of your statement from ssa.gov. Golden shows the monthly amounts it found and can use one in your plan. Nothing leaves this computer.</p>
  <textarea rows="6" bind:value={text} aria-label="Statement text" placeholder="Age 62: $1,742 a month&#10;Full retirement age (67 years): $2,490 a month&#10;Age 70: $3,086 a month"></textarea>
  <button onclick={read} disabled={!text.trim()}>Read statement</button>
  {#if error}<p class="err" role="alert">{error}</p>{/if}
  {#if summary}
    <div data-testid="ssa-summary">
      <ul>
        {#if summary.monthlyAt62Cents !== null}<li>At 62: <span class="num">{formatCents(summary.monthlyAt62Cents, { whole: true })}</span> a month</li>{/if}
        {#if summary.monthlyAtFullRetirementCents !== null}<li>At full retirement age{summary.fullRetirementAge ? ` (${summary.fullRetirementAge})` : ""}: <span class="num">{formatCents(summary.monthlyAtFullRetirementCents, { whole: true })}</span> a month</li>{/if}
        {#if summary.monthlyAt70Cents !== null}<li>At 70: <span class="num">{formatCents(summary.monthlyAt70Cents, { whole: true })}</span> a month</li>{/if}
      </ul>
      <div class="row">
        <label>Use the amount at
          <select aria-label="Claim age" bind:value={claim}>
            <option value="62">62</option>
            <option value="fra">Full retirement age</option>
            <option value="70">70</option>
          </select>
        </label>
        <label>For person <input aria-label="Person number" bind:value={person} inputmode="numeric" /></label>
        <button onclick={use}>Use in my plan</button>
      </div>
    </div>
  {/if}
  {#if done}<p data-testid="ssa-done">{done}</p>{/if}
</section>

<style>
  section { margin-top: 16px; }
  textarea { width: 100%; box-sizing: border-box; margin: 8px 0; font-family: inherit; }
  .muted { color: var(--muted); margin: 0 0 8px; }
  .row { display: flex; gap: 12px; align-items: end; flex-wrap: wrap; }
  label { display: inline-flex; flex-direction: column; gap: 4px; font-size: 12px; color: var(--muted); }
  input { width: 60px; }
  .err { color: var(--bad); }
</style>
