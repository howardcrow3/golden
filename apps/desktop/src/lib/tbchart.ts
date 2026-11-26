import "../turtlebox/tb-charts.js";

type Series = { name: string; color: string; values: number[] };
export type ColumnsOptions = { cats: string[]; heads?: string[]; series: Series[]; height?: number; label?: string };

type Kit = { columns: (root: HTMLElement, o: ColumnsOptions) => { redraw: () => void } };
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
