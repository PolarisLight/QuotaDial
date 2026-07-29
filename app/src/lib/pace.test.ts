import { describe, expect, test } from "vitest";
import {
  classifySuggestedPace,
  recentRateCopy,
  suggestedPace,
} from "./pace";

describe("quota pace presentation", () => {
  test("calculates guidance from remaining quota and remaining time", () => {
    const day = 86_400;
    const start = 1_800_000_000;
    expect(
      suggestedPace({
        remainingPercent: 51,
        observedAt: start + 14 * 3_600,
        periodStart: start,
        resetsAt: start + 7 * day,
      }),
    ).toEqual({
      ratioPercent: 56,
      status: "fast",
      copy: "明显偏快 · 建议降至 56%",
    });
  });

  test.each([
    [84, "fast"],
    [85, "normal"],
    [115, "normal"],
    [116, "slow"],
  ] as const)("classifies %s percent as %s", (ratio, status) => {
    expect(classifySuggestedPace(ratio)).toBe(status);
  });

  test("labels the historical slope as a recent rate", () => {
    expect(recentRateCopy({ percentPerDay: 95.8 })).toBe(
      "近期消耗率 · 95.8%/天",
    );
  });
});
