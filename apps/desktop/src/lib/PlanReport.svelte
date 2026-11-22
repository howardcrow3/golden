<script lang="ts">
  import { saveReport, type Invoke, type QuickPlanResult } from "./api";
  import { planCsv, planHtml, planJson } from "./report";

  let { invoke, result }: { invoke?: Invoke; result: QuickPlanResult } = $props();
  let note = $state("");
  let error = $state("");

  async function save(kind: "html" | "csv" | "json") {
    note = "";
    error = "";
    const when = new Date().toLocaleDateString(undefined, { dateStyle: "long" });
    const title = "Golden retirement plan";
    const content =
      kind === "html" ? planHtml(result, title, when) : kind === "csv" ? planCsv(result) : planJson(result, title, when);
    try {
      const path = await saveReport(`Golden plan ${new Date().toISOString().slice(0, 10)}`, kind, content, invoke);
      note = `Saved to ${path}`;
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }
</script>

<div class="report">
  <h3>Take the plan with you</h3>
  <p class="hint">Saved to your Exports folder. Open the report in a browser and print it to make a PDF.</p>
  <div class="row">
    <button type="button" onclick={() => save("html")}>Save report</button>
    <button type="button" onclick={() => save("csv")}>Save table (CSV)</button>
    <button type="button" onclick={() => save("json")}>Save data (JSON)</button>
  </div>
  {#if note}<p class="ok" role="status">{note}</p>{/if}
  {#if error}<p class="err" role="alert">{error}</p>{/if}
</div>

<style>
  .report { margin-top: 16px; }
  h3 { margin: 0 0 4px; font-size: 15px; }
  .hint { color: var(--muted); font-size: 13px; margin: 0 0 8px; }
  .row { display: flex; gap: 8px; flex-wrap: wrap; }
  .ok { color: var(--muted); font-size: 13px; word-break: break-all; }
  .err { color: var(--bad); }
</style>
