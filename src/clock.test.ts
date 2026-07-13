import { describe, expect, it } from "vitest";

import { formatClockTime } from "./clock";

describe("formatClockTime", () => {
  it("formats the 12-hour clock tokens used by the panel", () => {
    const value = formatClockTime(new Date("2026-07-13T00:05:00"), "%I:%M %p");

    expect(value).toBe("12:05 AM");
  });

  it("formats afternoon times", () => {
    const value = formatClockTime(new Date("2026-07-13T15:09:00"), "%I:%M %p");

    expect(value).toBe("03:09 PM");
  });
});
