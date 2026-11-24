import { describe, expect, it, vi } from "vitest";
import { errorText, installGlobalHandlers, shellReporter, wrapInvoke } from "./clientLog";

describe("clientLog", () => {
  it("formats errors of any kind", () => {
    expect(errorText(new Error("boom"))).toBe("boom");
    expect(errorText("plain")).toBe("plain");
    expect(errorText({ code: 3 })).toBe('{"code":3}');
  });

  it("logs a failed command with its name and error, then rethrows", async () => {
    const report = vi.fn();
    const raw = vi.fn().mockRejectedValue("Give every holding a name.");
    const call = wrapInvoke(raw, report);
    await expect(call("save_holdings", { holdings: [{ valueCents: 5 }] })).rejects.toBe("Give every holding a name.");
    expect(report).toHaveBeenCalledWith("error", "save_holdings", "Give every holding a name.");
    // The arguments are never part of the message.
    expect(JSON.stringify(report.mock.calls)).not.toContain("valueCents");
  });

  it("passes successful results through without logging", async () => {
    const report = vi.fn();
    const call = wrapInvoke(vi.fn().mockResolvedValue(42), report);
    expect(await call("answer")).toBe(42);
    expect(report).not.toHaveBeenCalled();
  });

  it("sends lines to client_log, trimmed, and never throws", async () => {
    const raw = vi.fn().mockRejectedValue(new Error("shell gone"));
    const report = shellReporter(raw);
    expect(() => report("warn", "x", "y".repeat(2000))).not.toThrow();
    expect(raw).toHaveBeenCalledTimes(1);
    const [cmd, args] = raw.mock.calls[0];
    expect(cmd).toBe("client_log");
    expect((args as { message: string }).message.length).toBe(500);
    await Promise.resolve();
  });

  it("reports uncaught errors and rejected promises, and can be removed", () => {
    const report = vi.fn();
    const off = installGlobalHandlers(window, report);
    window.dispatchEvent(new ErrorEvent("error", { message: "kaput", filename: "http://x/app.js", lineno: 7 }));
    expect(report).toHaveBeenCalledWith("error", "window", "kaput (app.js:7)");
    const rejection = new Event("unhandledrejection") as Event & { reason?: unknown };
    rejection.reason = new Error("late failure");
    window.dispatchEvent(rejection);
    expect(report).toHaveBeenCalledWith("error", "promise", "late failure");
    off();
    window.dispatchEvent(new ErrorEvent("error", { message: "ignored" }));
    expect(report).toHaveBeenCalledTimes(2);
  });
});
