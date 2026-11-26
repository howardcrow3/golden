<script lang="ts">
  import { localIso } from "./localDate";
  import { addJournalEntry, getProgress, removeJournalEntry, startProgress, stopProgress, type Invoke, type ProgressReport } from "./api";
  import { pairPoints } from "./chart";
  import { formatCents, parseDollars } from "./money";

  let { invoke }: { invoke?: Invoke } = $props();
  let report = $state<ProgressReport | null>(null);
  let loaded = $state(false);
  let error = $state("");
  const W = 560;
  const H = 120;

  async function load() {
    try {
      report = await getProgress(invoke);
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loaded = true;
    }
  }
  async function act(fn: () => Promise<void>) {
    error = "";
    try {
      await fn();
      await load();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }
  $effect(() => {
    load();
  });
  let jDate = $state(localIso());
  let jAmount = $state("");
  let jNote = $state("");
  async function addEntry() {
    const cents = parseDollars(jAmount);
    if (cents === null) {
      error = "Savings total: enter a dollar amount.";
      return;
    }
    await act(() => addJournalEntry(jDate, cents, jNote, invoke));
    if (!error) {
      jAmount = "";
      jNote = "";
    }
  }
  const lines = $derived(report ? pairPoints(report.points.map((p) => p.actualChangeCents), report.points.map((p) => p.projectedChangeCents), W, H) : ["", ""]);
</script>

{#if loaded}
  <section class="progress" aria-label="Progress against the plan" data-testid="progress">
    <h3>Progress against the plan</h3>
    {#if !report}
      <p class="hint">Start tracking to see, month by month, how your real net worth moves against what the plan projected from today. It uses the statements you import.</p>
      <button type="button" onclick={() => act(() => startProgress(invoke))}>Start tracking from today</button>
    {:else}
      <p data-testid="progress-summary">
        Since {report.started}, your net worth has {report.aheadCents >= 0 ? "beaten" : "trailed"} the plan's projected savings growth by
        <strong>{formatCents(Math.abs(report.aheadCents), { whole: true })}</strong>.
      </p>
      {#if report.points.length > 1}
        <svg viewBox={`0 0 ${W} ${H}`} role="img" aria-label="Change in net worth against the projected change">
          <polyline points={lines[1]} fill="none" stroke="var(--muted)" stroke-width="2" stroke-dasharray="4 3" />
          <polyline points={lines[0]} fill="none" stroke="var(--gold)" stroke-width="2" data-testid="actual-line" />
        </svg>
        <p class="hint">Gold is your actual net worth change, dashed is the plan. Net worth also counts a home and debts, so this compares change, not level.</p>
      {:else}
        <p class="hint">Come back next month: one more month of statements gives a line to compare.</p>
      {/if}
      <h4>Your own check-ins</h4>
      <p class="hint">Write down what your savings add up to on a day, for accounts the statements do not cover. Each one is compared with what the plan projected for that day.</p>
      <div class="jform">
        <label>Date <input type="date" bind:value={jDate} aria-label="Check-in date" /></label>
        <label>Savings total <input bind:value={jAmount} aria-label="Savings total" inputmode="numeric" /></label>
        <label>Note <input bind:value={jNote} aria-label="Check-in note" /></label>
        <button type="button" onclick={addEntry}>Add check-in</button>
      </div>
      {#if report.journal && report.journal.length > 0}
        <ul class="journal">
          {#each report.journal as j (j.date)}
            <li data-testid="journal-row">
              {j.date}: {formatCents(j.savingsCents, { whole: true })}, plan said {formatCents(j.projectedCents, { whole: true })}
              ({j.aheadCents >= 0 ? "ahead" : "behind"} by {formatCents(Math.abs(j.aheadCents), { whole: true })}){j.note ? ` – ${j.note}` : ""}
              <button type="button" aria-label={`Remove check-in ${j.date}`} onclick={() => act(() => removeJournalEntry(j.date, invoke))}>Remove</button>
            </li>
          {/each}
        </ul>
      {/if}
      <button type="button" onclick={() => act(() => stopProgress(invoke))}>Stop tracking</button>
      <button type="button" onclick={() => act(() => startProgress(invoke))}>Restart from today</button>
    {/if}
    {#if error}<p class="err" role="alert">{error}</p>{/if}
  </section>
{/if}

<style>
  .progress { margin-top: 16px; }
  h3 { margin: 0 0 6px; font-size: 15px; }
  svg { width: 100%; max-width: 640px; height: auto; display: block; margin: 6px 0; }
  .hint { color: var(--muted); font-size: 12px; margin: 4px 0; }
  .err { color: var(--bad); }
  h4 { margin: 12px 0 4px; font-size: 13px; }
  .jform { display: flex; gap: 8px; flex-wrap: wrap; align-items: end; }
  .jform label { display: flex; flex-direction: column; font-size: 12px; color: var(--muted); }
  .journal { font-size: 13px; padding-left: 18px; }
</style>
