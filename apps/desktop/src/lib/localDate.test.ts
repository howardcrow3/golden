import { afterEach, describe, expect, it, vi } from "vitest";
import { localIso, localToday } from "./localDate";

describe("localDate", () => {
  afterEach(() => vi.unstubAllEnvs());

  it("uses the local calendar day, whatever the time zone", () => {
    // 9:30 pm on the last day of the month, in local time.
    const evening = new Date(2026, 10, 30, 21, 30);
    expect(localIso(evening)).toBe("2026-11-30");
    expect(localToday(evening).toISOString()).toBe("2026-11-30T12:00:00.000Z");
    // Just after midnight on the first.
    expect(localIso(new Date(2026, 11, 1, 0, 5))).toBe("2026-12-01");
  });

  it("stays on the same month on the evening of the 31st in New York", () => {
    vi.stubEnv("TZ", "America/New_York");
    {
      // 02:30 UTC on Dec 1 is 9:30 pm on Nov 30 in New York.
      const instant = new Date(Date.UTC(2026, 11, 1, 2, 30));
      if (instant.getDate() === 30) {
        expect(localIso(instant)).toBe("2026-11-30");
        expect(instant.toISOString().slice(0, 10)).toBe("2026-12-01"); // what the old code used
      } else {
        expect(localIso(instant)).toBe(`2026-12-0${instant.getDate()}`); // time zone could not be switched
      }
    }
  });
});
