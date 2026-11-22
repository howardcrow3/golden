<script lang="ts">
  import { getHoldings, investmentCheckup, saveHoldings, type AssetClass, type Checkup, type Holding, type Invoke, type RiskProfile } from "./api";
  import { formatCents, formatPercent, parseDollars } from "./money";

  let { invoke }: { invoke?: Invoke } = $props();
  type Row = { name: string; value: string; class: AssetClass; fee: string; single: boolean };
  let rows = $state<Row[]>([]);
  let risk = $state<RiskProfile>("moderate");
  let result = $state<Checkup | null>(null);
  let error = $state("");
  let saved = $state(false);
  let edited = false;

  $effect(() => {
    getHoldings(invoke)
      .then((h) => {
        if (edited) return;
        rows = (h ?? []).map((x) => ({
          name: x.name,
          value: formatCents(x.valueCents, { whole: true }).replace("$", ""),
          class: x.class,
          fee: String(x.expenseBp / 100),
          single: x.singleStock,
        }));
      })
      .catch(() => {});
  });

  function toHoldings(): Holding[] {
    return rows.map((r, i) => {
      const label = r.name.trim() || `Holding ${i + 1}`;
      const v = parseDollars(r.value);
      if (v === null) throw new Error(`${label}: enter a dollar value.`);
      const fee = Number(r.fee.trim() || "0");
      if (!Number.isFinite(fee) || fee < 0) throw new Error(`${label}: enter the yearly fee as a percentage.`);
      return { name: label, valueCents: v, class: r.class, expenseBp: Math.round(fee * 100), singleStock: r.single };
    });
  }

  async function run() {
    error = "";
    saved = false;
    result = null;
    try {
      await saveHoldings(toHoldings(), invoke);
      saved = true;
      result = await investmentCheckup(risk, invoke);
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }
  const names = { stock: "Stocks", bond: "Bonds", cash: "Cash", other: "Other" } as const;
</script>

<div class="inv" data-testid="investments">
  <h3>Investment check-up</h3>
  <p class="hint">List what you hold to compare your mix with a target for your risk profile, see what fees cost, and spot a holding that is too big. Saved on this computer only.</p>
  {#each rows as r, i}
    <div class="row" data-testid="holding-row">
      <label>Name <input bind:value={r.name} aria-label={`Holding ${i + 1} name`} /></label>
      <label>Value <input bind:value={r.value} aria-label={`Holding ${i + 1} value`} inputmode="numeric" /></label>
      <label>Type
        <select bind:value={r.class} aria-label={`Holding ${i + 1} type`}>
          <option value="stock">Stocks</option><option value="bond">Bonds</option><option value="cash">Cash</option><option value="other">Other</option>
        </select>
      </label>
      <label>Fee % a year <input bind:value={r.fee} aria-label={`Holding ${i + 1} fee`} size="4" /></label>
      <label class="check"><input type="checkbox" bind:checked={r.single} aria-label={`Holding ${i + 1} is one company`} /> One company</label>
      <button type="button" onclick={() => { edited = true; rows.splice(i, 1); }}>Remove</button>
    </div>
  {/each}
  <button type="button" onclick={() => { edited = true; rows.push({ name: "", value: "0", class: "stock", fee: "0", single: false }); }}>Add a holding</button>
  <label class="risk">Risk profile
    <select bind:value={risk} aria-label="Risk profile">
      <option value="conservative">Conservative</option><option value="moderate">Moderate</option><option value="aggressive">Aggressive</option>
    </select>
  </label>
  <button type="button" onclick={run}>Check my investments</button>
  {#if error}<p class="err" role="alert">{error}</p>{/if}
  {#if result}
    <p data-testid="inv-total">{formatCents(result.totalCents, { whole: true })} across {rows.length} holdings.</p>
    <table>
      <thead><tr><th>Type</th><th>You hold</th><th>Target</th></tr></thead>
      <tbody>
        {#each result.mix.filter((m) => m.share > 0 || m.target > 0) as m}
          <tr data-testid="inv-mix"><th scope="row">{names[m.class]}</th><td>{formatPercent(m.share, 0)}</td><td>{formatPercent(m.target, 0)}</td></tr>
        {/each}
      </tbody>
    </table>
    <p data-testid="inv-gap">
      {#if Math.abs(result.stockGap) < 0.05}Your stock share is close to the target.
      {:else if result.stockGap > 0}You hold {formatPercent(result.stockGap, 0)} more in stocks than the target, so more ups and downs.
      {:else}You hold {formatPercent(-result.stockGap, 0)} less in stocks than the target, so less growth.{/if}
    </p>
    <p data-testid="inv-fees">
      Fees average {formatPercent(result.weightedFee, 2)} a year, {formatCents(result.yearlyFeeCents, { whole: true })} now. Over 20 years at a 5% return that costs about {formatCents(result.feeCost20yCents, { whole: true })}.
    </p>
    {#if result.flags.length > 0}
      <ul>
        {#each result.flags as f}
          <li data-testid="inv-flag"><strong>{f.name}</strong> ({formatPercent(f.share, 0)}): {f.reason}</li>
        {/each}
      </ul>
    {/if}
    <p class="hint">A rule of thumb, not advice: the target is 40%, 60% or 80% stocks by profile, lower from age 60.</p>
  {/if}
</div>

<style>
  .inv { margin-top: 16px; border-top: 1px solid var(--line, #ddd); padding-top: 12px; }
  h3 { margin: 0 0 4px; font-size: 15px; }
  .hint { color: var(--muted); font-size: 13px; margin: 0 0 8px; }
  .row { display: flex; gap: 8px; flex-wrap: wrap; align-items: end; margin-bottom: 6px; }
  label { display: flex; flex-direction: column; gap: 4px; font-size: 12px; color: var(--muted); }
  .check { flex-direction: row; align-items: center; }
  .risk { margin: 8px 0; max-width: 200px; }
  table { border-collapse: collapse; margin: 8px 0; font-size: 14px; }
  th, td { text-align: left; padding: 3px 12px 3px 0; }
  .err { color: var(--danger, #b3261e); }
</style>
