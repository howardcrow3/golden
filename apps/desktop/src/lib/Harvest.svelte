<script lang="ts">
  import { harvestPlan, type HarvestPlan, type Invoke } from "./api";
  import { formatCents, parseDollars } from "./money";

  let { invoke }: { invoke?: Invoke } = $props();
  let filing = $state("single");
  let income = $state("30,000");
  let gains = $state("0");
  let size = $state("1");
  let out = $state<HarvestPlan | null>(null);
  let error = $state("");

  async function run() {
    error = "";
    out = null;
    const i = parseDollars(income);
    const g = parseDollars(gains);
    if (i === null || g === null) {
      error = "Enter income and gains as dollar amounts.";
      return;
    }
    try {
      out = await harvestPlan(filing, i, g, Math.round(Number(size) || 0), invoke);
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }
</script>

<div class="harvest" data-testid="harvest">
  <h3>Gain harvesting and the ACA cliff</h3>
  <p class="hint">
    Up to a limit, long-term gains are taxed at 0% federally. Selling and rebuying winners in a low-income year raises your cost basis for free.
    Before Medicare, higher income can also cost you marketplace health subsidies.
  </p>
  <div class="row">
    <label>Filing as
      <select bind:value={filing} aria-label="Harvest filing status">
        <option value="single">Single</option>
        <option value="married_joint">Married filing jointly</option>
        <option value="head_of_household">Head of household</option>
      </select>
    </label>
    <label>Income before deductions <input bind:value={income} aria-label="Harvest income" inputmode="numeric" /></label>
    <label>Gains already this year <input bind:value={gains} aria-label="Harvest gains" inputmode="numeric" /></label>
    <label>People on the plan <input bind:value={size} aria-label="Household size" size="3" /></label>
  </div>
  <button type="button" onclick={run}>Find my room</button>
  {#if error}<p class="err" role="alert">{error}</p>{/if}
  {#if out}
    <p data-testid="harvest-room">
      You could realize about <strong>{formatCents(out.zeroRateRoomCents, { whole: true })}</strong> of long-term gains at 0%,
      saving roughly {formatCents(out.taxSavedCents, { whole: true })} against the 15% rate.
    </p>
    <p data-testid="harvest-aca">
      {#if out.gainCrossesAcaCliff}
        Careful: that much gain would put you {formatCents(-out.acaRoomCents, { whole: true })} over the {formatCents(out.acaCliffCents, { whole: true })} marketplace subsidy limit. Harvest less, or only the part under it.
      {:else}
        Even with that gain you stay {formatCents(out.acaRoomCents, { whole: true })} under the {formatCents(out.acaCliffCents, { whole: true })} marketplace subsidy limit.
      {/if}
    </p>
    <p class="hint">2026 federal rules; the subsidy limit is 400% of the poverty line (48 states). State tax and wash-sale rules are not counted.</p>
  {/if}
</div>

<style>
  .harvest { margin-top: 16px; border-top: 1px solid var(--line, #ddd); padding-top: 12px; }
  h3 { margin: 0 0 4px; font-size: 15px; }
  .hint { color: var(--muted); font-size: 13px; margin: 0 0 8px; }
  .row { display: flex; gap: 12px; flex-wrap: wrap; }
  label { display: flex; flex-direction: column; gap: 4px; font-size: 12px; color: var(--muted); margin-bottom: 8px; max-width: 180px; }
  .err { color: var(--danger, #b3261e); }
</style>
