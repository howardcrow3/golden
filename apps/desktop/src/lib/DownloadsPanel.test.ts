import { afterEach, describe, expect, it } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import DownloadsPanel from "./DownloadsPanel.svelte";
import { downloads, modelItem } from "./downloads.svelte";

afterEach(() => {
  cleanup();
  for (const i of downloads.items()) downloads.remove(i.id);
});

const status = { ramGb: 16, id: "q", name: "Qwen3 4B", sizeMb: 2500, installed: false, path: "" };
const p = (o = {}) => ({ running: false, downloadedMb: 0, totalMb: 0, error: "", ...o });

describe("modelItem", () => {
  it("lists nothing before a download", () => expect(modelItem(status, p(), "none", false)).toBeNull());
  it("follows pause and cancel", () => {
    expect(modelItem(status, p({ downloadedMb: 5 }), "pause", true)?.state).toBe("paused");
    expect(modelItem(status, p({ downloadedMb: 5 }), "cancel", true)?.state).toBe("canceled");
  });
  it("reports a failure", () => expect(modelItem(status, p({ error: "Checksum" }), "none", true)).toMatchObject({ state: "failed", error: "Checksum" }));
});

describe("DownloadsPanel", () => {
  it("is hidden with nothing to show", () => {
    render(DownloadsPanel);
    expect(screen.queryByRole("button", { name: /Downloads/ })).toBeNull();
  });
  it("shows a count while downloading and lists items with actions", async () => {
    let paused = false;
    downloads.update({ id: "a", name: "Qwen3 4B", total: 2.5e9, done: 1e9, state: "running", actions: { pause: () => (paused = true) } });
    downloads.update({ id: "b", name: "Old", total: 1, done: 1, state: "done" });
    render(DownloadsPanel);
    const btn = await screen.findByRole("button", { name: "Downloads, 1 active" });
    expect(btn.textContent).toContain("1");
    await fireEvent.click(btn);
    expect(screen.getByText("Ongoing")).toBeTruthy();
    expect(screen.getByText(/1 GB of 2.5 GB/)).toBeTruthy();
    await fireEvent.click(screen.getByRole("button", { name: "Pause Qwen3 4B" }));
    expect(paused).toBe(true);
    await fireEvent.input(screen.getByRole("searchbox", { name: "Filter downloads" }), { target: { value: "old" } });
    expect(screen.queryByText("Qwen3 4B")).toBeNull();
    await fireEvent.click(screen.getByRole("button", { name: "Clear all" }));
    expect(downloads.items()).toHaveLength(1);
  });
});
