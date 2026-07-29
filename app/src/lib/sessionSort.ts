import type { SessionSummary, TokenBreakdown } from "../types/dashboard";

export type SessionSort = "recent" | "tokensDesc" | "tokensAsc";

function totalTokens(tokens: TokenBreakdown) {
  return tokens.inputTokens + tokens.outputTokens;
}

export function sortSessions(
  sessions: SessionSummary[],
  sort: SessionSort,
): SessionSummary[] {
  return [...sessions].sort((left, right) => {
    if (sort === "recent") return right.lastActiveAt - left.lastActiveAt;
    const delta =
      totalTokens(left.monthlyTokens ?? left.tokens) -
      totalTokens(right.monthlyTokens ?? right.tokens);
    if (delta === 0) return right.lastActiveAt - left.lastActiveAt;
    return sort === "tokensAsc" ? delta : -delta;
  });
}
