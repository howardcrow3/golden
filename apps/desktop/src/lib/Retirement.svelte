<script lang="ts">
  import { getHousehold, runPlan, saveHousehold, type Household, type Invoke, type QuickPlanResult } from "./api";
  import { linePoints, verdict } from "./chart";
  import { formatCents, formatPercent, parseDollars } from "./money";
  import Assumptions from "./Assumptions.svelte";

  let { invoke }: { invoke?: Invoke } = $props();
  let h = $state<Household | null>(null);
  let dollars = $state({ salary: "", spending: "", cash: "", taxable: "", traditional: "", roth: "", ss: "", contribution: "" });
  let result = $state<QuickPlanResult | null>(null);
  let error = $state("");
  let busy = $state(false);

  const fields = [
    ["salary", "Salary", "salaryCents"],
    ["contribution", "Pre-tax 401(k) saving per year", "contributionCents"],
    ["spending", "Spending per year", "spendingCents"],
    ["cash", "Cash", "cashCents"],
    ["taxable", "Taxable investments", "taxableCents"],
    ["traditional", "Traditional 401(k)/IRA", "traditionalCents"],
    ["roth", "Roth", "rothCents"],
    ["ss", "Social Security per year", "ssAnnualCents"],
  ] as const;

  function fill(x: Household) {
    h = x;
    for (const [k, , c] of fields) dollars[k] = formatCents(x[c], { whole: true }).replace("$", "");
  }

  $effect(() => {
    getHousehold(invoke)
      .then(fill)
      .catch((e) => (error = e instanceof Error ? e.message : String(e)));
  });

  async function run() {
    if (!h) return;
    busy = true;
    error = "";
    try {
      const next: Household = { ...h };
      for (const [k, label, c] of fields) {
        const cents = parseDollars(dollars[k]);
        if (cents === null) throw new Error(`${label}: enter a dollar amount.`);
        next[c] = cents;
      }
      await saveHousehold(next, invoke);
      h = next;
      result = await runPlan(400, invoke);
    } catch (e) {
      result = null;
      error = e instanceof Error ? e.message : String(e);
    } finally {
      busy = false;
    }
  }

  async function rerun() {
    if (!result) return;
    try {
      result = await runPlan(400, invoke);
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }

  let v = $derived(result ? verdict(result.successRate) : null);
  let points = $derived(result ? linePoints(result.years.map((y) => y.balanceCents), 640, 180) : "");
</script>

<h1>Retirement</h1>
<p class="sub">Your household, saved on this computer. Change a number, then run the plan.</p>

{#if h}
<form class="card controls" onsubmit={(e) => { e.preventDefault(); run(); }}>
  <label>Birth date <input bind:value={h.birthDate} aria-label="Birth date" placeholder="YYYY-MM-DD" /></label>
  <label>Retire at <input type="number" bind:value={h.retireAge} min="18" max="90" aria-label="Retire at" /></label>
  <label>State <input bind:value={h.state} maxlength="2" size="3" aria-label="State" /></label>
  <label>Filing
    <select bind:value={h.filing} aria-label="Filing status">
      <option value="single">Single</option>
      <option value="married_joint">Married filing jointly</option>
      <option value="married_separate">Married filing separately</option>
      <option value="head_of_household">Head of household</option>
    </select>
  </label>
  {#each fields as [k, label]}
    <label>{label} <input bind:value={dollars[k]} aria-label={label} inputmode="numeric" /></label>
  {/each}
  <button type="submit" disabled={busy}>{busy ? "Running…" : "Save and run plan"}</button>
</form>
{/if}

{#if error}<p class="err" role="alert">{error}</p>{/if}
{#if result && v}
  <section class="card">
    <h2>Chance the money lasts to {result.horizonAge}</h2>
    <div class="big num" data-testid="success">{formatPercent(result.successRate, 0)} <span class={v.tone}>{v.label}</span></div>
    <p class="range num">
      Middle outcome {formatCents(result.p50Cents, { whole: true })} at the end; a rough decade {formatCents(result.p10Cents, { whole: true })},
      a good decade {formatCents(result.p90Cents, { whole: true })}.
      {#if result.depletedYear}On the average path the money runs out in {result.depletedYear}.{/if}
    </p>
    <svg viewBox="0 0 640 180" role="img" aria-label="Projected balance by year"><polyline fill="none" stroke="var(--gold)" stroke-width="2" {points} /></svg>
    <p class="assumptions">Assumes {formatPercent(result.realReturn)} real return, {formatPercent(result.volatility)} volatility, Social Security at {result.ssClaimAge}. These are estimates, not advice.</p>
  </section>
{/if}

<Assumptions {invoke} onchange={rerun} />

<style>
  .sub { color: var(--muted); margin: 0 0 20px; }
  .controls { display: flex; gap: 16px; flex-wrap: wrap; align-items: end; margin-bottom: 16px; }
  label { display: flex; flex-direction: column; gap: 4px; font-size: 12px; color: var(--muted); }
  input[type="number"] { width: 130px; }
  input { width: 150px; }
  section { margin-bottom: 16px; }
  .big { font-size: 36px; font-weight: 600; }
  .big span { font-size: 14px; font-weight: 500; margin-left: 8px; }
  .good { color: var(--good); } .ok { color: var(--muted); } .bad { color: var(--bad); }
  .range, .assumptions { color: var(--muted); font-size: 13px; }
  svg { width: 100%; height: 180px; border: 1px solid var(--line); margin: 12px 0; background: var(--ivory); }
  .err { color: var(--bad); }
</style>
