<script lang="ts">
  import Overview from "./lib/Overview.svelte";
  import TaxCalculator from "./lib/TaxCalculator.svelte";
  import Retirement from "./lib/Retirement.svelte";
  import Import from "./lib/Import.svelte";
  import Documents from "./lib/Documents.svelte";
  import Transactions from "./lib/Transactions.svelte";
  import WhatIf from "./lib/WhatIf.svelte";
  import Banks from "./lib/Banks.svelte";
  import ChatDock from "./lib/ChatDock.svelte";
  import Settings, { type SettingsTab } from "./lib/Settings.svelte";
  import Ask from "./lib/Ask.svelte";
  import AccountMenu from "./lib/AccountMenu.svelte";
  import PrivacyBadge from "./lib/PrivacyBadge.svelte";
  import TermsGate from "./lib/TermsGate.svelte";
  import { needsAcceptance } from "./lib/terms";
  import { feedbackAvailable, inTauri, licenseStatus, onLicenseLink, termsAcceptance, type Invoke, type LicenseStatus } from "./lib/api";

  let { invoke }: { invoke?: Invoke } = $props();
  import { setHideAmounts } from "./lib/money";

  const pages = ["Overview", "Transactions", "Import", "Banks", "Documents", "Ask", "Retirement", "What if", "Tax estimate"] as const;
  let page = $state<(typeof pages)[number]>("Overview");
  let settings = $state<SettingsTab | null>(null);
  let canFeedback = $state(false);
  $effect(() => {
    feedbackAvailable().then((ok) => (canFeedback = ok));
  });
  /** "new" on first launch, "updated" when an older version was accepted, null once accepted. */
  let gate = $state<"new" | "updated" | null>(null);
  $effect(() => {
    termsAcceptance(invoke)
      .then((a) => (gate = needsAcceptance(a) ? (a ? "updated" : "new") : null))
      // In the desktop app a failed read still asks; the browser preview has no database.
      .catch(() => (gate = invoke || inTauri() ? "new" : null));
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
  const DOCK_KEY = "golden.chatDock";
  let dockOpen = $state(true);
  try {
    dockOpen = localStorage.getItem(DOCK_KEY) !== "0";
  } catch {
    // storage can be unavailable; the dock stays open for this session
  }
  function setDock(open: boolean) {
    dockOpen = open;
    try {
      localStorage.setItem(DOCK_KEY, open ? "1" : "0");
    } catch {
      // not remembered, still applied
    }
  }
  /** Ask and What if are chats themselves, so they do not get the dock beside them. */
  let showDock = $derived(dockOpen && page !== "Ask" && page !== "What if");

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
</script>

<div class="shell" class:hide inert={gate !== null}>
  <nav class="side" aria-label="Main">
    <div class="wm"><i>G</i>Golden</div>
    <div class="scroll">
    {#each pages as p}
      <button class="nav" aria-pressed={page === p} aria-current={page === p ? "true" : undefined} onclick={() => (page = p)}>{p}</button>
    {/each}
    <div class="grow"></div>
    <button class="nav" aria-pressed={dockOpen} onclick={() => setDock(!dockOpen)} title="Ask about your numbers beside any screen">Chat pane</button>
    <button class="nav" aria-pressed={hide} onclick={toggleHide} title="Hide dollar amounts, for screen sharing">{hide ? "Show amounts" : "Hide amounts"}</button>
    </div>
    <div class="acct-foot">
      <PrivacyBadge onclick={() => (settings = "Privacy")} />
      <AccountMenu {lic} {canFeedback} onsettings={(t) => (settings = t)} />
    </div>
  </nav>
  <main>
    {#key hide}
    {#if page === "Overview"}<Overview onnavigate={(p) => { if ((pages as readonly string[]).includes(p)) page = p as (typeof pages)[number]; }} />{:else if page === "Transactions"}<Transactions />{:else if page === "Import"}<Import />{:else if page === "Banks"}<Banks />{:else if page === "Documents"}<Documents />{:else if page === "Ask"}<Ask />{:else if page === "What if"}<WhatIf />{:else if page === "Retirement"}<Retirement />{:else}<TaxCalculator />{/if}
    {/key}
  </main>
  {#if showDock}<ChatDock page={page} onhide={() => setDock(false)} />{/if}
</div>

{#if settings}<Settings bind:tab={settings} {canFeedback} notice={linkNotice} onclose={() => (settings = null)} />{/if}
{#if gate}<TermsGate updated={gate === "updated"} {invoke} onaccepted={() => (gate = null)} />{/if}

<style>
  .shell { display: flex; height: 100vh; }
  .shell > nav { width: 216px; flex: none; }
  .shell > main { flex: 1; }
  /* .side, .wm and .nav come from the framework; the nav buttons here are plain buttons, so reset the bare-control look. */
  nav.side { overflow: visible; }
  nav.side .nav { border-color: transparent; background: transparent; height: 28px; padding: 0 8px; font-weight: 450; }
  nav.side .nav:hover:not(:disabled) { background: var(--hover); }
  nav.side .nav[aria-pressed="true"] { background: var(--selected); border-color: transparent; color: var(--ink); }
  .acct-foot { display: flex; flex-direction: column; gap: 2px; }
  .hide :global(input) { -webkit-text-security: disc; }
  main { padding: 28px 36px; overflow-y: auto; min-width: 0; }
  main > :global(*) { max-width: 1040px; }
</style>
