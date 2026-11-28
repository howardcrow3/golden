<script lang="ts">
  import type { LicenseStatus } from "./api";
  import { menuLicense, pillText } from "./licenseText";

  type Tab = "Licence" | "Privacy" | "Terms" | "About" | "Feedback";
  let { lic, canFeedback = false, onsettings }: { lic: LicenseStatus | null; canFeedback?: boolean; onsettings: (tab: Tab) => void } = $props();
  let open = $state(false);
  let root: HTMLDivElement | undefined = $state();
  let pill = $derived(pillText(lic));
  let block = $derived(menuLicense(lic));
  let tabs = $derived<Tab[]>(canFeedback ? ["Licence", "Privacy", "Terms", "About", "Feedback"] : ["Licence", "Privacy", "Terms", "About"]);

  type Theme = "system" | "light" | "dark";
  const THEME_KEY = "golden.theme";
  let theme = $state<Theme>("system");
  function read(): Theme {
    try {
      const saved = localStorage.getItem(THEME_KEY);
      return saved === "light" || saved === "dark" ? saved : "system";
    } catch {
      return "system"; // not remembered, follows the system
    }
  }
  function apply(t: Theme) {
    if (t === "system") document.documentElement.removeAttribute("data-theme");
    else document.documentElement.setAttribute("data-theme", t);
  }
  theme = read();
  $effect(() => apply(theme));
  function setTheme(t: Theme) {
    theme = t;
    apply(t);
    try {
      localStorage.setItem(THEME_KEY, t);
    } catch {
      // not remembered, still applied
    }
  }
  function choose(tab: Tab) {
    open = false;
    onsettings(tab);
  }
  function onWindowClick(e: MouseEvent) {
    if (open && root && !root.contains(e.target as Node)) open = false;
  }
  function onKey(e: KeyboardEvent) {
    if (e.key === "Escape") open = false;
  }
</script>

<svelte:window onclick={onWindowClick} onkeydown={onKey} />

<div class="acct-wrap" bind:this={root}>
  {#if open}
    <div class="pop" role="menu" aria-label="Account" data-testid="account-menu">
      {#if block}
        <div class="lic {block.tone}" role="group" aria-label="Licence">
          <span class="t">{block.title}</span>
          <span class="d">{block.detail}</span>
          {#if block.buy}<button type="button" class="buy" onclick={() => choose("Licence")}>Buy Golden or enter a key</button>{/if}
        </div>
      {/if}
      <div class="grp">Settings</div>
      {#each tabs as t}
        <button type="button" role="menuitem" class="nav" onclick={() => choose(t)}>{t}</button>
      {/each}
      <hr />
      <div class="seg" role="group" aria-label="Theme">
        {#each ["system", "light", "dark"] as const as t}
          <button type="button" aria-pressed={theme === t} onclick={() => setTheme(t)}>{t[0].toUpperCase() + t.slice(1)}</button>
        {/each}
      </div>
    </div>
  {/if}
  <button type="button" class="acct" aria-haspopup="menu" aria-expanded={open} data-testid="account" onclick={() => (open = !open)}>
    <span class="av" aria-hidden="true">G</span>
    <span class="who"><span class="n">Account</span>{#if pill}<span class="s {pill.tone}" data-testid="badge">{pill.text}</span>{/if}</span>
  </button>
</div>

<style>
  .acct-wrap { position: relative; }
  .who { display: flex; flex-direction: column; min-width: 0; }
  .nav { border-color: transparent; background: transparent; height: 26px; padding: 0 8px; font-weight: 450; }
  .nav:hover:not(:disabled) { background: var(--hover); }
  .buy { height: 26px; border: 0; border-radius: var(--r-sm); background: var(--ink); color: var(--on-ink); font-size: 12px; font-weight: 500; }
  .seg { display: flex; border: 1px solid var(--line); border-radius: var(--r-sm); overflow: hidden; margin: 2px; }
  .seg button { flex: 1; height: 24px; padding: 0; border: 0; border-radius: 0; background: transparent; color: var(--body); font-size: 12px; }
  .seg button[aria-pressed="true"] { background: var(--ink); color: var(--on-ink); }
</style>
