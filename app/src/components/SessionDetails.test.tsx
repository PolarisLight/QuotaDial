import { fireEvent, render, screen, within } from "@testing-library/react";
import { describe, expect, test } from "vitest";
import type { SessionSummary } from "../types/dashboard";
import { SessionDetails } from "./SessionDetails";

function session(
  sessionId: string,
  lastActiveAt: number,
  inputTokens: number,
): SessionSummary {
  return {
    sessionId,
    title: sessionId,
    projectPath: `/tmp/${sessionId}`,
    lastActiveAt,
    primaryModel: "gpt-5.5",
    tokens: {
      inputTokens,
      cachedInputTokens: 0,
      outputTokens: 0,
      reasoningOutputTokens: 0,
    },
    equivalentCostUsd: 0.01,
    pricedTokens: inputTokens,
    unpricedTokens: 0,
    childSessionCount: 0,
  };
}

function renderedSessionTitles() {
  return within(screen.getByRole("table"))
    .getAllByRole("row")
    .slice(1)
    .map(row => within(row).getAllByRole("cell")[0].textContent);
}

describe("SessionDetails", () => {
  test("clicking the Token column toggles descending and ascending order", () => {
    render(
      <SessionDetails
        monthlySubscriptionUsd={20}
        view={{
          sessions: [
            session("new-small", 300, 100),
            session("old-large", 100, 1_000),
            session("middle", 200, 500),
          ],
          monthlySummary: {
            periodStart: 0,
            periodEnd: 1_000,
            tokens: {
              inputTokens: 1_600,
              cachedInputTokens: 0,
              outputTokens: 0,
              reasoningOutputTokens: 0,
            },
            equivalentCostUsd: 0.03,
            pricedTokens: 1_600,
            unpricedTokens: 0,
          },
          diagnostics: {
            scannedFiles: 3,
            skippedLines: 0,
            lastImportedAt: 300,
            lastError: null,
          },
        }}
      />,
    );

    expect(renderedSessionTitles()).toEqual([
      "new-small",
      "middle",
      "old-large",
    ]);
    expect(screen.queryByLabelText("会话排序")).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Token" }));
    expect(renderedSessionTitles()).toEqual([
      "old-large",
      "middle",
      "new-small",
    ]);

    fireEvent.click(screen.getByRole("button", { name: "Token，降序" }));
    expect(renderedSessionTitles()).toEqual([
      "new-small",
      "middle",
      "old-large",
    ]);
  });
});
