import { afterEach, describe, expect, it } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import ModelDownload from "./ModelDownload.svelte";
import { downloads } from "./downloads.svelte";

// The download state is shared by the whole window; start each test with none.
afterEach(() => {
  cleanup();
  for (const i of downloads.items()) downloads.remove(i.id);
});

const status = { ramGb: 16, id: "qwen3-4b", name: "Qwen3 4B", sizeMb: 2500, installed: false, path: "/m/q.gguf" };
const idle = { running: false, downloadedMb: 0, totalMb: 0, error: "" };

function fake(log: string[], opts: { status?: typeof status; progress?: () => typeof idle; startFail?: string } = {}) {
  let st = opts.status ?? status;
  return async (cmd: string) => {
    log.push(cmd);
    if (cmd === "model_status") return st;
    if (cmd === "model_progress") return opts.progress ? opts.progress() : idle;
    if (cmd === "start_model_download" && opts.startFail) throw new Error(opts.startFail);
    if (cmd === "remove_model") st = { ...st, installed: false };
    return undefined;
  };
}

describe("ModelDownload", () => {
  it("offers a download sized for this computer", async () => {
    render(ModelDownload, { invoke: fake([]) });
    expect(await screen.findByText(/Get Qwen3 4B \(2\.5 GB\)/)).toBeTruthy();
    expect(screen.getByText(/16 GB of memory/)).toBeTruthy();
  });
  it("starts the download and shows progress", async () => {
    const log: string[] = [];
    let n = 0;
    render(ModelDownload, { invoke: fake(log, { progress: () => (n++ === 0 ? idle : { running: true, downloadedMb: 1250, totalMb: 2500, error: "" }) }), pollMs: 5 });
    await fireEvent.click(await screen.findByRole("button", { name: "Download" }));
    expect(log).toContain("start_model_download");
    const bar = await screen.findByRole("progressbar");
    await new Promise((r) => setTimeout(r, 30));
    expect(bar.getAttribute("aria-valuenow")).toBe("50");
    expect(screen.getByRole("button", { name: /^Cancel/ })).toBeTruthy();
  });
  it("cancels", async () => {
    const log: string[] = [];
    render(ModelDownload, { invoke: fake(log, { progress: () => ({ running: true, downloadedMb: 10, totalMb: 2500, error: "" }) }), pollMs: 5 });
    await fireEvent.click(await screen.findByRole("button", { name: /^Cancel/ }));
    expect(log).toContain("cancel_model_download");
  });
  it("shows installed and removes", async () => {
    const log: string[] = [];
    render(ModelDownload, { invoke: fake(log, { status: { ...status, installed: true } }) });
    expect(await screen.findByText(/is installed/)).toBeTruthy();
    await fireEvent.click(screen.getByRole("button", { name: "Remove" }));
    expect(log).toContain("remove_model");
    expect(await screen.findByRole("button", { name: "Download" })).toBeTruthy();
  });
  it("says why a download failed to start", async () => {
    render(ModelDownload, { invoke: fake([], { startFail: "A download is already running." }) });
    await fireEvent.click(await screen.findByRole("button", { name: "Download" }));
    expect((await screen.findByRole("alert")).textContent).toMatch(/already running/);
  });
  it("shows a download error", async () => {
    render(ModelDownload, { invoke: fake([], { progress: () => ({ ...idle, error: "Checksum mismatch" }) }) });
    expect((await screen.findByRole("alert")).textContent).toMatch(/Checksum/);
  });
});
