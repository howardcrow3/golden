<script lang="ts">
  import { applyPaystub, readPaystub, type Invoke, type PaystubSummary } from "./api";
  import { formatCents } from "./money";

  let { invoke }: { invoke?: Invoke } = $props();
  let text = $state("");
  let summary = $state<PaystubSummary | null>(null);
  let error = $state("");
  let done = $state("");

  async function read() {
    error = "";
    done = "";
    summary = null;
    try {
      summary = await readPaystub(text, invoke);
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }

  async function add() {
    error = "";
    try {
      const row = await applyPaystub(text, 0, invoke);
      done = `Added "${row.label}" at ${formatCents(row.annualCents, { whole: true })} a year to your plan.`;
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }

  const pct = (bp: number) => `${(bp / 100).toFixed(1)}%`;
</script>

<section class="card" data-testid="paystub">
  <h2>Read a paystub</h2>
  <p class="muted">Paste the text of a paystub. Golden shows what it found and can add the pay to your plan. Nothing leaves this computer.</p>
  <textarea rows="6" bind:value={text} aria-label="Paystub text" placeholder="Gross Pay 4,000.00&#10;Federal Income Tax 420.00&#10;..."></textarea>
  <button onclick={read} disabled={!text.trim()}>Read paystub</button>
  {#if error}<p class="err" role="alert">{error}</p>{/if}
  {#if summary}
    <div data-testid="paystub-summary">
      <p>
        <b>{summary.employer || "Paystub"}</b>{summary.payDate ? `, paid ${summary.payDate}` : ""}:
        <span class="num">{formatCents(summary.grossCents)}</span> gross,
        <span class="num">{formatCents(summary.netCents)}</span> take-home.
        Income tax withheld is {pct(summary.incomeTaxRateBp)} of gross.
      </p>
      {#if summary.periodsPerYear}
        <p class="muted">Paid {summary.frequency}: about <span class="num">{formatCents(summary.annualGrossCents, { whole: true })}</span> a year, with <span class="num">{formatCents(summary.annualRetirementCents, { whole: true })}</span> going to retirement.</p>
        <button onclick={add}>Add to my plan</button>
      {:else}
        <p class="muted">The pay period isn't on this stub, so I can't tell how often you're paid.</p>
      {/if}
    </div>
  {/if}
  {#if done}<p data-testid="paystub-done">{done}</p>{/if}
</section>

<style>
  section { margin-top: 16px; }
  textarea { width: 100%; box-sizing: border-box; margin: 8px 0; font-family: inherit; }
  .muted { color: var(--muted); margin: 0 0 8px; }
  .err { color: var(--bad); }
</style>
