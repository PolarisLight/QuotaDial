import type { SessionSummary, TokenBreakdown } from "../types/dashboard";
import type { SessionSort } from "./sessionSort";

export interface SessionProjectGroup {
  id: string;
  name: string;
  projectPath: string | null;
  sessions: SessionSummary[];
  lastActiveAt: number;
  models: string[];
  tokens: TokenBreakdown;
  monthlyTokens: TokenBreakdown;
  equivalentCostUsd: number | null;
  monthlyEquivalentCostUsd: number | null;
  unpricedTokens: number;
  monthlyUnpricedTokens: number;
  childSessionCount: number;
}

function emptyTokens(): TokenBreakdown {
  return {
    inputTokens: 0,
    cachedInputTokens: 0,
    outputTokens: 0,
    reasoningOutputTokens: 0,
  };
}

function addTokens(target: TokenBreakdown, value: TokenBreakdown) {
  target.inputTokens += value.inputTokens;
  target.cachedInputTokens += value.cachedInputTokens;
  target.outputTokens += value.outputTokens;
  target.reasoningOutputTokens += value.reasoningOutputTokens;
}

function tokenTotal(tokens: TokenBreakdown) {
  return tokens.inputTokens + tokens.outputTokens;
}

function normalizedProject(path: string | null) {
  if (!path) return "__unknown__";
  return path
    .replace(/[\\/]+/g, "/")
    .replace(/\/$/, "")
    .toLocaleLowerCase();
}

export function projectName(path: string | null) {
  return path?.split(/[\\/]/).filter(Boolean).at(-1) ?? "未命名项目";
}

export function groupSessionsByProject(
  sessions: SessionSummary[],
  sort: SessionSort,
): SessionProjectGroup[] {
  const groups = new Map<string, SessionProjectGroup>();

  for (const session of sessions) {
    const id = normalizedProject(session.projectPath);
    let group = groups.get(id);
    if (!group) {
      group = {
        id,
        name: projectName(session.projectPath),
        projectPath: session.projectPath,
        sessions: [],
        lastActiveAt: 0,
        models: [],
        tokens: emptyTokens(),
        monthlyTokens: emptyTokens(),
        equivalentCostUsd: null,
        monthlyEquivalentCostUsd: null,
        unpricedTokens: 0,
        monthlyUnpricedTokens: 0,
        childSessionCount: 0,
      };
      groups.set(id, group);
    }

    group.sessions.push(session);
    group.lastActiveAt = Math.max(group.lastActiveAt, session.lastActiveAt);
    addTokens(group.tokens, session.tokens);
    addTokens(group.monthlyTokens, session.monthlyTokens ?? session.tokens);
    group.unpricedTokens += session.unpricedTokens;
    group.monthlyUnpricedTokens +=
      session.monthlyUnpricedTokens ?? session.unpricedTokens;
    group.childSessionCount += session.childSessionCount;
    if (session.equivalentCostUsd !== null) {
      group.equivalentCostUsd =
        (group.equivalentCostUsd ?? 0) + session.equivalentCostUsd;
    }
    const monthlyCost =
      session.monthlyEquivalentCostUsd === undefined
        ? session.equivalentCostUsd
        : session.monthlyEquivalentCostUsd;
    if (monthlyCost !== null) {
      group.monthlyEquivalentCostUsd =
        (group.monthlyEquivalentCostUsd ?? 0) + monthlyCost;
    }
  }

  for (const group of groups.values()) {
    group.sessions.sort((left, right) =>
      right.lastActiveAt === left.lastActiveAt
        ? left.sessionId.localeCompare(right.sessionId)
        : right.lastActiveAt - left.lastActiveAt,
    );
    group.models = [
      ...new Set(
        group.sessions
          .map(session => session.primaryModel)
          .filter((model): model is string => Boolean(model)),
      ),
    ].sort();
  }

  return [...groups.values()].sort((left, right) => {
    if (sort === "tokensDesc") {
      return (
        tokenTotal(right.monthlyTokens) - tokenTotal(left.monthlyTokens) ||
        right.lastActiveAt - left.lastActiveAt
      );
    }
    if (sort === "tokensAsc") {
      return (
        tokenTotal(left.monthlyTokens) - tokenTotal(right.monthlyTokens) ||
        right.lastActiveAt - left.lastActiveAt
      );
    }
    return (
      right.lastActiveAt - left.lastActiveAt || left.name.localeCompare(right.name)
    );
  });
}
