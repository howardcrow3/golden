import { afterEach, describe, expect, it, vi } from "vitest";
import { columnsChart } from "./tbchart";

afterEach(() => {
  delete (globalThis as { TBChart?: unknown }).TBChart;
});

describe("columnsChart", () => {
  it("draws with the turtlebox kit and redraws on update", () => {
    const columns = vi.fn(() => ({ redraw() {} }));
    (globalThis as { TBChart?: unknown }).TBChart = { columns };
    const node = document.createElement("div");
    const o = { cats: ["Jan"], series: [{ name: "Income", color: "var(--s1)", values: [10] }] };
    const a = columnsChart(node, o);
    expect(columns).toHaveBeenCalledWith(node, o);
    a.update({ ...o, cats: ["Feb"] });
    expect(columns).toHaveBeenCalledTimes(2);
  });

  it("does nothing when the kit is missing", () => {
    const node = document.createElement("div");
    expect(() => columnsChart(node, { cats: [], series: [] })).not.toThrow();
  });
});
