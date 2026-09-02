import { describe, expect, test } from "vitest";

import { formatMomentTime } from "./format";

describe("formatMomentTime", () => {
  test("keeps tenths for short GIF ranges", () => {
    expect(formatMomentTime(12.4)).toBe("0:12.4");
  });

  test("carries rounded seconds into minutes and hours", () => {
    expect(formatMomentTime(59.96)).toBe("1:00.0");
    expect(formatMomentTime(3_600)).toBe("1:00:00.0");
  });

  test("bounds negative values and rejects non-finite values", () => {
    expect(formatMomentTime(-2)).toBe("0:00.0");
    expect(formatMomentTime(Number.NaN)).toBe("");
  });
});
