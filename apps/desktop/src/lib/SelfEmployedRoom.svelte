<script lang="ts">
  import type { QuickPlanResult } from "./api";
  import { formatCents } from "./money";

  let { result }: { result: QuickPlanResult } = $props();
  let rows = $derived(result.selfEmployedLimits ?? []);
</script>

{#if rows.length > 0}
  <section class="room" aria-label="Retirement saving room for self-employed income">
    <h3>Saving room from self-employment</h3>
    {#each rows as r}
      <p data-testid="se-room">
        <strong>{r.label}</strong>: a Solo 401(k) allows up to {formatCents(r.solo401kCents, { whole: true })} this year, a SEP IRA
        {formatCents(r.sepIraCents, { whole: true })}. The plan counts {formatCents(r.savingCents, { whole: true })} saved.
        {#if r.savingCents < r.solo401kCents}
          <span class="note">You could save {formatCents(r.solo401kCents - r.savingCents, { whole: true })} more.</span>
        {/if}
      </p>
    {/each}
    <p class="note">2026 limits, on net earnings after half the self-employment tax. Estimates, not tax advice.</p>
  </section>
{/if}

<style>
  .room { margin-top: 16px; }
  h3 { margin: 0 0 6px; font-size: 15px; }
  p { margin: 2px 0; font-size: 13px; }
  .note { color: var(--muted); font-size: 12px; }
</style>
