<script lang="ts">
  import { getForecast, type Forecast, type Invoke } from "./api";
  import { formatCents } from "./money";

  let { invoke, months = 12 }: { invoke?: Invoke; months?: number } = $props();
  let data = $state<Forecast | null>(null);
  let message = $state("");
  let available = $state(false);

  $effect(() => {
    getForecast(months, invoke)
      .then((f) => {
        data = f;
        available = true;
        message = "";
      })
      .catch((e) => {
        data = null;
        // In a plain browser there is no app to ask; inside the app a refusal is explained.
        const text = e instanceof Error ? e.message : String(e);
        available = !/lives in the Golden desktop app/.test(text);
        message = available ? text : "";
      });
  });

  /** SVG points for the balance line, scaled to a 300 x 60 box. */
  export function linePoints(f: Forecast): string {
    const values = [f.startBalanceCents, ...f.months.map((m) => m.balanceCents)];
    const lo = Math.min(...values);
    const hi = Math.max(...values);
    const span = hi - lo || 1;
    return values
      .map((v, i) => `${Math.round((i / (values.length - 1 || 1)) * 300)},${Math.round(58 - ((v - lo) / span) * 56)}`)
      .join(" ");
  }
</script>

{#if available}
  <section class="tb-section" data-testid="forecast">
    <h2>Where you're headed</h2>
    {#if data}
      <p class="muted">
        From your last {data.historyMonths} {data.historyMonths === 1 ? "month" : "months"} of statements{data.seasonalIncome ? ", with income placed on the months it usually arrives" : ""}. Position today: <b class="num">{formatCents(data.startBalanceCents, { whole: true })}</b>.
      </p>
      <svg viewBox="0 0 300 60" class="line" role="img" aria-label="Projected position by month">
        <polyline points={linePoints(data)} fill="none" stroke="currentColor" stroke-width="2" />
      </svg>
      <table>
        <thead><tr><th>Month</th><th class="r">Income</th><th class="r">Spending</th><th class="r">Net</th><th class="r">Position</th></tr></thead>
        <tbody>
          {#each data.months as m}
            <tr data-testid="forecast-row">
              <td>{m.month}</td>
              <td class="r num">{formatCents(m.incomeCents, { whole: true })}</td>
              <td class="r num">{formatCents(m.spendingCents, { whole: true })}</td>
              <td class="r num" class:neg={m.netCents < 0}>{formatCents(m.netCents, { whole: true })}</td>
              <td class="r num" class:neg={m.balanceCents < 0}>{formatCents(m.balanceCents, { whole: true })}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    {:else if message}
      <p class="muted" data-testid="forecast-note">{message}</p>
    {/if}
  </section>
{/if}

<style>
  section { margin-top: 16px; }
  .muted { color: var(--muted); margin: 0 0 8px; }
  .line { width: 100%; max-width: 480px; height: 60px; color: var(--accent); margin: 4px 0 8px; }
  table { width: 100%; border-collapse: collapse; }
  th, td { padding: 6px 0; border-bottom: 1px solid var(--line); text-align: left; font-weight: normal; }
  th { color: var(--muted); font-size: 12px; }
  .r { text-align: right; }
  .neg { color: var(--bad); }
</style>
