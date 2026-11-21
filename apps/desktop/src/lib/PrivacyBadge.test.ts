import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import PrivacyBadge from "./PrivacyBadge.svelte";

afterEach(cleanup);

describe("PrivacyBadge", () => {
  it("says private when nothing outside is in use", async () => {
    render(PrivacyBadge, { invoke: async () => ({ private: true, items: [] }) });
    expect(screen.getByTestId("privacy-badge").textContent).toMatch(/nothing leaves this computer/);
  });
  it("warns when an outside service is in use", async () => {
    render(PrivacyBadge, { invoke: async () => ({ private: false, items: [] }) });
    expect((await screen.findByText(/Connected to an outside service/)).textContent).toBeTruthy();
  });
  it("keeps the private wording when status can't be read", async () => {
    render(PrivacyBadge, { invoke: async () => { throw new Error("no engine"); } });
    expect(screen.getByTestId("privacy-badge").textContent).toMatch(/Private mode/);
  });
  it("opens the privacy page when clicked", async () => {
    const onclick = vi.fn();
    render(PrivacyBadge, { invoke: async () => ({ private: true, items: [] }), onclick });
    await fireEvent.click(screen.getByTestId("privacy-badge"));
    expect(onclick).toHaveBeenCalled();
  });
});
