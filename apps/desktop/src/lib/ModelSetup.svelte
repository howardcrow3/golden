<script lang="ts">
  import { modelSettings, setModel, type Invoke } from "./api";

  let { invoke }: { invoke?: Invoke } = $props();
  let endpoint = $state("");
  let name = $state("");
  let saved = $state(false);
  let error = $state("");
  let note = $state("");
  let open = $state(false);

  $effect(() => {
    modelSettings(invoke)
      .then((s) => {
        endpoint = s.endpoint;
        name = s.name;
        saved = s.endpoint !== "";
      })
      .catch(() => {});
  });

  async function save() {
    error = "";
    note = "";
    try {
      const s = await setModel(endpoint, name, invoke);
      saved = s.endpoint !== "";
      note = saved ? "Saved. The what-if chat now asks that model first." : "Turned off. The what-if chat uses its built-in phrases.";
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }
</script>

<details class="card model" bind:open>
  <summary>{saved ? "Using a model on this computer" : "Understand more phrasings with a model on this computer"}</summary>
  <p class="muted">Optional. Point Golden at a language model already running here, such as Ollama. Only your question, first names and retirement ages are sent, never balances or birth dates, and never off this computer. If the model is off or confused, the built-in phrases still work.</p>
  <label>Model address <input bind:value={endpoint} aria-label="Model address" placeholder="http://127.0.0.1:11434/api/generate" /></label>
  <label>Model name <input bind:value={name} aria-label="Model name" placeholder="llama3.2" /></label>
  <button type="button" onclick={save}>Save</button>
  {#if error}<p class="err" role="alert">{error}</p>{/if}
  {#if note}<p role="status">{note}</p>{/if}
</details>

<style>
  .model { margin: 12px 0; }
  summary { cursor: pointer; }
  label { display: block; margin: 8px 0; }
  .muted { color: var(--muted); }
  .err { color: var(--bad); }
</style>
