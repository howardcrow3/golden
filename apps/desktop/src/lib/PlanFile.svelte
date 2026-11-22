<script lang="ts">
  import { exportPlanFile, importPlanFile, saveReport, type Invoke } from "./api";

  let { invoke, onimported }: { invoke?: Invoke; onimported?: () => void } = $props();
  let note = $state("");
  let error = $state("");

  async function save() {
    note = "";
    error = "";
    try {
      const text = await exportPlanFile(invoke);
      const path = await saveReport(`Golden plan file ${new Date().toISOString().slice(0, 10)}`, "json", text, invoke);
      note = `Saved to ${path}. Send that file to someone with Golden; it holds your household and holdings but no transactions.`;
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }

  function readText(f: File): Promise<string> {
    return new Promise((resolve, reject) => {
      const r = new FileReader();
      r.onload = () => resolve(String(r.result));
      r.onerror = () => reject(new Error("Couldn't read that file."));
      r.readAsText(f);
    });
  }

  async function open(e: Event) {
    const input = e.currentTarget as HTMLInputElement;
    const file = input.files?.[0];
    if (!file) return;
    note = "";
    error = "";
    try {
      const text = await readText(file);
      const r = await importPlanFile(text, invoke);
      note = `Opened a plan with ${r.people} ${r.people === 1 ? "person" : "people"}${r.holdings ? ` and ${r.holdings} holdings` : ""}. It replaced the plan on this computer.`;
      onimported?.();
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    } finally {
      input.value = "";
    }
  }
</script>

<div class="pf" data-testid="plan-file">
  <h3>Share this plan</h3>
  <p class="hint">Save the plan as a file to send to a spouse or advisor, or open one you were sent. No account and nothing goes online. Opening a file replaces your current plan, so save yours first.</p>
  <div class="row">
    <button type="button" onclick={save}>Save plan file</button>
    <label class="open">Open a plan file <input type="file" accept=".json" aria-label="Plan file" onchange={open} /></label>
  </div>
  {#if note}<p class="ok" role="status">{note}</p>{/if}
  {#if error}<p class="err" role="alert">{error}</p>{/if}
</div>

<style>
  .pf { margin-top: 16px; border-top: 1px solid var(--line, #ddd); padding-top: 12px; }
  h3 { margin: 0 0 4px; font-size: 15px; }
  .hint { color: var(--muted); font-size: 13px; margin: 0 0 8px; }
  .row { display: flex; gap: 12px; align-items: center; flex-wrap: wrap; }
  .open { font-size: 13px; }
  .err { color: var(--danger, #b3261e); }
</style>
