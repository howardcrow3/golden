import { afterEach, describe, expect, it } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import Ask from "./Ask.svelte";

afterEach(cleanup);

const answer = {
  understood: true,
  text: "Last month you spent $1,531.00.",
  facts: [{ label: "Spent", cents: 153100 }],
  behind: [{ id: 1, day: "2026-10-12", payee: "LANDLORD RENT", memo: "", amountCents: -150000, category: "Rent", account: "Checking" }],
};

describe("Ask", () => {
  it("is disabled until there is a question", () => {
    render(Ask, { invoke: async () => answer });
    expect((screen.getByRole("button", { name: "Ask" }) as HTMLButtonElement).disabled).toBe(true);
  });
  it("sends the question and shows the answer with the lines behind it", async () => {
    const calls: [string, Record<string, unknown> | undefined][] = [];
    render(Ask, { invoke: async (c: string, a?: Record<string, unknown>) => (calls.push([c, a]), answer) });
    await fireEvent.input(screen.getByLabelText("Question"), { target: { value: "How much did I spend last month?" } });
    await fireEvent.click(screen.getByRole("button", { name: "Ask" }));
    const card = await screen.findByTestId("answer");
    expect(card.textContent).toMatch(/Last month you spent \$1,531\.00/);
    expect(screen.getByTestId("behind").textContent).toMatch(/LANDLORD RENT.*-\$1,500\.00.*Rent/);
    expect(calls[0]).toEqual(["ask", { question: "How much did I spend last month?" }]);
  });
  it("example questions run straight away", async () => {
    const calls: string[] = [];
    render(Ask, { invoke: async (c: string, a?: Record<string, unknown>) => (calls.push(String(a?.question)), answer) });
    await fireEvent.click(screen.getByRole("button", { name: "What is my net worth?" }));
    await screen.findByTestId("answer");
    expect(calls).toEqual(["What is my net worth?"]);
  });
  it("shows an error", async () => {
    render(Ask, { invoke: async () => { throw new Error("locked"); } });
    await fireEvent.click(screen.getByRole("button", { name: "What is my net worth?" }));
    expect((await screen.findByRole("alert")).textContent).toMatch(/locked/);
  });
});
