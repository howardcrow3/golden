<script lang="ts">
  import { getWithholdingCheck, type Gap, type Invoke, type WithholdingCheck } from "./api";
  import { formatCents } from "./money";

  let { invoke }: { invoke?: Invoke } = $props();
  let check = $state<WithholdingCheck | null>(null);

  $effect(() => {
    getWithholdingCheck(invoke)
      .then((c) => (check = c))
      .catch(() => (check = null));
  });

  function verdict(label: string, g: Gap): string {
    const d = Math.abs(g.differenceCents);
    // Within $100 a year of the estimate counts as right on.
    if (d < 10_000) return `${label} withholding is about right.`;
    return g.differenceCents > 0
      ? `${label}: about ${formatCents(d, { whole: true })} more is withheld than the estimate, so expect a refund.`
      : `${label}: about ${formatCents(d, { whole: true })} less is withheld than the estimate, so expect a bill.`;
  }
</script>

{#if check && check.taxableWagesCents > 0}
  <section class="card" data-testid="withholding">
    <h2>Is enough tax being withheld?</h2>
    {#if check.missing.length}
      <p class="muted" data-testid="withholding-missing">Read a paystub for {check.missing.join(", ")} to include it. Until then this can't be trusted.</p>
    {:else}
      <p data-testid="withholding-federal">{verdict("Federal", check.federal)}</p>
      <p data-testid="withholding-state">{verdict("State", check.state)}</p>
      <p class="muted">An estimate from pay alone: it leaves out interest, gains and other income{check.stateIsEstimate ? ", and your state's schedule is approximate" : ""}.</p>
    {/if}
  </section>
{/if}

<style>
  section { margin-top: 16px; }
  .muted { color: var(--muted); }
</style>
