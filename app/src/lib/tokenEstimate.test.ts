import { describe, expect, test } from "vitest";
import { estimateCurrentDayTokens } from "./tokenEstimate";

const hour = 3_600;

describe("estimateCurrentDayTokens", () => {
  test("infers today's tokens from quota burn calibrated by published cycle data", () => {
    const cycleStart = new Date(2026, 6, 29, 12).getTime() / 1_000;
    const todayStart = new Date(2026, 6, 30).getTime() / 1_000;
    const observedAt = todayStart + 12 * hour;

    expect(
      estimateCurrentDayTokens({
        buckets: [{ startDate: "2026-07-29", tokens: 240_000 }],
        history: [
          { observedAt: cycleStart + 6 * hour, remainingPercent: 95 },
          { observedAt: todayStart, remainingPercent: 90 },
        ],
        quota: {
          resetsAt: cycleStart + 7 * 86_400,
          windowDurationMins: 10_080,
          remainingPercent: 85,
        },
        observedAt,
      }),
    ).toBe(60_000);
  });

  test("does not estimate when the account already published today's bucket", () => {
    const todayStart = new Date(2026, 6, 30).getTime() / 1_000;

    expect(
      estimateCurrentDayTokens({
        buckets: [
          { startDate: "2026-07-29", tokens: 240_000 },
          { startDate: "2026-07-30", tokens: 100_000 },
        ],
        history: [
          { observedAt: todayStart, remainingPercent: 90 },
          { observedAt: todayStart + 12 * hour, remainingPercent: 85 },
        ],
        quota: {
          resetsAt: todayStart + 7 * 86_400,
          windowDurationMins: 10_080,
          remainingPercent: 85,
        },
        observedAt: todayStart + 12 * hour,
      }),
    ).toBeNull();
  });
});
