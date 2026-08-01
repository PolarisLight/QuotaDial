import { describe, expect, test } from "vitest";
import type { SessionSummary } from "../types/dashboard";
import { groupSessionsByProject } from "./sessionGroups";

function session(
  id: string,
  projectPath: string | null,
  lastActiveAt: number,
  monthlyTokens: number,
): SessionSummary {
  return {
    sessionId: id,
    title: id,
    projectPath,
    lastActiveAt,
    primaryModel: "gpt-5.6",
    tokens: {
      inputTokens: monthlyTokens,
      cachedInputTokens: 0,
      outputTokens: 10,
      reasoningOutputTokens: 0,
    },
    monthlyTokens: {
      inputTokens: monthlyTokens,
      cachedInputTokens: 0,
      outputTokens: 10,
      reasoningOutputTokens: 0,
    },
    equivalentCostUsd: 0.25,
    monthlyEquivalentCostUsd: 0.25,
    pricedTokens: monthlyTokens + 10,
    unpricedTokens: 0,
    monthlyPricedTokens: monthlyTokens + 10,
    monthlyUnpricedTokens: 0,
    childSessionCount: 0,
  };
}

describe("project session grouping", () => {
  test("shows one project for repeated sessions and aggregates usage", () => {
    const groups = groupSessionsByProject(
      [
        session("one", "E:\\Research\\vispfn", 100, 80),
        session("two", "e:/Research/vispfn/", 200, 120),
      ],
      "recent",
    );

    expect(groups).toHaveLength(1);
    expect(groups[0].name).toBe("vispfn");
    expect(groups[0].sessions.map(item => item.sessionId)).toEqual([
      "two",
      "one",
    ]);
    expect(groups[0].monthlyTokens.inputTokens).toBe(200);
    expect(groups[0].monthlyEquivalentCostUsd).toBe(0.5);
  });

  test("keeps projects with the same leaf name separate when paths differ", () => {
    const groups = groupSessionsByProject(
      [
        session("one", "E:\\Research\\vispfn", 100, 80),
        session("two", "D:\\Archive\\vispfn", 200, 120),
      ],
      "recent",
    );

    expect(groups).toHaveLength(2);
    expect(groups.every(group => group.name === "vispfn")).toBe(true);
  });

  test("sorts aggregated projects by monthly usage", () => {
    const groups = groupSessionsByProject(
      [
        session("small", "E:\\small", 300, 10),
        session("large", "E:\\large", 100, 500),
      ],
      "tokensDesc",
    );

    expect(groups.map(group => group.name)).toEqual(["large", "small"]);
  });
});
