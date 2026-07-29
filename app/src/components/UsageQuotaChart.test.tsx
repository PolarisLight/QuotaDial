import { render, screen } from "@testing-library/react";
import { describe, expect, test } from "vitest";
import { UsageQuotaChart } from "./UsageQuotaChart";

const day = 86_400;
const start = new Date(2026, 6, 23).getTime() / 1_000;
const buckets = Array.from({ length: 7 }, (_, index) => {
  const date = new Date((start + index * day) * 1_000);
  return {
    startDate: [
      date.getFullYear(),
      String(date.getMonth() + 1).padStart(2, "0"),
      String(date.getDate()).padStart(2, "0"),
    ].join("-"),
    tokens: (index + 1) * 10_000,
  };
});
const quota = {
  limitId: "codex",
  label: "7 日额度",
  windowKind: "primary",
  usedPercent: 40,
  remainingPercent: 60,
  windowDurationMins: 10_080,
  resetsAt: start + 7 * day,
  planType: "plus",
};

describe("UsageQuotaChart", () => {
  test("renders token bars and a remaining-quota line that descends over time", () => {
    const { container } = render(
      <UsageQuotaChart
        buckets={buckets}
        history={[
          { observedAt: start + 3_600, remainingPercent: 90 },
          { observedAt: start + day + 3_600, remainingPercent: 75 },
          { observedAt: start + 2 * day + 3_600, remainingPercent: 60 },
        ]}
        pace={{
          percentPerDay: 15,
          idealPercentPerDay: 14.2857,
          status: "normal",
          sampleCount: 3,
        }}
        quota={quota}
      />,
    );

    expect(container.querySelectorAll(".token-bar")).toHaveLength(7);
    const path = container.querySelector(
      '[data-testid="remaining-quota-line"]',
    );
    expect(path).toHaveAttribute("d", expect.stringMatching(/^M /));
    const yCoordinates = path!
      .getAttribute("d")!
      .match(/(?:M|L) [\d.]+ ([\d.]+)/g)!
      .map(command => Number(command.split(" ").at(-1)));
    expect(yCoordinates[1]).toBeGreaterThan(yCoordinates[0]);
    expect(yCoordinates[2]).toBeGreaterThan(yCoordinates[1]);
    expect(screen.getByText("正常 · 15.0%/天")).toBeVisible();
    expect(screen.getByText("剩余额度")).toBeVisible();
  });

  test("does not invent a line from one quota sample", () => {
    const { container } = render(
      <UsageQuotaChart
        buckets={buckets}
        history={[{ observedAt: start, remainingPercent: 82 }]}
        pace={null}
        quota={quota}
      />,
    );

    expect(
      container.querySelector('[data-testid="remaining-quota-line"]'),
    ).toBeNull();
    expect(screen.getByText("正在积累额度样本")).toBeVisible();
  });

  test("labels slow and fast consumption without changing line direction", () => {
    const { rerender } = render(
      <UsageQuotaChart
        buckets={buckets}
        history={[]}
        pace={{
          percentPerDay: 10,
          idealPercentPerDay: 14.2857,
          status: "slow",
          sampleCount: 4,
        }}
        quota={quota}
      />,
    );
    expect(screen.getByText("偏慢 · 10.0%/天")).toBeVisible();

    rerender(
      <UsageQuotaChart
        buckets={buckets}
        history={[]}
        pace={{
          percentPerDay: 18,
          idealPercentPerDay: 14.2857,
          status: "fast",
          sampleCount: 4,
        }}
        quota={quota}
      />,
    );
    expect(screen.getByText("过快 · 18.0%/天")).toBeVisible();
  });
});
