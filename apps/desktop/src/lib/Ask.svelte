<script lang="ts">
  import { ask, type AskAnswer, type Invoke } from "./api";
  import { formatCents } from "./money";

  let { invoke }: { invoke?: Invoke } = $props();
  type Turn = { question: string; answer: AskAnswer };
  let text = $state("");
  let turns = $state<Turn[]>([]);
  let busy = $state(false);
  let error = $state("");

  const examples = [
    "How much did I spend last month?",
    "What did I earn this year?",
    "How much did I save last quarter?",
    "What is my net worth?",
  ];

  async function go(q: string) {
    const question = q.trim();
    if (!question) return;
    busy = true;
    error = "";
    try {
      turns = [...turns, { question, answer: await ask(question, invoke) }];
      text = "";
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      busy = false;
    }
  }
</script>

<h1>Ask</h1>
<p class="sub">Ask about your own numbers in plain words. Every figure comes straight from your statements, and the transactions behind it are listed.</p>

<form class="card ask" onsubmit={(e) => { e.preventDefault(); go(text); }}>
  <label>Your question <input bind:value={text} aria-label="Question" placeholder="How much did I spend on dining out last month?" /></label>
  <button type="submit" disabled={busy || !text.trim()}>{busy ? "Looking…" : "Ask"}</button>
</form>
<div class="chips">
  {#each examples as ex}<button type="button" class="chip" onclick={() => go(ex)} disabled={busy}>{ex}</button>{/each}
</div>
{#if error}<p class="err" role="alert">{error}</p>{/if}

{#each turns as t}
  <section class="card turn" data-testid="answer">
    <div class="q">{t.question}</div>
    <p class="msg">{t.answer.text}</p>
    {#if t.answer.behind.length}
      <details>
        <summary>The transactions behind this ({t.answer.behind.length})</summary>
        <ul>
          {#each t.answer.behind as r}
            <li data-testid="behind">{r.day} · {r.payee} · {r.account} · <span class="num">{formatCents(r.amountCents)}</span>{#if r.category} · {r.category}{/if}</li>
          {/each}
        </ul>
      </details>
    {/if}
  </section>
{/each}

<style>
  .sub { color: var(--muted); margin: 0 0 20px; }
  .ask { display: flex; gap: 12px; align-items: end; margin-bottom: 12px; }
  .ask label { flex: 1; display: flex; flex-direction: column; gap: 4px; font-size: 12px; color: var(--muted); }
  .chips { display: flex; gap: 8px; flex-wrap: wrap; margin-bottom: 16px; }
  .turn { margin-bottom: 12px; }
  .q { color: var(--muted); font-size: 13px; }
  .msg { margin: 6px 0; font-size: 16px; }
  ul { padding-left: 18px; }
</style>
