import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import RetireWhen from "./RetireWhen.svelte";

afterEach(cleanup);

describe("RetireWhen", () => {
  it("names the earliest age and compares it with the one set", async () => {
    const invoke = vi.fn(async () => ({ target: 0.85, age: 60, currentAge: 65, tries: [{ age: 59, successRate: 0.7 }, { age: 60, successRate: 0.87 }] }));
    render(RetireWhen, { invoke });
    await fireEvent.click(screen.getByText("Find my earliest age"));
    await waitFor(() => expect(screen.getByTestId("when-answer").textContent).toContain("5 years earlier"));
    expect(invoke).toHaveBeenCalledWith("retire_when", { target: 0.85 });
    expect(screen.getAllByTestId("when-try")).toHaveLength(2);
  });
  it("says so when no age works", async () => {
    render(RetireWhen, { invoke: async () => ({ target: 0.85, age: null, currentAge: 65, tries: [] }) });
    await fireEvent.click(screen.getByText("Find my earliest age"));
    await waitFor(() => expect(screen.getByTestId("when-answer").textContent).toContain("No retirement age"));
  });
  it("shows an error", async () => {
    render(RetireWhen, { invoke: async () => { throw new Error("Add a person first."); } });
    await fireEvent.click(screen.getByText("Find my earliest age"));
    await waitFor(() => expect(screen.getByRole("alert").textContent).toContain("Add a person"));
  });
});
