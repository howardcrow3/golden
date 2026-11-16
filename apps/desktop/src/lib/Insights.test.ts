import { cleanup, render, screen, waitFor } from "@testing-library/svelte";
import { afterEach, describe, expect, it } from "vitest";
import Insights from "./Insights.svelte";

describe("Insights", () => {
  afterEach(cleanup);

  it("lists each observation in order", async () => {
    const invoke = async () => [
      { kind: "savings", text: "In October you kept $3,000 of the $5,000 that came in (60%)." },
      { kind: "needs_category", text: "2 transactions in October still need a category." },
    ];
    render(Insights, { invoke });
    await waitFor(() => expect(screen.getAllByRole("listitem")).toHaveLength(2));
    expect(screen.getAllByRole("listitem")[0].textContent).toContain("kept $3,000");
  });

  it("is hidden with nothing to say or in a plain browser", async () => {
    render(Insights, { invoke: async () => [] });
    await new Promise((r) => setTimeout(r, 20));
    expect(screen.queryByTestId("insights")).toBeNull();
    cleanup();
    render(Insights, {});
    await new Promise((r) => setTimeout(r, 20));
    expect(screen.queryByTestId("insights")).toBeNull();
  });
});
