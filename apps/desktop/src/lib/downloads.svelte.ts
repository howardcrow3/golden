import { createDownloads, type DownloadItem } from "../turtlebox/tb-downloads.js";
import { cancelModelDownload, modelProgress, modelStatus, startModelDownload, type Invoke, type ModelProgress, type ModelStatus } from "./api";

/** The one download state for this window. The toolbar panel and the What if card both read it. */
export const downloads = createDownloads();

class DownloadView {
  items = $state<DownloadItem[]>([]);
  constructor() {
    downloads.subscribe((i) => (this.items = i));
  }
  get active() {
    return this.items.filter((i) => i.state === "running" || i.state === "queued").length;
  }
}
export const view = new DownloadView();

type Intent = "none" | "pause" | "cancel";

/** Map what the backend reports to a downloads-panel item. Null when there is nothing to list. */
export function modelItem(status: ModelStatus | null, p: ModelProgress, intent: Intent, seen: boolean) {
  if (!status) return null;
  const base = { id: `model:${status.id}`, name: status.name, note: "model", total: (p.totalMb || status.sizeMb) * 1e6, done: p.downloadedMb * 1e6, error: "" };
  if (p.running) return { ...base, state: "running" as const };
  if (status.installed) return seen ? { ...base, done: base.total, state: "done" as const } : null;
  if (intent === "pause") return { ...base, state: "paused" as const };
  if (intent === "cancel") return { ...base, state: "canceled" as const };
  if (p.error) return { ...base, state: "failed" as const, error: p.error };
  return seen ? { ...base, state: "failed" as const, error: "The download stopped" } : null;
}

/** The model download: status, progress and its actions, shared by every screen. */
export class ModelDownloadState {
  status = $state<ModelStatus | null>(null);
  progress = $state<ModelProgress>({ running: false, downloadedMb: 0, totalMb: 0, error: "" });
  error = $state("");
  private intent: Intent = "none";
  private seen = false;
  private timer: ReturnType<typeof setInterval> | undefined;

  constructor(private invoke?: Invoke, private pollMs = 500) {}

  async refresh() {
    try {
      this.status = await modelStatus(this.invoke);
      this.error = "";
    } catch (e) {
      this.error = e instanceof Error ? e.message : String(e);
    }
    this.publish();
  }

  /** Status plus progress, and keep polling if a download is already running. */
  async load() {
    await this.refresh();
    try {
      this.progress = await modelProgress(this.invoke);
      if (this.progress.running) {
        this.seen = true;
        this.watch();
      }
      this.publish();
    } catch {
      // Outside the desktop app.
    }
  }

  private watch() {
    if (this.timer) return;
    this.timer = setInterval(() => this.poll(), this.pollMs);
  }
  stop() {
    if (this.timer) clearInterval(this.timer);
    this.timer = undefined;
  }

  private async poll() {
    try {
      const was = this.progress.running;
      this.progress = await modelProgress(this.invoke);
      this.publish();
      if (!this.progress.running) {
        this.stop();
        if (was) await this.refresh();
      }
    } catch {
      this.stop();
    }
  }

  private publish() {
    const i = modelItem(this.status, this.progress, this.intent, this.seen);
    if (!i) return;
    downloads.update({ ...i, actions: { pause: () => this.pause(), resume: () => this.start(), cancel: () => this.cancel(), retry: () => this.start() } });
  }

  async start() {
    if (!this.status) return;
    this.error = "";
    this.intent = "none";
    try {
      await startModelDownload(this.status.id, this.invoke);
      this.seen = true;
      this.progress = { running: true, downloadedMb: 0, totalMb: this.status.sizeMb, error: "" };
      this.publish();
      this.watch();
    } catch (e) {
      this.error = e instanceof Error ? e.message : String(e);
    }
  }
  /** The backend resumes a partial file, so pausing is a cancel that remembers you meant to come back. */
  async pause() {
    this.intent = "pause";
    await cancelModelDownload(this.invoke).catch(() => {});
  }
  async cancel() {
    this.intent = "cancel";
    await cancelModelDownload(this.invoke).catch(() => {});
  }
}

const byInvoke = new Map<Invoke | undefined, ModelDownloadState>();
/** One state per backend, so the app and the What if card share it (tests pass their own invoke). */
export function modelDownload(invoke?: Invoke, pollMs?: number): ModelDownloadState {
  let s = byInvoke.get(invoke);
  if (!s) byInvoke.set(invoke, (s = new ModelDownloadState(invoke, pollMs)));
  return s;
}
