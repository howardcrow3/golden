import { beforeEach, describe, expect, it } from "vitest";
import { newlyUrgent } from "./alertNotify";
import type { Alert } from "./api";

const a = (key: string, severity: Alert["severity"]) => ({ key, severity, message: key }) as Alert;

describe("newlyUrgent", () => {
  beforeEach(() => localStorage.clear());

  it("counts only warn and danger alerts", () => {
    expect(newlyUrgent([a("x", "good"), a("y", "info"), a("z", "warn"), a("w", "danger")])).toBe(2);
  });

  it("announces each alert once", () => {
    expect(newlyUrgent([a("z", "warn")])).toBe(1);
    expect(newlyUrgent([a("z", "warn")])).toBe(0);
    expect(newlyUrgent([a("z", "warn"), a("q", "danger")])).toBe(1);
  });

  it("announces again after an alert clears and returns", () => {
    newlyUrgent([a("z", "warn")]);
    newlyUrgent([]);
    expect(newlyUrgent([a("z", "warn")])).toBe(1);
  });

  it("survives broken storage", () => {
    localStorage.setItem("golden.notifiedAlerts", "not json");
    expect(newlyUrgent([a("z", "warn")])).toBe(1);
  });
});
