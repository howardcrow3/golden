<script lang="ts">
  import License from "./License.svelte";
  import Privacy from "./Privacy.svelte";
  import About from "./About.svelte";
  import Feedback from "./Feedback.svelte";
  import Terms from "./Terms.svelte";

  export type SettingsTab = "Licence" | "Privacy" | "Terms" | "About" | "Feedback";
  let {
    tab = $bindable("Licence"),
    canFeedback = false,
    notice = "",
    onclose,
  }: { tab?: SettingsTab; canFeedback?: boolean; notice?: string; onclose: () => void } = $props();
  let tabs = $derived<SettingsTab[]>(canFeedback ? ["Licence", "Privacy", "Terms", "About", "Feedback"] : ["Licence", "Privacy", "Terms", "About"]);

  function onkeydown(e: KeyboardEvent) {
    if (e.key === "Escape") onclose();
  }
</script>

<svelte:window {onkeydown} />

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="tb-scrim" onclick={(e) => { if (e.target === e.currentTarget) onclose(); }}>
  <div class="tb-modal" role="dialog" aria-modal="true" aria-label="Settings">
    <div class="tb-mtabs" role="tablist" aria-label="Settings sections">
      <div class="ttl">Settings</div>
      {#each tabs as t}
        <button role="tab" aria-selected={tab === t} onclick={() => (tab = t)}>{t}</button>
      {/each}
    </div>
    <div class="tb-mbody">
      <div class="tb-mhead">
        <h2>{tab}</h2>
        <button class="tb-xbtn" aria-label="Close settings" onclick={onclose}>×</button>
      </div>
      <div class="tb-mpanel" role="tabpanel">
        {#if tab === "Licence"}<License {notice} />{:else if tab === "Privacy"}<Privacy />{:else if tab === "Terms"}<Terms />{:else if tab === "About"}<About />{:else}<Feedback />{/if}
      </div>
    </div>
  </div>
</div>

<style>
  /* The modal header already names the tab, so the screen's own title is redundant here. */
  .tb-mpanel :global(h1) { display: none; }
  .tb-mhead h2 { margin: 0; }
</style>
