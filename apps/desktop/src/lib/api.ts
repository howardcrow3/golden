export interface TaxRequest {
  wagesCents: number;
  filing: "single" | "married_joint" | "married_separate" | "head_of_household";
  state: string;
}
export interface TaxEstimate {
  federalCents: number;
  stateCents: number;
  ficaCents: number;
  totalCents: number;
  effectiveRate: number;
  marginalRate: number;
  stateIsEstimate: boolean;
}

type Invoke = (cmd: string, args?: Record<string, unknown>) => Promise<unknown>;

/** True when running inside the Tauri shell (as opposed to a plain browser during UI development). */
export function inTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

async function tauriInvoke(): Promise<Invoke> {
  const mod = await import("@tauri-apps/api/core");
  return mod.invoke as Invoke;
}

export async function estimateTax(req: TaxRequest, invoke?: Invoke): Promise<TaxEstimate> {
  const call = invoke ?? (inTauri() ? await tauriInvoke() : null);
  if (!call) throw new Error("The tax engine runs inside the Golden desktop app.");
  return (await call("estimate_tax", { req })) as TaxEstimate;
}
