<script lang="ts">
  import Explain from "./Explain.svelte";
  import { getHousehold, runPlan, saveHousehold, type Household, type Invoke, type QuickPlanResult } from "./api";
  import { linePoints, verdict } from "./chart";
  import { formatCents, formatPercent } from "./money";
  import { fromForm, newPerson, newStream, removePerson, toForm, type Form } from "./household";
  import Assumptions from "./Assumptions.svelte";
  import Allocate from "./Allocate.svelte";
  import SafeSpend from "./SafeSpend.svelte";
  import RothExplorer from "./RothExplorer.svelte";
  import SsExplorer from "./SsExplorer.svelte";
  import PlanReport from "./PlanReport.svelte";
  import PlannerScore from "./PlannerScore.svelte";
  import FanChart from "./FanChart.svelte";
  import TaxInsights from "./TaxInsights.svelte";

  let { invoke }: { invoke?: Invoke } = $props();
  let f = $state<Form | null>(null);
  let result = $state<QuickPlanResult | null>(null);
  let error = $state("");
  let busy = $state(false);
  let trials = $state(400);
  const thisYear = new Date().getFullYear();

  $effect(() => {
    getHousehold(invoke)
      .then((h: Household) => (f = toForm(h)))
      .catch((e) => (error = e instanceof Error ? e.message : String(e)));
  });

  async function run() {
    if (!f) return;
    busy = true;
    error = "";
    try {
      await saveHousehold(fromForm(f), invoke);
      result = await runPlan(trials, invoke);
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
      result = await runPlan(trials, invoke);
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }

  let scoreHousehold = $derived.by(() => {
    if (!f) return null;
    try {
      return fromForm(f);
    } catch {
      return null;
    }
  });
  let v = $derived(result ? verdict(result.successRate) : null);
  let points = $derived(result ? linePoints(result.years.map((y) => y.balanceCents), 640, 180) : "");
</script>

<h1>Retirement</h1>
<p class="sub">Your household, saved on this computer. Change a number, then run the plan.</p>

{#if f}
  {#if scoreHousehold}<PlannerScore household={scoreHousehold} />{/if}
<form onsubmit={(e) => { e.preventDefault(); run(); }}>
  <section class="card" aria-label="People">
    <h2>People</h2>
    {#each f.people as p, i}
      <div class="controls">
        <label>Name <input bind:value={p.name} aria-label={`Name ${i + 1}`} /></label>
        <label>Birth date <input bind:value={p.birthDate} aria-label={`Birth date ${i + 1}`} placeholder="YYYY-MM-DD" /></label>
        <label>Retire at <input type="number" bind:value={p.retireAge} min="18" max="100" aria-label={`Retire at ${i + 1}`} /></label>
        <label>Social Security per year at full retirement age <input bind:value={p.ss} aria-label={`Social Security ${i + 1}`} inputmode="numeric" /></label>
        {#if i > 0}<button type="button" onclick={() => (f = removePerson(f!, i))} aria-label={`Remove ${p.name || `person ${i + 1}`}`}>Remove</button>{/if}
      </div>
    {/each}
    {#if f.people.length < 4}<button type="button" onclick={() => f!.people.push(newPerson())}>Add person</button>{/if}
  </section>

  <section class="card" aria-label="Income">
    <h2>Income</h2>
    <p class="hint">Leave the years empty to earn until that person retires. For a bonus or a stock vest, use "Add one-time income".</p>
    {#each f.streams as s, i}
      <div class="controls">
        <label>What <input bind:value={s.label} aria-label={`Income name ${i + 1}`} /></label>
        <label>Who
          <select bind:value={s.person} aria-label={`Income owner ${i + 1}`}>
            {#each f.people as p, pi}<option value={pi}>{p.name || `Person ${pi + 1}`}</option>{/each}
          </select>
        </label>
        <label>Per year <input bind:value={s.amount} aria-label={`Income amount ${i + 1}`} inputmode="numeric" /></label>
        <label>First year <input bind:value={s.start} aria-label={`First year ${i + 1}`} size="5" placeholder="now" /></label>
        <label>Last year <input bind:value={s.end} aria-label={`Last year ${i + 1}`} size="5" placeholder="retirement" /></label>
        <label>401(k) saving <input bind:value={s.contribution} aria-label={`401(k) saving ${i + 1}`} inputmode="numeric" /></label>
        <button type="button" onclick={() => f!.streams.splice(i, 1)} aria-label={`Remove income ${i + 1}`}>Remove</button>
      </div>
    {/each}
    <button type="button" onclick={() => f!.streams.push(newStream(false, thisYear))}>Add income</button>
    <button type="button" onclick={() => f!.streams.push(newStream(true, thisYear))}>Add one-time income</button>
  </section>

  <section class="card controls" aria-label="Household">
    <label>State <input bind:value={f.state} maxlength="2" size="3" aria-label="State" /></label>
    <label>Filing
      <select bind:value={f.filing} aria-label="Filing status">
        <option value="single">Single</option>
        <option value="married_joint">Married filing jointly</option>
        <option value="married_separate">Married filing separately</option>
        <option value="head_of_household">Head of household</option>
      </select>
    </label>
    <label>Spending per year <input bind:value={f.spending} aria-label="Spending per year" inputmode="numeric" /></label>
    <label>Cash <input bind:value={f.cash} aria-label="Cash" inputmode="numeric" /></label>
    <label>Taxable investments <input bind:value={f.taxable} aria-label="Taxable investments" inputmode="numeric" /></label>
    <label>Traditional 401(k)/IRA <input bind:value={f.traditional} aria-label="Traditional 401(k)/IRA" inputmode="numeric" /></label>
    <label>Roth <input bind:value={f.roth} aria-label="Roth" inputmode="numeric" /></label>
    <label>Spending in retirement
      <select bind:value={f.spendingRule} aria-label="Spending rule">
        <option value="fixed">The same amount every year</option>
        <option value="guardrails">Guardrails: trim after bad years, raise after good ones</option>
        <option value="percent">4% of the portfolio each year</option>
        <option value="vpw">Variable: spend down evenly to age 95</option>
      </select>
    </label>
    <label>Draw accounts in this order
      <select bind:value={f.withdrawalOrder} aria-label="Withdrawal order">
        <option value="standard">Cash, taxable, traditional, then Roth</option>
        <option value="traditional_first">Cash, traditional, taxable, then Roth</option>
        <option value="roth_first">Cash, Roth, taxable, then traditional</option>
        <option value="blended">Cash, then all three in proportion</option>
      </select>
    </label>
    <label class="check">
      <input type="checkbox" bind:checked={f.medicare} aria-label="Include Medicare premiums" />
      Include Medicare premiums from 65, with the income surcharge (IRMAA)
    </label>
    <button type="submit" disabled={busy}>{busy ? "Running…" : "Save and run plan"}</button>
  </section>
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
    <FanChart {result} />
    <label class="trials">Simulated futures
      <select bind:value={trials} onchange={rerun} aria-label="Simulated futures">
        <option value={200}>200 (fast)</option>
        <option value={400}>400</option>
        <option value={1000}>1,000</option>
        <option value={2500}>2,500</option>
        <option value={5000}>5,000 (slowest)</option>
      </select>
    </label>
    <Explain {invoke} facts={{ successRate: result.successRate, depletedYear: result.depletedYear, retireAge: null, horizonAge: result.horizonAge, compare: null }} />
    <SafeSpend {invoke} refresh={result} />
    <SsExplorer {invoke} refresh={result} />
    <RothExplorer {invoke} refresh={result} />
    <TaxInsights {result} household={scoreHousehold} />
    <PlanReport {invoke} {result} />
    <p class="assumptions">Assumes {formatPercent(result.realReturn)} real return, {formatPercent(result.volatility)} volatility, Social Security at {result.ssClaimAge}. These are estimates, not advice.</p>
  </section>
{/if}

<Assumptions {invoke} onchange={rerun} />
<Allocate {invoke} />

<style>
  .sub { color: var(--muted); margin: 0 0 20px; }
  .controls { display: flex; gap: 16px; flex-wrap: wrap; align-items: end; margin-bottom: 12px; }
  .hint { color: var(--muted); font-size: 13px; margin: 0 0 12px; }
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
