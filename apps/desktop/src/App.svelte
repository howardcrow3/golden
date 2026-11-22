<script lang="ts">
  import Overview from "./lib/Overview.svelte";
  import TaxCalculator from "./lib/TaxCalculator.svelte";
  import Retirement from "./lib/Retirement.svelte";
  import Import from "./lib/Import.svelte";
  import Transactions from "./lib/Transactions.svelte";
  import WhatIf from "./lib/WhatIf.svelte";
  import License from "./lib/License.svelte";
  import About from "./lib/About.svelte";
  import Privacy from "./lib/Privacy.svelte";
  import Ask from "./lib/Ask.svelte";
  import PrivacyBadge from "./lib/PrivacyBadge.svelte";
  import { licenseStatus, type LicenseStatus } from "./lib/api";
  import { setHideAmounts } from "./lib/money";

  const pages = ["Overview", "Transactions", "Import", "Ask", "Retirement", "What if", "Tax estimate", "Licence", "Privacy", "About"] as const;
  let page = $state<(typeof pages)[number]>("Overview");
  let lic = $state<LicenseStatus | null>(null);
  $effect(() => {
    licenseStatus().then((s) => (lic = s)).catch(() => (lic = null));
  });
  const KEY = "golden.hideAmounts";
  let remembered = false;
  try {
    remembered = localStorage.getItem(KEY) === "1";
  } catch {
    // storage can be unavailable; the toggle still works for this session
  }
  let hide = $state(remembered);
  setHideAmounts(remembered);
  function toggleHide() {
    hide = !hide;
    setHideAmounts(hide);
    try {
      localStorage.setItem(KEY, hide ? "1" : "0");
    } catch {
      // not remembered, still applied
    }
  }
  let badge = $derived(
    lic === null ? "" : lic.state === "trial" ? `Trial · ${lic.daysLeft} days left` : lic.state === "expired" ? "Trial ended · read-only" : "Licensed",
  );
</script>

<div class="shell" class:hide>
  <nav aria-label="Main">
    <div class="brand">Golden</div>
    {#each pages as p}
      <button aria-pressed={page === p} onclick={() => (page = p)}>{p}</button>
    {/each}
    <div class="spacer"></div>
    <button class="hide-toggle" aria-pressed={hide} onclick={toggleHide} title="Hide dollar amounts, for screen sharing">{hide ? "Show amounts" : "Hide amounts"}</button>
    {#if badge}<div class="badge" data-testid="badge">{badge}</div>{/if}
    <PrivacyBadge onclick={() => (page = "Privacy")} />
  </nav>
  <main>
    {#key hide}
    {#if page === "Overview"}<Overview onnavigate={(p) => { if ((pages as readonly string[]).includes(p)) page = p as (typeof pages)[number]; }} />{:else if page === "Transactions"}<Transactions />{:else if page === "Import"}<Import />{:else if page === "Ask"}<Ask />{:else if page === "What if"}<WhatIf />{:else if page === "Retirement"}<Retirement />{:else if page === "Licence"}<License />{:else if page === "Privacy"}<Privacy />{:else if page === "About"}<About />{:else}<TaxCalculator />{/if}
    {/key}
  </main>
</div>

<style>
  .shell { display: grid; grid-template-columns: 220px 1fr; min-height: 100vh; }
  nav { border-right: 1px solid var(--line); padding: 20px 14px; display: flex; flex-direction: column; gap: 6px; background: var(--paper); }
  nav button { text-align: left; border-color: transparent; background: transparent; }
  nav button[aria-pressed="true"] { border-color: var(--gold); background: var(--ivory); }
  .brand { font-weight: 600; font-size: 18px; margin-bottom: 18px; }
  .spacer { flex: 1; }
  .badge { font-size: 12px; border: 1px solid var(--gold); padding: 6px 8px; }
  .hide :global(input) { -webkit-text-security: disc; }
  .hide-toggle { font-size: 12px; }
  main { padding: 32px 40px; max-width: 1100px; }
</style>
