<script lang="ts">
  // First-launch terms dialog. It cannot be dismissed: the person accepts (after ticking the box)
  // or quits. It shows again whenever TERMS_VERSION changes.
  import TermsText from "./TermsText.svelte";
  import { TERMS_TITLE, TERMS_VERSION } from "./terms";
  import { acceptTerms, quitApp, type Invoke } from "./api";

  let {
    updated = false,
    invoke,
    onaccepted,
    onquit = quitApp,
  }: { updated?: boolean; invoke?: Invoke; onaccepted: () => void; onquit?: () => void | Promise<void> } = $props();

  let agreed = $state(false);
  let busy = $state(false);
  let error = $state("");

  async function accept() {
    busy = true;
    error = "";
    try {
      await acceptTerms(TERMS_VERSION, invoke);
      onaccepted();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      busy = false;
    }
  }
</script>

<div class="tb-scrim">
  <div class="tb-modal gate" role="dialog" aria-modal="true" aria-labelledby="terms-title">
    <div class="tb-mbody">
      <div class="tb-mhead">
        <h2 id="terms-title">{updated ? "Golden's terms have changed" : TERMS_TITLE}</h2>
      </div>
      <p class="lead">
        {updated ? "Please read and accept the updated terms to keep using Golden." : "Welcome to Golden. Please read and accept these terms before you start."}
      </p>
      <!-- svelte-ignore a11y_no_noninteractive_tabindex: the text scrolls, so it must take focus for keyboard scrolling. -->
      <div class="tb-mpanel" role="region" tabindex="0" aria-label="Terms of use text">
        <TermsText />
      </div>
      <div class="foot">
        {#if error}<p class="err" role="alert">{error}</p>{/if}
        <label class="check">
          <input type="checkbox" bind:checked={agreed} />
          I have read and agree to the Terms of Use, and I understand Golden is not financial, tax or legal advice.
        </label>
        <div class="btns">
          <button type="button" onclick={() => onquit()}>Quit</button>
          <button type="button" class="primary" onclick={accept} disabled={!agreed || busy}>Accept and continue</button>
        </div>
      </div>
    </div>
  </div>
</div>

<style>
  .gate { grid-template-columns: 1fr; width: min(640px, 100%); height: min(640px, 100%); }
  .lead { margin: 0; padding: 0 20px 10px; color: var(--muted); }
  .foot { border-top: 1px solid var(--line); padding: 12px 20px 16px; display: grid; gap: 10px; }
  .check { display: flex; gap: 8px; align-items: flex-start; line-height: 1.45; color: var(--ink); }
  .check input { margin-top: 2px; flex: none; }
  .btns { display: flex; justify-content: flex-end; gap: 8px; }
  .err { color: var(--danger); margin: 0; }
  .primary { background: var(--accent); border-color: var(--accent); color: var(--on-accent); }
  .primary:hover:not(:disabled) { background: var(--accent); filter: brightness(.95); }
</style>
