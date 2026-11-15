<script lang="ts">
  import { quickPlan, type QuickPlanRequest, type QuickPlanResult } from "./api";
  import { linePoints, verdict } from "./chart";
  import { formatCents, formatPercent } from "./money";

  let age = $state(45);
  let retireAge = $state(62);
  let spending = $state(90_000);
  let salary = $state(180_000);
  let returnPick = $state<"" | "Cautious" | "Historical" | "Optimistic">("");
  let result = $state<QuickPlanResult | null>(null);
  let error = $state("");
  let busy = $state(false);

  function request(): QuickPlanRequest {
    return {
      currentYear: new Date().getFullYear(),
      age,
      retireAge,
      filing: "married_joint",
      state: "TX",
      salaryCents: salary * 100,
      contributionCents: 2_350_000,
      spendingCents: spending * 100,
      cashCents: 3_000_000,
      taxableCents: 15_000_000,
      traditionalCents: 60_000_000,
      rothCents: 12_000_000,
      ssAnnualCents: 3_800_000,
      picks: returnPick ? { real_return: returnPick } : {},
      runs: 400,
    };
  }

  async function run() {
    busy = true;
    error = "";
    try {
      result = await quickPlan(request());
    } catch (e) {
      result = null;
      error = e instanceof Error ? e.message : String(e);
    } finally {
      busy = false;
    }
  }

  let v = $derived(result ? verdict(result.successRate) : null);
  let points = $derived(result ? linePoints(result.years.map((y) => y.balanceCents), 640, 180) : "");
</script>

<h1>Retirement</h1>
<p class="sub">Sample household. Change a number or pick a what-if and run it again.</p>

<form class="card controls" onsubmit={(e) => { e.preventDefault(); run(); }}>
  <label>Age <input type="number" bind:value={age} min="18" max="90" aria-label="Age" /></label>
  <label>Retire at <input type="number" bind:value={retireAge} min="18" max="90" aria-label="Retire at" /></label>
  <label>Spending per year <input type="number" bind:value={spending} step="1000" aria-label="Spending per year" /></label>
  <label>Salary <input type="number" bind:value={salary} step="1000" aria-label="Salary" /></label>
  <label>Market what-if
    <select bind:value={returnPick} aria-label="Market what-if">
      <option value="">Likeliest (4% real)</option>
      <option value="Cautious">Cautious (2%)</option>
      <option value="Optimistic">Optimistic (6%)</option>
    </select>
  </label>
  <button type="submit" disabled={busy}>{busy ? "Running…" : "Run plan"}</button>
</form>

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

<style>
  .sub { color: var(--muted); margin: 0 0 20px; }
  .controls { display: flex; gap: 16px; flex-wrap: wrap; align-items: end; margin-bottom: 16px; }
  label { display: flex; flex-direction: column; gap: 4px; font-size: 12px; color: var(--muted); }
  input[type="number"] { width: 130px; }
  .big { font-size: 36px; font-weight: 600; }
  .big span { font-size: 14px; font-weight: 500; margin-left: 8px; }
  .good { color: var(--good); } .ok { color: var(--muted); } .bad { color: var(--bad); }
  .range, .assumptions { color: var(--muted); font-size: 13px; }
  svg { width: 100%; height: 180px; border: 1px solid var(--line); margin: 12px 0; background: var(--ivory); }
  .err { color: var(--bad); }
</style>
