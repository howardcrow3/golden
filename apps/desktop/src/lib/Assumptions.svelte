<script lang="ts">
  import { getAssumptions, setPick, type AssumptionRow, type Invoke } from "./api";
  import { assumptionName, optionText } from "./assumptions";

  let { invoke, onchange }: { invoke?: Invoke; onchange?: () => void } = $props();
  let rows = $state<AssumptionRow[]>([]);
  let error = $state("");

  async function load() {
    try {
      rows = await getAssumptions(invoke);
      error = "";
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }
  $effect(() => {
    load();
  });

  async function pick(key: string, label: string) {
    try {
      await setPick(key, label, invoke);
      await load();
      onchange?.();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }
</script>

<section class="card" aria-label="Assumptions">
  <h2>Assumptions</h2>
  <p class="sub">Each assumption has several possible values. Golden uses the likeliest unless you pick another.</p>
  {#if error}<p class="err" role="alert">{error}</p>{/if}
  {#each rows as r}
    {@const current = r.options.find((o) => o.label === r.active)}
    <div class="row">
      <label>{assumptionName(r.key)}
        <select aria-label={assumptionName(r.key)} value={r.picked ? r.active : ""} onchange={(e) => pick(r.key, e.currentTarget.value)}>
          <option value="">Likeliest</option>
          {#each r.options as o}<option value={o.label}>{optionText(o)}</option>{/each}
        </select>
      </label>
      <span class="src">{current ? `Using ${current.label}: ${current.source}` : ""}</span>
    </div>
  {/each}
</section>

<style>
  .sub { color: var(--muted); margin: 0 0 12px; font-size: 13px; }
  .row { display: flex; gap: 16px; align-items: end; padding: 8px 0; border-bottom: 1px solid var(--line); flex-wrap: wrap; }
  label { display: flex; flex-direction: column; gap: 4px; font-size: 12px; color: var(--muted); min-width: 280px; }
  .src { font-size: 12px; color: var(--muted); }
  .err { color: var(--bad); }
</style>
