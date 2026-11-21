<script lang="ts">
  import { closeMonth, monthCloseCheck, reopenMonth, type CloseReport, type Invoke } from "./api";

  let { invoke, month }: { invoke?: Invoke; month?: string } = $props();

  function lastFullMonth(): string {
    const d = new Date();
    d.setDate(1);
    d.setMonth(d.getMonth() - 1);
    return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}`;
  }

  let report = $state<CloseReport | null>(null);
  let error = $state("");
  let busy = $state(false);

  $effect(() => {
    monthCloseCheck(month ?? lastFullMonth(), invoke)
      .then((r) => (report = r.count > 0 || r.closed ? r : null))
      .catch(() => (report = null));
  });

  async function act(f: typeof closeMonth) {
    if (!report) return;
    busy = true;
    error = "";
    try {
      report = await f(report.month, invoke);
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      busy = false;
    }
  }
</script>

{#if report}
  <section class="card" data-testid="month-close">
    <h2>Month close <span class="badge">{report.closed ? "Closed" : "Open"}</span></h2>
    <p>{report.summary}</p>
    {#if report.issues.length}
      <ul>
        {#each report.issues as i}<li data-kind={i.kind}>{i.text}</li>{/each}
      </ul>
    {:else}
      <p class="ok">Nothing to check: every line has a category and no account went quiet.</p>
    {/if}
    {#if error}<p class="err" role="alert">{error}</p>{/if}
    {#if report.closed}
      <button type="button" disabled={busy} onclick={() => act(reopenMonth)}>Reopen this month</button>
    {:else if report.finished}
      <button type="button" disabled={busy} onclick={() => act(closeMonth)}>Close this month</button>
    {/if}
  </section>
{/if}

<style>
  section { margin-top: 16px; }
  .badge { font-size: 0.75rem; color: var(--muted); margin-left: 8px; }
  ul { margin: 0 0 12px; padding-left: 18px; }
  li { margin: 6px 0; }
  .ok { color: var(--muted); }
</style>
