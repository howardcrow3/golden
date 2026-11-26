import "../turtlebox/tb-charts.js";

type Series = { name: string; color: string; values: number[] };
export type ColumnsOptions = { cats: string[]; heads?: string[]; series: Series[]; height?: number; label?: string };

export type LineOptions = {
  cats: string[];
  heads?: string[];
  series: Series[];
  bands?: { lo: number[]; hi: number[]; opacity?: number; color?: string }[];
  height?: number;
  label?: string;
  zero?: boolean;
};

type Kit = {
  columns: (root: HTMLElement, o: ColumnsOptions) => { redraw: () => void };
  line: (root: HTMLElement, o: LineOptions) => { redraw: () => void };
};
const kit = (): Kit | undefined => (globalThis as unknown as { TBChart?: Kit }).TBChart;

/** Svelte action: draws grouped columns with the turtlebox chart kit (hover, legend, resize). */
export function columnsChart(node: HTMLElement, options: ColumnsOptions) {
  const draw = (o: ColumnsOptions) => kit()?.columns(node, o);
  draw(options);
  return {
    update(o: ColumnsOptions) {
      draw(o);
    },
  };
}

/** Svelte action: draws lines (with optional shaded bands) with the turtlebox chart kit. */
export function lineChart(node: HTMLElement, options: LineOptions) {
  const draw = (o: LineOptions) => kit()?.line(node, o);
  draw(options);
  return {
    update(o: LineOptions) {
      draw(o);
    },
  };
}
