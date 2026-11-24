import { afterEach, describe, expect, it } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import About from "./About.svelte";

afterEach(cleanup);

const info = (build: string) => async () => ({ name: "Golden", version: "0.1.57", build, commit: "abc1234", mode: "private" });

describe("About", () => {
  it("shows the version, build number and commit", async () => {
    render(About, { invoke: info("57") });
    expect((await screen.findByTestId("about-version")).textContent).toBe("0.1.57");
    expect(screen.getByTestId("about-build").textContent).toBe("Build 57");
    expect(screen.getByTestId("about-commit").textContent).toBe("abc1234");
  });
  it("calls a local build a local build", async () => {
    render(About, { invoke: info("dev") });
    expect((await screen.findByTestId("about-build")).textContent).toBe("Local build");
  });
  it("explains itself outside the desktop app", async () => {
    render(About, { invoke: async () => { throw new Error("no"); } });
    expect(await screen.findByText(/only available in the Golden desktop app/)).toBeTruthy();
  });
  it("shows the log folder and copies the log", async () => {
    const written: string[] = [];
    Object.assign(navigator, { clipboard: { writeText: async (t: string) => void written.push(t) } });
    const invoke = async (cmd: string) =>
      cmd === "log_folder" ? "/data/Logs" : cmd === "log_tail" ? "line one\nline two" : { name: "Golden", version: "0.1.57", build: "57", commit: "abc1234", mode: "private" };
    render(About, { invoke });
    expect((await screen.findByTestId("about-log-folder")).textContent).toBe("/data/Logs");
    await fireEvent.click(screen.getByTestId("about-copy-log"));
    expect((await screen.findByTestId("about-copied")).textContent).toMatch(/Copied/);
    expect(written).toEqual(["line one\nline two"]);
  });
});
