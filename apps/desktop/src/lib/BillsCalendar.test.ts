import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import BillsCalendar from "./BillsCalendar.svelte";

afterEach(cleanup);

const today = new Date("2026-11-26T12:00:00Z");
const rent = { payee: "Landlord", cadence: "monthly", typicalCents: -250_000, timesSeen: 6, lastSeen: "2026-10-28", nextExpected: "2026-11-28", yearlyCents: -3_000_000 };
const pay = { payee: "Employer", cadence: "monthly", typicalCents: 500_000, timesSeen: 6, lastSeen: "2026-10-30", nextExpected: "2026-11-30", yearlyCents: 6_000_000 };

const mock = (rows: unknown[]) => vi.fn(async () => rows);

describe("BillsCalendar", () => {
  it("shows the current month with bills and pay on their days", async () => {
    render(BillsCalendar, { invoke: mock([rent, pay]), today });
    await waitFor(() => screen.getByTestId("bills-calendar"));
    expect(screen.getByTestId("calendar-title").textContent).toBe("November 2026");
    expect(screen.getAllByTestId("calendar-busy").length).toBe(2);
    expect(screen.getByLabelText("2026-11-28, 1 items").textContent).toContain("-$2,500");
  });

  it("lists the items on a clicked day", async () => {
    render(BillsCalendar, { invoke: mock([rent, pay]), today });
    await waitFor(() => screen.getByTestId("bills-calendar"));
    await fireEvent.click(screen.getByLabelText("2026-11-30, 1 items"));
    expect(screen.getByTestId("calendar-detail").textContent).toContain("Employer");
  });

  it("moves to the next month, where the repeats land again, and back", async () => {
    render(BillsCalendar, { invoke: mock([rent]), today });
    await waitFor(() => screen.getByTestId("bills-calendar"));
    const prev = screen.getByLabelText("Previous month") as HTMLButtonElement;
    expect(prev.disabled).toBe(true);
    await fireEvent.click(screen.getByLabelText("Next month"));
    expect(screen.getByTestId("calendar-title").textContent).toBe("December 2026");
    expect(screen.getAllByTestId("calendar-busy").length).toBe(1);
    await fireEvent.click(prev);
    expect(screen.getByTestId("calendar-title").textContent).toBe("November 2026");
  });

  it("is hidden when nothing repeats", async () => {
    render(BillsCalendar, { invoke: mock([]), today });
    await new Promise((r) => setTimeout(r, 20));
    expect(screen.queryByTestId("bills-calendar")).toBeNull();
  });
});
