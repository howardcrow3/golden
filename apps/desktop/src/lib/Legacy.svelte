<script lang="ts">
  import type { QuickPlanResult } from "./api";
  import { formatCents } from "./money";

  let { result }: { result: QuickPlanResult } = $props();
  let l = $derived(result.legacy);
</script>

{#if l && l.grossCents > 0}
  <section class="legacy" aria-label="What is left for heirs">
    <h3>What is left for heirs</h3>
    <dl>
      <dt>Estate at the end of the plan</dt>
      <dd data-testid="legacy-gross">{formatCents(l.grossCents)}</dd>
      <dt>Federal estate tax</dt>
      <dd data-testid="legacy-estate-tax">
        {l.estateTaxCents > 0 ? formatCents(l.estateTaxCents) : "None"}
        <span class="note">(exemption {formatCents(l.exemptionCents)})</span>
      </dd>
      {#if l.inheritedIraPerYearCents > 0}
        <dt>Inherited pre-tax accounts</dt>
        <dd data-testid="legacy-ira">
          {formatCents(l.inheritedIraPerYearCents)} a year for ten years,
          about {formatCents(l.heirIncomeTaxCents)} in income tax
        </dd>
      {/if}
      <dt>Net to heirs</dt>
      <dd data-testid="legacy-net"><strong>{formatCents(l.netToHeirsCents)}</strong></dd>
    </dl>
    <p class="note">Today's dollars. Assumes heirs pay 24% on pre-tax money and that a couple uses both exemptions. State estate and inheritance taxes are not included.</p>
  </section>
{/if}

<style>
  .legacy { margin-top: 16px; }
  h3 { margin: 0 0 6px; font-size: 15px; }
  dl { display: grid; grid-template-columns: auto 1fr; gap: 2px 12px; margin: 0; font-size: 13px; }
  dt { color: var(--muted); }
  dd { margin: 0; }
  .note { color: var(--muted); font-size: 12px; }
</style>
