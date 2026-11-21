<script lang="ts">
  import { keepWhatIf, saveScenario, whatIf, type Invoke, type WhatIfReply } from "./api";
  import { verdict } from "./chart";
  import { formatPercent } from "./money";
  import ModelSetup from "./ModelSetup.svelte";
  import ModelDownload from "./ModelDownload.svelte";
  import AiActivity from "./AiActivity.svelte";
  import Explain from "./Explain.svelte";
  import Scenarios from "./Scenarios.svelte";

  let { invoke }: { invoke?: Invoke } = $props();
  type Turn = { question: string; reply: WhatIfReply; state: "open" | "kept" | "dismissed" };
  let text = $state("");
  let turns = $state<Turn[]>([]);
  let busy = $state(false);
  let error = $state("");

  const examples = ["What if I retire at 65?", "What if the market crashes?", "Spend $80k a year", "A bonus of $100k in 2028"];

  async function ask(q: string) {
    const question = q.trim();
    if (!question) return;
    busy = true;
    error = "";
    try {
      const reply = await whatIf(question, 400, invoke);
      turns = [...turns, { question, reply, state: "open" }];
      text = "";
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      busy = false;
    }
  }

  let scenarioRefresh = $state(0);
  let saveName = $state<Record<number, string>>({});
  let saved = $state<Record<number, string>>({});
  async function saveAsScenario(i: number) {
    error = "";
    try {
      const s = await saveScenario(saveName[i] ?? "", turns[i].reply.changes, invoke);
      saved[i] = s.name;
      scenarioRefresh++;
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }

  async function keep(i: number) {
    try {
      await keepWhatIf(turns[i].reply.changes, invoke);
      turns[i].state = "kept";
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }

  export function delta(before: number, after: number): string {
    const d = Math.round((after - before) * 100);
    if (d === 0) return "no change";
    return `${d > 0 ? "up" : "down"} ${Math.abs(d)} points`;
  }
</script>

<h1>What if</h1>
<p class="sub">Ask in plain words. Golden runs your plan with and without the change and shows both. Nothing is saved until you keep it.</p>

<form class="card ask" onsubmit={(e) => { e.preventDefault(); ask(text); }}>
  <label>Your question <input bind:value={text} aria-label="What if question" placeholder="What if I retire at 65?" /></label>
  <button type="submit" disabled={busy || !text.trim()}>{busy ? "Thinking…" : "Ask"}</button>
</form>
<div class="chips">
  {#each examples as ex}<button type="button" class="chip" onclick={() => ask(ex)} disabled={busy}>{ex}</button>{/each}
</div>

{#if error}<p class="err" role="alert">{error}</p>{/if}

<Scenarios {invoke} refresh={scenarioRefresh} />
<ModelDownload {invoke} />
<ModelSetup {invoke} />
<AiActivity {invoke} />

{#each turns as t, i}
  <section class="card turn" data-testid="turn">
    <div class="q">{t.question}</div>
    <p class="msg">{t.reply.message}</p>
    {#if t.reply.before && t.reply.after}
      {@const b = verdict(t.reply.before.successRate)}
      {@const a = verdict(t.reply.after.successRate)}
      <div class="compare num">
        <div><span>Now</span><b data-testid="before">{formatPercent(t.reply.before.successRate, 0)}</b> <em class={b.tone}>{b.label}</em></div>
        <div><span>With the change</span><b data-testid="after">{formatPercent(t.reply.after.successRate, 0)}</b> <em class={a.tone}>{a.label}</em></div>
        <div class="d">{delta(t.reply.before.successRate, t.reply.after.successRate)}</div>
      </div>
      <Explain
        {invoke}
        facts={{
          successRate: t.reply.after.successRate,
          depletedYear: t.reply.after.depletedYear,
          retireAge: null,
          horizonAge: t.reply.after.horizonAge,
          compare: { beforeRate: t.reply.before.successRate, afterRate: t.reply.after.successRate, change: t.reply.descriptions.join(" and ") }
        }}
      />
      {#if t.state === "open"}
        <div class="actions">
          <button type="button" onclick={() => keep(i)}>Keep this change</button>
          <button type="button" onclick={() => (turns[i].state = "dismissed")}>Dismiss</button>
        </div>
        {#if saved[i]}
          <p role="status" data-testid="scenario-saved">Saved as "{saved[i]}" below.</p>
        {:else}
          <div class="actions">
            <input bind:value={saveName[i]} aria-label="Scenario name" placeholder="Name this scenario" />
            <button type="button" onclick={() => saveAsScenario(i)} disabled={!(saveName[i] ?? "").trim()}>Save as scenario</button>
          </div>
        {/if}
      {:else}
        <p class="done" role="status">{t.state === "kept" ? "Saved to your plan." : "Dismissed. Nothing was changed."}</p>
      {/if}
    {/if}
  </section>
{/each}

<style>
  .sub { color: var(--muted); margin: 0 0 20px; }
  .ask { display: flex; gap: 12px; align-items: end; margin-bottom: 12px; }
  .ask label { flex: 1; display: flex; flex-direction: column; gap: 4px; font-size: 12px; color: var(--muted); }
  .ask input { width: 100%; }
  .chips { display: flex; flex-wrap: wrap; gap: 8px; margin-bottom: 16px; }
  .chip { font-size: 13px; }
  .turn { margin-bottom: 12px; }
  .q { font-weight: 600; }
  .msg { color: var(--muted); font-size: 13px; }
  .compare { display: flex; gap: 32px; align-items: baseline; flex-wrap: wrap; }
  .compare span { display: block; font-size: 12px; color: var(--muted); }
  .compare b { font-size: 28px; }
  .compare em { font-style: normal; font-size: 13px; }
  .d { color: var(--muted); }
  .good { color: var(--good); } .ok { color: var(--muted); } .bad { color: var(--bad); }
  .actions { display: flex; gap: 8px; margin-top: 12px; }
  .err { color: var(--bad); }
</style>
