<script lang="ts">
  // Settings > Terms: the terms of use, and when this computer accepted them.
  import TermsText from "./TermsText.svelte";
  import { TERMS_VERSION } from "./terms";
  import { termsAcceptance, type Invoke, type TermsAcceptance } from "./api";

  let { invoke }: { invoke?: Invoke } = $props();
  let accepted = $state<TermsAcceptance | null>(null);
  $effect(() => {
    termsAcceptance(invoke).then((a) => (accepted = a)).catch(() => (accepted = null));
  });
</script>

<h1>Terms</h1>
{#if accepted}
  <p class="muted" data-testid="terms-accepted">
    You accepted version {accepted.version} on {accepted.acceptedOn}.{accepted.version !== TERMS_VERSION ? ` The current version is ${TERMS_VERSION}.` : ""}
  </p>
{/if}
<TermsText />

<style>
  .muted { color: var(--muted); margin-top: 0; }
</style>
