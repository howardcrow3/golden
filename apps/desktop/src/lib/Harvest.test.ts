import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import Harvest from "./Harvest.svelte";

afterEach(cleanup);
const out = { zeroRateRoomCents: 3_000_000, taxSavedCents: 450_000, acaCliffCents: 6_260_000, acaRoomCents: 1_000_000, gainCrossesAcaCliff: false };

describe("Harvest", () => {
  it("shows the 0% room and the staying-under message", async () => {
    const invoke = vi.fn(async () => out);
    render(Harvest, { invoke });
    await fireEvent.click(screen.getByText("Find my room"));
    await waitFor(() => expect(screen.getByTestId("harvest-room").textContent).toContain("$30,000"));
    expect(invoke).toHaveBeenCalledWith("harvest_plan", { filing: "single", incomeCents: 3_000_000, gainsCents: 0, householdSize: 1 });
    expect(screen.getByTestId("harvest-aca").textContent).toContain("stay $10,000 under");
  });
  it("warns when the gain would cross the cliff", async () => {
    render(Harvest, { invoke: async () => ({ ...out, acaRoomCents: -500_000, gainCrossesAcaCliff: true }) });
    await fireEvent.click(screen.getByText("Find my room"));
    await waitFor(() => expect(screen.getByTestId("harvest-aca").textContent).toContain("$5,000 over"));
  });
  it("validates amounts and shows engine errors", async () => {
    const invoke = vi.fn(async () => { throw new Error("Household size should be between 1 and 10."); });
    render(Harvest, { invoke });
    await fireEvent.input(screen.getByLabelText("Harvest income"), { target: { value: "x" } });
    await fireEvent.click(screen.getByText("Find my room"));
    expect((await screen.findByRole("alert")).textContent).toContain("dollar amounts");
    expect(invoke).not.toHaveBeenCalled();
    await fireEvent.input(screen.getByLabelText("Harvest income"), { target: { value: "1000" } });
    await fireEvent.click(screen.getByText("Find my room"));
    await waitFor(() => expect(screen.getByRole("alert").textContent).toContain("Household size"));
  });
});
