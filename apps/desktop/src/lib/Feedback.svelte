<script lang="ts">
  // "Send feedback": stars, a category, the person's words and an optional reply address.
  // The desktop shell signs it and sends it to the High Turtle feedback service.
  import { sendFeedback, type FeedbackInput, type Invoke } from "./api";

  let { invoke }: { invoke?: Invoke } = $props();

  const categories: { value: FeedbackInput["category"]; label: string }[] = [
    { value: "feature", label: "Idea" },
    { value: "issue", label: "Problem" },
    { value: "praise", label: "Praise" },
    { value: "other", label: "Other" },
  ];
  const MAX = 5000;

  let rating = $state(0);
  let category = $state<FeedbackInput["category"]>("feature");
  let message = $state("");
  let email = $state("");
  let busy = $state(false);
  let error = $state("");
  let sent = $state(false);
  let ready = $derived(rating > 0 || message.trim().length > 0);
  let left = $derived(MAX - message.length);

  async function send() {
    busy = true;
    error = "";
    try {
      await sendFeedback({ rating: rating || null, category, message: message.trim(), email: email.trim() || null }, invoke);
      sent = true;
      rating = 0;
      message = "";
      email = "";
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
    busy = false;
  }
</script>

<h1>Send feedback</h1>
{#if sent}
  <section class="card" data-testid="feedback-thanks">
    <p><b>Thank you.</b> We read every message. It helps decide what Golden does next.</p>
    <button type="button" onclick={() => (sent = false)}>Send more</button>
  </section>
{:else}
  <p class="muted">Tell us what to add, what went wrong or what you like. It goes straight to the people who make Golden. Only what you write here is sent, never your financial data.</p>
  <section class="card form">
    <span class="lbl" id="fb-rate">How do you like Golden?</span>
    <div class="stars" role="radiogroup" aria-labelledby="fb-rate">
      {#each [1, 2, 3, 4, 5] as n}
        <button type="button" role="radio" aria-checked={rating === n} aria-label={`${n} star${n > 1 ? "s" : ""}`}
          class:on={n <= rating} onclick={() => (rating = rating === n ? 0 : n)}>★</button>
      {/each}
    </div>

    <span class="lbl">What's it about?</span>
    <div class="cats">
      {#each categories as c}
        <label><input type="radio" name="fb-cat" value={c.value} bind:group={category} /> {c.label}</label>
      {/each}
    </div>

    <label class="lbl" for="fb-msg">Your feedback</label>
    <textarea id="fb-msg" rows="7" maxlength={MAX} bind:value={message} placeholder="A feature you'd like, a problem you hit, or anything else"></textarea>
    {#if left <= 500}<span class="muted" data-testid="feedback-left">{left} characters left</span>{/if}

    <label class="lbl" for="fb-email">Email (optional, only if you'd like a reply)</label>
    <input id="fb-email" type="email" autocomplete="email" bind:value={email} />

    {#if error}<p class="err" role="alert">{error}</p>{/if}
    <div><button type="button" class="send" onclick={send} disabled={busy || !ready}>{busy ? "Sending…" : "Send"}</button></div>
  </section>
{/if}

<style>
  .muted { color: var(--muted); max-width: 40em; }
  .form { display: flex; flex-direction: column; gap: 10px; max-width: 620px; }
  .lbl { font-size: 12px; font-weight: 600; letter-spacing: 0.02em; text-transform: uppercase; color: var(--muted); margin-top: 6px; }
  .stars { display: flex; gap: 4px; }
  .stars button { width: 38px; height: 38px; padding: 0; font-size: 20px; color: var(--line); }
  .stars button.on { color: var(--gold); border-color: var(--gold); }
  .cats { display: flex; gap: 16px; flex-wrap: wrap; }
  .cats label { display: flex; align-items: center; gap: 6px; cursor: pointer; }
  .cats input { accent-color: var(--charcoal); }
  textarea { font: inherit; color: inherit; border: 1px solid var(--line); background: var(--paper); padding: 8px 12px; resize: vertical; }
  .err { color: var(--bad); }
  .send { border-color: var(--charcoal); background: var(--charcoal); color: var(--paper); }
  .send:disabled { opacity: 0.5; cursor: default; }
</style>
