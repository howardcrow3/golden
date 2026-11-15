<script lang="ts">
  import Overview from "./lib/Overview.svelte";
  import TaxCalculator from "./lib/TaxCalculator.svelte";
  import Retirement from "./lib/Retirement.svelte";

  const pages = ["Overview", "Retirement", "Tax estimate"] as const;
  let page = $state<(typeof pages)[number]>("Overview");
</script>

<div class="shell">
  <nav aria-label="Main">
    <div class="brand">Golden</div>
    {#each pages as p}
      <button aria-pressed={page === p} onclick={() => (page = p)}>{p}</button>
    {/each}
    <div class="mode">Private mode · nothing leaves this computer</div>
  </nav>
  <main>
    {#if page === "Overview"}<Overview />{:else if page === "Retirement"}<Retirement />{:else}<TaxCalculator />{/if}
  </main>
</div>

<style>
  .shell { display: grid; grid-template-columns: 220px 1fr; min-height: 100vh; }
  nav { border-right: 1px solid var(--line); padding: 20px 14px; display: flex; flex-direction: column; gap: 6px; background: var(--paper); }
  nav button { text-align: left; border-color: transparent; background: transparent; }
  nav button[aria-pressed="true"] { border-color: var(--gold); background: var(--ivory); }
  .brand { font-weight: 600; font-size: 18px; margin-bottom: 18px; }
  .mode { margin-top: auto; font-size: 12px; color: var(--muted); }
  main { padding: 32px 40px; max-width: 1100px; }
</style>
