import type { MonthlyUsageSummary, SessionSummary } from "../types/dashboard";

export interface SessionCostSummary {
  value: string;
  note: string;
}

export interface MonthlyValueSummary {
  value: string;
  note: string;
  roi: string;
  roiNote: string;
}

export function summarizeMonthlyValue(
  summary: MonthlyUsageSummary,
  monthlySubscriptionUsd: number,
): MonthlyValueSummary {
  const hasUnpricedTokens = summary.unpricedTokens > 0;
  if (summary.equivalentCostUsd === null) {
    return {
      value: "费用待定",
      note: "本月模型尚未定价",
      roi: "—",
      roiNote: `按 US$${monthlySubscriptionUsd.toFixed(2)}/月`,
    };
  }
  const prefix = hasUnpricedTokens ? "≥" : "≈";
  const ratio = (summary.equivalentCostUsd / monthlySubscriptionUsd) * 100;
  return {
    value: `${prefix} US$${summary.equivalentCostUsd.toFixed(2)}`,
    note: hasUnpricedTokens ? "本月含未定价 Token" : "本月累计等效价值",
    roi: `${hasUnpricedTokens ? "≥ " : ""}${ratio.toFixed(1)}%`,
    roiNote: `按 US$${monthlySubscriptionUsd.toFixed(2)}/月`,
  };
}

export function summarizeSessionCosts(
  sessions: SessionSummary[],
): SessionCostSummary {
  if (sessions.length === 0) {
    return { value: "—", note: "尚无本机会话" };
  }

  const knownCosts = sessions
    .map(session => session.equivalentCostUsd)
    .filter((cost): cost is number => cost !== null);
  const total = knownCosts.reduce((sum, cost) => sum + cost, 0);
  const hasUnpricedTokens = sessions.some(session => session.unpricedTokens > 0);

  if (knownCosts.length === 0) {
    return {
      value: "费用待定",
      note: `本机 ${sessions.length} 个会话 · 模型尚未定价`,
    };
  }

  return {
    value: `${hasUnpricedTokens ? "≥" : "≈"} US$${total.toFixed(2)}`,
    note: `本机 ${sessions.length} 个会话${hasUnpricedTokens ? " · 含未定价 Token" : ""}`,
  };
}
