<script lang="ts">
  import { keepWhatIf, listScenarios, saveScenario, whatIf, type Invoke, type SavedScenario, type WhatIfReply } from "./api";
  import { onMount } from "svelte";
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
      const reply = await whatIf(question, 400, invoke, base ? Number(base) : undefined);
      turns = [...turns, { question, reply, state: "open" }];
      text = "";
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      busy = false;
    }
  }

  let base = $state("");
  let scenarios = $state<SavedScenario[]>([]);
  async function loadScenarios() {
    try {
      const list = await listScenarios(invoke);
      scenarios = Array.isArray(list) ? list : [];
      if (base && !scenarios.some((s) => String(s.id) === base)) base = "";
    } catch {
      scenarios = [];
    }
  }
  onMount(loadScenarios);

  let scenarioRefresh = $state(0);
  let saveName = $state<Record<number, string>>({});
  let saved = $state<Record<number, string>>({});
  async function saveAsScenario(i: number) {
    error = "";
    try {
      const s = await saveScenario(saveName[i] ?? "", turns[i].reply.changes, invoke);
      saved[i] = s.name;
      scenarioRefresh++;
      await loadScenarios();
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

<form class="tb-composer" onsubmit={(e) => { e.preventDefault(); ask(text); }}>
  <input bind:value={text} aria-label="What if question" placeholder="What if I retire at 65?" />
  {#if scenarios.length}
    <select bind:value={base} aria-label="Build on">
      <option value="">Current plan</option>
      {#each scenarios as sc}<option value={String(sc.id)}>{sc.name}</option>{/each}
    </select>
  {/if}
  <button type="submit" class="tb-send" disabled={busy || !text.trim()}>{busy ? "Thinking…" : "Ask"}</button>
</form>
<div class="tb-sugs">
  {#each examples as ex}<button type="button" class="tb-sug" onclick={() => ask(ex)} disabled={busy}>{ex}</button>{/each}
</div>

{#if error}<p class="err" role="alert">{error}</p>{/if}

<Scenarios {invoke} refresh={scenarioRefresh} />
<ModelDownload {invoke} />
<ModelSetup {invoke} />
<AiActivity {invoke} />

{#each turns as t, i}
  <section class="card turn" data-testid="turn">
    <div class="tb-msg-u">{t.question}</div>
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
  .tb-composer { margin-bottom: 10px; }
  .tb-composer select { border: 0; background-color: transparent; height: 26px; }
  .tb-sugs { margin-bottom: 16px; }
  .turn { margin-bottom: 12px; }
  .tb-msg-u { margin-bottom: 10px; width: fit-content; margin-left: auto; }
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
