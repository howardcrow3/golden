import { afterEach, describe, expect, it } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import Feedback from "./Feedback.svelte";

afterEach(cleanup);

describe("Feedback", () => {
  it("waits for stars or words before it can send", async () => {
    render(Feedback, { invoke: async () => undefined });
    const send = screen.getByRole("button", { name: "Send" }) as HTMLButtonElement;
    expect(send.disabled).toBe(true);
    await fireEvent.click(screen.getByRole("radio", { name: "4 stars" }));
    expect(send.disabled).toBe(false);
  });

  it("sends the rating, category, trimmed words and email, then thanks the person", async () => {
    const calls: [string, unknown][] = [];
    const invoke = async (cmd: string, args?: Record<string, unknown>) => void calls.push([cmd, args]);
    render(Feedback, { invoke });
    await fireEvent.click(screen.getByRole("radio", { name: "2 stars" }));
    await fireEvent.click(screen.getByLabelText("Problem"));
    await fireEvent.input(screen.getByLabelText("Your feedback"), { target: { value: "  Import fails on my bank's CSV  " } });
    await fireEvent.input(screen.getByLabelText(/Email/), { target: { value: " me@example.com " } });
    await fireEvent.click(screen.getByRole("button", { name: "Send" }));
    expect(await screen.findByTestId("feedback-thanks")).toBeTruthy();
    expect(calls).toEqual([
      ["send_feedback", { input: { rating: 2, category: "issue", message: "Import fails on my bank's CSV", email: "me@example.com" } }],
    ]);
  });

  it("shows the service's error and keeps what was typed", async () => {
    render(Feedback, { invoke: async () => { throw new Error("You've sent a lot of feedback recently. Please try again later."); } });
    await fireEvent.input(screen.getByLabelText("Your feedback"), { target: { value: "Hello" } });
    await fireEvent.click(screen.getByRole("button", { name: "Send" }));
    expect((await screen.findByRole("alert")).textContent).toMatch(/try again later/);
    expect((screen.getByLabelText("Your feedback") as HTMLTextAreaElement).value).toBe("Hello");
  });
});
