<script lang="ts">
  import { appInfo, logFolder as loadLogFolder, logTail, type AppInfo, type Invoke } from "./api";

  let { invoke }: { invoke?: Invoke } = $props();
  let info = $state<AppInfo | null>(null);
  let failed = $state(false);
  $effect(() => {
    appInfo(invoke).then((i) => (info = i)).catch(() => (failed = true));
  });

  let logFolder = $state<string | null>(null);
  let copied = $state("");
  $effect(() => {
    loadLogFolder(invoke).then((f) => (logFolder = f)).catch(() => {});
  });

  /** Put the newest log lines on the clipboard to paste into a bug report. */
  export async function copyLog() {
    try {
      const text = await logTail(300, invoke);
      await navigator.clipboard.writeText(text);
      copied = "Copied the last 300 log lines.";
    } catch {
      copied = "Couldn't copy the log.";
    }
  }

  /** "Build 57" for a CI build, "local build" otherwise. */
  export function buildLabel(i: AppInfo): string {
    return /^\d+$/.test(i.build) ? `Build ${i.build}` : "Local build";
  }
</script>

<h1>About Golden</h1>
{#if info}
  <section class="card" data-testid="about">
    <div class="line"><span>Version</span><b class="num" data-testid="about-version">{info.version}</b></div>
    <div class="line"><span>Build</span><b class="num" data-testid="about-build">{buildLabel(info)}</b></div>
    <div class="line"><span>Commit</span><b class="num" data-testid="about-commit">{info.commit}</b></div>
  </section>
  {#if logFolder}
    <section class="card" data-testid="about-log">
      <div class="line"><span>Log folder</span><b class="num" data-testid="about-log-folder">{logFolder}</b></div>
      <p class="muted">Golden keeps a local, rotating log of what it did and what went wrong. Amounts, account numbers and keys are never written to it, and it never leaves this computer.</p>
      <button onclick={copyLog} data-testid="about-copy-log">Copy log</button>
      {#if copied}<span class="muted" data-testid="about-copied">{copied}</span>{/if}
    </section>
  {/if}
  <p class="muted">Quote the build number when you report something, so we know exactly which installer you ran. It also appears in the installer's file name.</p>
{:else if failed}
  <p class="muted">Version details are only available in the Golden desktop app.</p>
{/if}

<style>
  .line { display: flex; justify-content: space-between; padding: 8px 0; border-bottom: 1px solid var(--line); }
  .muted { color: var(--muted); }
</style>
