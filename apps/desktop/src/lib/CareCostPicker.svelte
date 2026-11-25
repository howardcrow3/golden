<script lang="ts">
  import { getCareCost, type CareKind, type Invoke } from "./api";
  import { formatCents } from "./money";

  let { region, onuse, invoke }: { region: string; onuse: (dollars: string) => void; invoke?: Invoke } = $props();
  let kind = $state<CareKind>("nursing_home_private");
  let note = $state("");
  let error = $state("");

  const KINDS: [CareKind, string][] = [
    ["nursing_home_private", "Nursing home, private room"],
    ["nursing_home_semi_private", "Nursing home, shared room"],
    ["assisted_living", "Assisted living"],
    ["adult_day", "Adult day health care"],
    ["caregiver", "Caregiver at home (44 hours a week)"],
  ];

  async function use() {
    error = "";
    note = "";
    try {
      const c = await getCareCost(region, kind, invoke);
      onuse(String(Math.round(c.annualCents / 100)));
      note = c.national
        ? `Used the national median, ${formatCents(c.annualCents, { whole: true })} a year, because the survey has no figure for ${region.toUpperCase() || "that state"}.`
        : `Used the ${region.toUpperCase()} median, ${formatCents(c.annualCents, { whole: true })} a year.`;
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }
</script>

<div class="care" data-testid="care-picker">
  <label>Typical cost for
    <select bind:value={kind} aria-label="Kind of care">
      {#each KINDS as [k, label]}<option value={k}>{label}</option>{/each}
    </select>
  </label>
  <button type="button" onclick={use}>Use my state's median</button>
  <span class="hint">CareScout Cost of Care Survey 2025, in 2025 dollars.</span>
  {#if note}<p class="hint" data-testid="care-note">{note}</p>{/if}
  {#if error}<p class="err" role="alert">{error}</p>{/if}
</div>

<style>
  .care { display: flex; gap: 12px; align-items: end; flex-wrap: wrap; margin-top: 8px; }
  label { display: inline-flex; flex-direction: column; gap: 4px; font-size: 12px; color: var(--muted); }
  .hint { color: var(--muted); font-size: 12px; margin: 0; }
  .err { color: var(--bad); }
</style>
