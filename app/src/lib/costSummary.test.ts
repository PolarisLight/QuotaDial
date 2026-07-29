import { describe, expect, test } from "vitest";
import type { MonthlyUsageSummary, SessionSummary } from "../types/dashboard";
import { summarizeMonthlyValue, summarizeSessionCosts } from "./costSummary";

function session(cost: number | null, unpricedTokens = 0): SessionSummary {
  return {
    sessionId: crypto.randomUUID(),
    title: "session",
    projectPath: null,
    lastActiveAt: 0,
    primaryModel: null,
    tokens: {
      inputTokens: 0,
      cachedInputTokens: 0,
      outputTokens: 0,
      reasoningOutputTokens: 0,
    },
    equivalentCostUsd: cost,
    pricedTokens: 0,
    unpricedTokens,
    childSessionCount: 0,
  };
}

describe("session cost summary", () => {
  test("sums known session costs and marks partial pricing as a lower bound", () => {
    expect(
      summarizeSessionCosts([
        session(0.42),
        session(1.08, 300),
        session(null, 500),
      ]),
    ).toEqual({
      value: "≥ US$1.50",
      note: "本机 3 个会话 · 含未定价 Token",
    });
  });

  test("uses a clear empty state before local sessions are available", () => {
    expect(summarizeSessionCosts([])).toEqual({
      value: "—",
      note: "尚无本机会话",
    });
  });
});

describe("monthly value summary", () => {
  const monthly: MonthlyUsageSummary = {
    periodStart: 0,
    periodEnd: 1,
    tokens: {
      inputTokens: 1_000,
      cachedInputTokens: 0,
      outputTokens: 200,
      reasoningOutputTokens: 0,
    },
    equivalentCostUsd: 30,
    pricedTokens: 1_000,
    unpricedTokens: 200,
  };

  test("calculates payback ratio from monthly value and subscription price", () => {
    expect(summarizeMonthlyValue(monthly, 20)).toEqual({
      value: "≥ US$30.00",
      note: "本月含未定价 Token",
      roi: "≥ 150.0%",
      roiNote: "按 US$20.00/月",
    });
  });
});
