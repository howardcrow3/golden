<script lang="ts">
  import { appInfo, type AppInfo, type Invoke } from "./api";

  let { invoke }: { invoke?: Invoke } = $props();
  let info = $state<AppInfo | null>(null);
  let failed = $state(false);
  $effect(() => {
    appInfo(invoke).then((i) => (info = i)).catch(() => (failed = true));
  });

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
  <p class="muted">Quote the build number when you report something, so we know exactly which installer you ran. It also appears in the installer's file name.</p>
{:else if failed}
  <p class="muted">Version details are only available in the Golden desktop app.</p>
{/if}

<style>
  .line { display: flex; justify-content: space-between; padding: 8px 0; border-bottom: 1px solid var(--line); }
  .muted { color: var(--muted); }
</style>
