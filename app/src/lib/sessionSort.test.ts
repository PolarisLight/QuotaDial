import { describe, expect, test } from "vitest";
import type { SessionSummary } from "../types/dashboard";
import { sortSessions } from "./sessionSort";

function session(
  sessionId: string,
  lastActiveAt: number,
  inputTokens: number,
  outputTokens: number,
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
    equivalentCostUsd: null,
    pricedTokens: 0,
    unpricedTokens: 0,
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

  test("does not mutate the backend array", () => {
    const original = [...sessions];
    sortSessions(sessions, "tokensDesc");
    expect(sessions).toEqual(original);
  });
});
