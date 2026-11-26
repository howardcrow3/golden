import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import SsaStatement from "./SsaStatement.svelte";

afterEach(cleanup);

const summary = { monthlyAt62Cents: 174_200, monthlyAtFullRetirementCents: 249_000, fullRetirementAge: 67, monthlyAt70Cents: null };

function mock() {
  return vi.fn(async (cmd: string) => (cmd === "read_ssa_statement" ? summary : cmd === "apply_ssa_statement" ? 2_988_000 : undefined));
}

async function readIt(invoke: ReturnType<typeof mock>) {
  render(SsaStatement, { invoke });
  await fireEvent.input(screen.getByLabelText("Statement text"), { target: { value: "Social Security Statement ..." } });
  await fireEvent.click(screen.getByText("Read statement"));
  await waitFor(() => screen.getByTestId("ssa-summary"));
}

describe("SsaStatement", () => {
  it("shows the amounts that were found and skips the ones that weren't", async () => {
    await readIt(mock());
    const t = screen.getByTestId("ssa-summary").textContent ?? "";
    expect(t).toContain("At 62: $1,742 a month");
    expect(t).toContain("(67)");
    expect(t).toContain("$2,490");
    expect(t).not.toContain("At 70");
  });

  it("uses the chosen claim age for the chosen person", async () => {
    const invoke = mock();
    await readIt(invoke);
    await fireEvent.change(screen.getByLabelText("Claim age"), { target: { value: "62" } });
    await fireEvent.input(screen.getByLabelText("Person number"), { target: { value: "2" } });
    await fireEvent.click(screen.getByText("Use in my plan"));
    await waitFor(() => screen.getByTestId("ssa-done"));
    expect(invoke).toHaveBeenCalledWith("apply_ssa_statement", { text: "Social Security Statement ...", person: 1, claim: "62" });
    expect(screen.getByTestId("ssa-done").textContent).toContain("$29,880 a year for person 2");
  });

  it("rejects a bad person number without calling the app", async () => {
    const invoke = mock();
    await readIt(invoke);
    await fireEvent.input(screen.getByLabelText("Person number"), { target: { value: "0" } });
    await fireEvent.click(screen.getByText("Use in my plan"));
    expect(screen.getByRole("alert").textContent).toContain("Enter 1");
    expect(invoke).toHaveBeenCalledTimes(1);
  });

  it("shows an error from the app", async () => {
    const invoke = vi.fn(async () => {
      throw new Error("This doesn't look like a Social Security Statement.");
    });
    render(SsaStatement, { invoke });
    await fireEvent.input(screen.getByLabelText("Statement text"), { target: { value: "hello" } });
    await fireEvent.click(screen.getByText("Read statement"));
    await waitFor(() => screen.getByRole("alert"));
    expect(screen.getByRole("alert").textContent).toContain("doesn't look like");
  });
});
