<script lang="ts">
  // The terms of use, rendered from terms.ts. Used by the first-launch dialog and Settings > Terms.
  import { TERMS, TERMS_SUMMARY, TERMS_VERSION } from "./terms";
</script>

<div class="tb-notice warn summary" data-testid="terms-summary">
  <b>In short</b>
  <ul>
    {#each TERMS_SUMMARY as s}<li>{s}</li>{/each}
  </ul>
</div>
<div class="terms" data-testid="terms-text">
  {#each TERMS as sec}
    <h3>{sec.heading}</h3>
    {#each sec.paragraphs as p}<p>{p}</p>{/each}
  {/each}
  <p class="muted">Version {TERMS_VERSION}</p>
</div>

<style>
  .summary ul { margin: 4px 0 0; padding-left: 18px; }
  .summary li { margin: 2px 0; }
  .terms h3 { font-size: 13px; font-weight: 600; color: var(--ink); margin: 16px 0 4px; }
  .terms p { margin: 0 0 8px; line-height: 1.55; }
  .muted { color: var(--muted); }
</style>
