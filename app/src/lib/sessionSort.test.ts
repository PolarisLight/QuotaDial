import { describe, expect, test } from "vitest";
import type { SessionSummary } from "../types/dashboard";
import { sortSessions } from "./sessionSort";

function session(
  sessionId: string,
  lastActiveAt: number,
  inputTokens: number,
  outputTokens: number,
  monthlyInputTokens = inputTokens,
): SessionSummary {
  return {
    sessionId,
    title: sessionId,
    projectPath: null,
    lastActiveAt,
    primaryModel: null,
    tokens: {
      inputTokens,
      cachedInputTokens: 0,
      outputTokens,
      reasoningOutputTokens: 0,
    },
    monthlyTokens: {
      inputTokens: monthlyInputTokens,
      cachedInputTokens: 0,
      outputTokens: 0,
      reasoningOutputTokens: 0,
    },
    equivalentCostUsd: null,
    monthlyEquivalentCostUsd: null,
    pricedTokens: 0,
    unpricedTokens: 0,
    monthlyPricedTokens: 0,
    monthlyUnpricedTokens: 0,
    childSessionCount: 0,
  };
}

describe("session sorting", () => {
  const sessions = [
    session("older-large", 100, 1_000, 200),
    session("newer-small", 200, 100, 20),
  ];

  test("defaults to most recently active first", () => {
    expect(sortSessions(sessions, "recent").map(item => item.sessionId)).toEqual(
      ["newer-small", "older-large"],
    );
  });

  test("sorts Token totals in both directions", () => {
    expect(
      sortSessions(sessions, "tokensDesc").map(item => item.sessionId),
    ).toEqual(["older-large", "newer-small"]);
    expect(
      sortSessions(sessions, "tokensAsc").map(item => item.sessionId),
    ).toEqual(["newer-small", "older-large"]);
  });

  test("sorts by this month's tokens instead of historical totals", () => {
    const crossMonth = [
      session("large-history-small-month", 200, 10_000, 0, 10),
      session("small-history-large-month", 100, 100, 0, 90),
    ];
    expect(
      sortSessions(crossMonth, "tokensDesc").map(item => item.sessionId),
    ).toEqual(["small-history-large-month", "large-history-small-month"]);
  });

  test("does not mutate the backend array", () => {
    const original = [...sessions];
    sortSessions(sessions, "tokensDesc");
    expect(sessions).toEqual(original);
  });
});
