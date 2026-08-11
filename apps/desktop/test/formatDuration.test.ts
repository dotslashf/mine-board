import { describe, expect, test } from "bun:test";

import { formatDuration } from "../src/components/SoundButton";

describe("formatDuration", () => {
  test("zero and small values", () => {
    expect(formatDuration(0)).toBe("0:00");
    expect(formatDuration(1)).toBe("0:01");
    expect(formatDuration(999)).toBe("0:01");
  });

  test("rounds up to the next second", () => {
    expect(formatDuration(1000)).toBe("0:01");
    expect(formatDuration(1001)).toBe("0:02");
    expect(formatDuration(59999)).toBe("1:00");
  });

  test("minutes and padding", () => {
    expect(formatDuration(60000)).toBe("1:00");
    expect(formatDuration(61000)).toBe("1:01");
    expect(formatDuration(3600000)).toBe("60:00");
  });

  test("negative values clamp to zero", () => {
    expect(formatDuration(-500)).toBe("0:00");
  });
});
