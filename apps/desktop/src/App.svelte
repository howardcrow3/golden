<script lang="ts">
  import Overview from "./lib/Overview.svelte";
  import TaxCalculator from "./lib/TaxCalculator.svelte";
  import Retirement from "./lib/Retirement.svelte";
  import Import from "./lib/Import.svelte";
  import Documents from "./lib/Documents.svelte";
  import Transactions from "./lib/Transactions.svelte";
  import WhatIf from "./lib/WhatIf.svelte";
  import Banks from "./lib/Banks.svelte";
  import Settings, { type SettingsTab } from "./lib/Settings.svelte";
  import Ask from "./lib/Ask.svelte";
  import PrivacyBadge from "./lib/PrivacyBadge.svelte";
  import { feedbackAvailable, licenseStatus, onLicenseLink, type LicenseStatus } from "./lib/api";
  import { setHideAmounts } from "./lib/money";

  const pages = ["Overview", "Transactions", "Import", "Banks", "Documents", "Ask", "Retirement", "What if", "Tax estimate"] as const;
  let page = $state<(typeof pages)[number]>("Overview");
  let settings = $state<SettingsTab | null>(null);
  let canFeedback = $state(false);
  $effect(() => {
    feedbackAvailable().then((ok) => (canFeedback = ok));
  });
  let lic = $state<LicenseStatus | null>(null);
  $effect(() => {
    licenseStatus().then((s) => (lic = s)).catch(() => (lic = null));
  });
  let linkNotice = $state("");
  $effect(() => {
    let stop = () => {};
    let gone = false;
    onLicenseLink((r) => {
      linkNotice = r.message;
      settings = "Licence";
      licenseStatus().then((s) => (lic = s)).catch(() => {});
    }).then((u) => (gone ? u() : (stop = u)));
    return () => {
      gone = true;
      stop();
    };
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
  <nav class="side" aria-label="Main">
    <div class="wm"><i>G</i>Golden</div>
    {#each pages as p}
      <button class="nav" aria-pressed={page === p} aria-current={page === p ? "true" : undefined} onclick={() => (page = p)}>{p}</button>
    {/each}
    <div class="grow"></div>
    <button class="nav" aria-pressed={hide} onclick={toggleHide} title="Hide dollar amounts, for screen sharing">{hide ? "Show amounts" : "Hide amounts"}</button>
    {#if badge}<div class="badge" data-testid="badge">{badge}</div>{/if}
    <PrivacyBadge onclick={() => (settings = "Privacy")} />
    <button class="nav" aria-haspopup="dialog" onclick={() => (settings = "Licence")}>Settings</button>
  </nav>
  <main>
    {#key hide}
    {#if page === "Overview"}<Overview onnavigate={(p) => { if ((pages as readonly string[]).includes(p)) page = p as (typeof pages)[number]; }} />{:else if page === "Transactions"}<Transactions />{:else if page === "Import"}<Import />{:else if page === "Banks"}<Banks />{:else if page === "Documents"}<Documents />{:else if page === "Ask"}<Ask />{:else if page === "What if"}<WhatIf />{:else if page === "Retirement"}<Retirement />{:else}<TaxCalculator />{/if}
    {/key}
  </main>
</div>

{#if settings}<Settings bind:tab={settings} {canFeedback} notice={linkNotice} onclose={() => (settings = null)} />{/if}

<style>
  .shell { display: grid; grid-template-columns: 216px 1fr; height: 100vh; }
  /* .side, .wm and .nav come from the framework; the nav buttons here are plain buttons, so reset the bare-control look. */
  nav.side { overflow-y: auto; }
  nav.side .nav { border-color: transparent; background: transparent; height: 28px; padding: 0 8px; font-weight: 450; }
  nav.side .nav:hover:not(:disabled) { background: var(--hover); }
  nav.side .nav[aria-pressed="true"] { background: var(--selected); border-color: transparent; color: var(--ink); }
  .badge { font-size: 11px; font-weight: 500; color: var(--ink); background: var(--accent-soft); border: 1px solid var(--accent); border-radius: var(--r-sm); padding: 4px 8px; margin: 6px 0 2px; }
  .hide :global(input) { -webkit-text-security: disc; }
  main { padding: 28px 36px; overflow-y: auto; min-width: 0; }
  main > :global(*) { max-width: 1040px; }
</style>
