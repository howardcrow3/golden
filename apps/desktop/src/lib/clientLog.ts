/** Client-side logging: failures and crashes go to Golden's local log file through the shell.
 *  Only the command name and the error text are sent, never the arguments, and the shell scrubs
 *  amounts, long numbers, e-mail addresses and keys again before writing. */
export type Level = "error" | "warn" | "info" | "debug";
export type Invoke = (cmd: string, args?: Record<string, unknown>) => Promise<unknown>;
export type Report = (level: Level, source: string, message: string) => void;

export function errorText(e: unknown): string {
  if (e instanceof Error) return e.message;
  if (typeof e === "string") return e;
  try {
    return JSON.stringify(e) ?? String(e);
  } catch {
    return String(e);
  }
}

/** A reporter that sends lines to the shell's `client_log` command and never throws. */
export function shellReporter(raw: Invoke): Report {
  return (level, source, message) => {
    raw("client_log", { level, source, message: message.slice(0, 500) }).catch(() => {});
  };
}

/** Wrap an invoke so every failed command is logged (then rethrown unchanged). */
export function wrapInvoke(raw: Invoke, report: Report): Invoke {
  return async (cmd, args) => {
    try {
      return await raw(cmd, args);
    } catch (e) {
      report("error", cmd, errorText(e));
      throw e;
    }
  };
}

/** Log errors that nothing caught: script errors and rejected promises. */
export function installGlobalHandlers(target: Window, report: Report): () => void {
  const onError = (ev: ErrorEvent) => {
    const where = ev.filename ? ` (${ev.filename.split("/").pop()}:${ev.lineno})` : "";
    report("error", "window", `${ev.message || "script error"}${where}`);
  };
  const onRejection = (ev: PromiseRejectionEvent) => report("error", "promise", errorText(ev.reason));
  target.addEventListener("error", onError);
  target.addEventListener("unhandledrejection", onRejection);
  return () => {
    target.removeEventListener("error", onError);
    target.removeEventListener("unhandledrejection", onRejection);
  };
}
