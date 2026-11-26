<script lang="ts">
  import { ask, type AskAnswer, type Invoke } from "./api";
  import { formatCents } from "./money";

  let { invoke, compact = false }: { invoke?: Invoke; compact?: boolean } = $props();
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

{#if !compact}<h1>Ask</h1>{/if}
{#if !compact}<p class="sub">Ask about your own numbers in plain words. Every figure comes straight from your statements, and the transactions behind it are listed.</p>{/if}

<div class="ask" class:compact>
<div class="tb-thread">
{#each turns as t}
  <section class="turn" data-testid="answer">
    <div class="tb-msg-u">{t.question}</div>
    <div class="tb-msg-a">
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
    </div>
  </section>
{/each}
</div>

<form class="tb-composer" onsubmit={(e) => { e.preventDefault(); go(text); }}>
  <input bind:value={text} aria-label="Question" placeholder={compact ? "Ask about your money" : "How much did I spend on dining out last month?"} />
  <button type="submit" class="tb-send" disabled={busy || !text.trim()}>{busy ? "Looking…" : "Ask"}</button>
</form>
<div class="tb-sugs">
  {#each examples as ex}<button type="button" class="tb-sug" onclick={() => go(ex)} disabled={busy}>{ex}</button>{/each}
</div>
{#if error}<p class="err" role="alert">{error}</p>{/if}
</div>

<style>
  .sub { color: var(--muted); margin: 0 0 20px; }
  .tb-thread { margin-bottom: 16px; }
  .tb-composer { margin-bottom: 10px; }
  .tb-sugs { margin-bottom: 16px; }
  .turn { display: grid; gap: 10px; }
  .msg { font-size: 14px; }
  summary { color: var(--muted); font-size: 12.5px; cursor: pointer; }
  ul { padding-left: 18px; }
  .compact .msg { font-size: 13px; }
  .compact .tb-msg-a { font-size: 13px; }
  .compact .tb-sugs { margin-bottom: 8px; }
</style>
