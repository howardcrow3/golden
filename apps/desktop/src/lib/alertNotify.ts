import type { Alert } from "./api";

const KEY = "golden.notifiedAlerts";

function read(): string[] {
  try {
    const v = JSON.parse(localStorage.getItem(KEY) ?? "[]");
    return Array.isArray(v) ? v.filter((x) => typeof x === "string") : [];
  } catch {
    return [];
  }
}

/**
 * How many warn/danger heads-ups have not been announced before. Remembers what it has counted,
 * so each one raises a single notification, and forgets alerts that have cleared so they can
 * notify again if they come back.
 */
export function newlyUrgent(items: Alert[]): number {
  const urgent = items.filter((a) => a.severity === "warn" || a.severity === "danger").map((a) => a.key);
  const seen = new Set(read());
  const fresh = urgent.filter((k) => !seen.has(k)).length;
  try {
    localStorage.setItem(KEY, JSON.stringify(urgent));
  } catch {
    // storage unavailable: worst case is a repeat notification
  }
  return fresh;
}
