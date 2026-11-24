<script lang="ts">
  import { backtest, backtestCompare, bootstrap, type BacktestResult, type MixRow, type BootstrapResult, type Invoke } from "./api";
  import { formatCents, formatPercent } from "./money";

  let { invoke, refresh = null }: { invoke?: Invoke; refresh?: unknown } = $props();
  let data = $state<BacktestResult | null>(null);
  let error = $state("");
  let busy = $state(false);
  let share = $state(0.6);
  let boot = $state<BootstrapResult | null>(null);
  let mixes = $state<MixRow[] | null>(null);

  $effect(() => {
    refresh;
    data = null;
    boot = null;
    mixes = null;
  });

  async function run() {
    busy = true;
    error = "";
    try {
      data = await backtest(share, invoke);
      boot = await bootstrap(share, 5, 1000, invoke);
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      busy = false;
    }
  }
  async function compare() {
    busy = true;
    error = "";
    try {
      mixes = await backtestCompare([0.2, 0.4, 0.6, 0.8, 1], invoke);
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      busy = false;
    }
  }
  const worstRows = $derived(data ? data.worst.map((y) => data!.windows.find((w) => w.startYear === y)!).filter(Boolean) : []);
  const worked = $derived(data ? data.windows.filter((w) => w.succeeded).length : 0);
</script>

<div class="bt" data-testid="backtest">
  <h3>Would it have worked in the past?</h3>
  <p class="hint">
    Replays your plan from every starting year since 1928, using what stocks, bonds and prices actually did (S&P 500 with dividends and 10-year Treasuries, after inflation).
    Source: Aswath Damodaran, NYU Stern.
  </p>
  <label>Mix
    <select bind:value={share} aria-label="Stock share">
      <option value={0.4}>40% stocks, 60% bonds</option>
      <option value={0.6}>60% stocks, 40% bonds</option>
      <option value={0.8}>80% stocks, 20% bonds</option>
      <option value={1}>All stocks</option>
    </select>
  </label>
  <button type="button" disabled={busy} onclick={run}>{busy ? "Working…" : "Replay history"}</button>
  <button type="button" disabled={busy} onclick={compare}>Compare mixes</button>
  {#if error}<p class="err" role="alert">{error}</p>{/if}
  {#if mixes}
    <table data-testid="bt-mixes">
      <thead><tr><th>Stocks</th><th>Worked</th><th>Typical left</th><th>Weakest start</th></tr></thead>
      <tbody>
        {#each mixes as m}
          <tr>
            <td>{Math.round(m.stockShare * 100)}%</td>
            <td>{formatPercent(m.successRate, 0)}</td>
            <td class="num">{formatCents(m.medianFinalCents, { whole: true })}</td>
            <td>{m.worstYear}: {formatCents(m.worstFinalCents, { whole: true })} left</td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
  {#if data}
    <p data-testid="bt-summary">
      It worked in <strong>{worked} of {data.windows.length}</strong> starting years ({formatPercent(data.successRate, 0)}), each run for {data.years} years.
    </p>
    <div class="strip" role="img" aria-label="Each starting year, green if the plan worked">
      {#each data.windows as w}
        <div class="cell" class:fail={!w.succeeded} title={`${w.startYear}: ${w.succeeded ? "worked, " + formatCents(w.finalCents, { whole: true }) + " left" : "ran out " + (w.depletedYear ?? "")}`}></div>
      {/each}
    </div>
    <p class="hint">{data.windows[0].startYear} to {data.windows[data.windows.length - 1].startYear}, by starting year.</p>
    {#if boot}
      <p data-testid="bt-boot">
        Reshuffling that history in {boot.blockYears}-year blocks, {boot.runs.toLocaleString()} times: it worked {formatPercent(boot.successRate, 0)} of the time
        (middle outcome {formatCents(boot.p50Cents, { whole: true })} left, range {formatCents(boot.p10Cents, { whole: true })} to {formatCents(boot.p90Cents, { whole: true })}).
      </p>
    {/if}
    <ul data-testid="bt-worst">
      {#each worstRows as w}
        <li>
          Starting in <strong>{w.startYear}</strong>: {w.succeeded ? `worked, ${formatCents(w.finalCents, { whole: true })} left` : `money ran out in ${w.depletedYear}`}
          ({formatPercent(w.annualisedReturn, 1)} a year after inflation)
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .bt { margin-top: 16px; }
  h3 { margin: 0 0 6px; font-size: 15px; }
  .hint { color: var(--muted); font-size: 12px; margin: 4px 0; }
  .strip { display: flex; height: 22px; border: 1px solid var(--line); }
  .cell { flex: 1; background: var(--good); }
  .cell.fail { background: var(--bad); }
  ul { margin: 6px 0 0; padding-left: 18px; font-size: 13px; }
  table { border-collapse: collapse; margin: 8px 0; font-size: 13px; }
  th, td { text-align: left; padding: 3px 12px 3px 0; }
  .err { color: var(--bad); }
</style>
