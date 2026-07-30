import { fireEvent, render, screen } from "@testing-library/react";
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
  const interactiveHistory = [
    { observedAt: start + 3_600, remainingPercent: 90 },
    { observedAt: start + day + 3_600, remainingPercent: 75 },
    { observedAt: start + 2 * day + 3_600, remainingPercent: 60 },
  ];

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
        mode="recentRate"
      />,
    );

    expect(container.querySelectorAll(".token-bar")).toHaveLength(7);
    expect(container.querySelectorAll(".usage-day-boundary")).toHaveLength(8);
    const path = container.querySelector(
      '[data-testid="remaining-quota-line"]',
    );
    expect(path).toHaveAttribute("d", expect.stringMatching(/^M /));
    expect(path!.getAttribute("d")).toMatch(/^M 30\.0 10\.0/);
    expect(path!.getAttribute("d")).toContain(" C ");
    const yCoordinates = path!
      .getAttribute("data-y-values")!
      .split(",")
      .map(Number);
    expect(yCoordinates[1]).toBeGreaterThan(yCoordinates[0]);
    expect(yCoordinates[2]).toBeGreaterThan(yCoordinates[1]);
    expect(container.querySelectorAll(".remaining-quota-point")).toHaveLength(3);
    expect(screen.getByText("近期消耗率 · 15.0%/天")).toBeVisible();
    expect(screen.getByText("剩余额度")).toBeVisible();
  });

  test("does not invent a line from one quota sample", () => {
    const { container } = render(
      <UsageQuotaChart
        buckets={buckets}
        history={[{ observedAt: start, remainingPercent: 82 }]}
        pace={null}
        quota={quota}
        mode="recentRate"
      />,
    );

    expect(
      container.querySelector('[data-testid="remaining-quota-line"]'),
    ).toBeNull();
    expect(screen.getByText("正在积累额度样本")).toBeVisible();
  });

  test("shows only the exact Token value when a bar is hovered or focused", () => {
    render(
      <UsageQuotaChart
        buckets={buckets}
        history={interactiveHistory}
        pace={null}
        quota={quota}
        mode="recentRate"
      />,
    );
    const bar = screen.getByLabelText("2026-07-29，70,000 Token");

    fireEvent.mouseEnter(bar);
    expect(screen.getByRole("tooltip")).toHaveTextContent("70,000");
    expect(screen.getByRole("tooltip")).not.toHaveTextContent("07/29");
    expect(screen.getByRole("tooltip")).not.toHaveTextContent("Token");

    fireEvent.mouseLeave(bar);
    expect(screen.queryByRole("tooltip")).toBeNull();

    fireEvent.focus(bar);
    expect(screen.getByRole("tooltip")).toHaveTextContent("70,000");
    fireEvent.blur(bar);
    expect(screen.queryByRole("tooltip")).toBeNull();
  });

  test("shows only the exact remaining percentage for an interactive line point", () => {
    render(
      <UsageQuotaChart
        buckets={buckets}
        history={interactiveHistory}
        pace={null}
        quota={quota}
        mode="recentRate"
      />,
    );
    const point = screen.getByLabelText(/60\.0%/);

    fireEvent.mouseEnter(point);
    expect(screen.getByRole("tooltip")).toHaveTextContent("60.0%");
    expect(screen.getByRole("tooltip")).not.toHaveTextContent("剩余额度");

    fireEvent.mouseLeave(point);
    expect(screen.queryByRole("tooltip")).toBeNull();
  });

  test("renders a visually distinct current-day estimate and reveals it only on hover", () => {
    const cycleStart = new Date(2026, 6, 29, 12).getTime() / 1_000;
    const todayStart = new Date(2026, 6, 30).getTime() / 1_000;
    const { container } = render(
      <UsageQuotaChart
        buckets={[{ startDate: "2026-07-29", tokens: 240_000 }]}
        history={[
          { observedAt: cycleStart + 6 * 3_600, remainingPercent: 95 },
          { observedAt: todayStart, remainingPercent: 90 },
        ]}
        observedAt={todayStart + 12 * 3_600}
        pace={null}
        quota={{
          ...quota,
          remainingPercent: 85,
          resetsAt: cycleStart + 7 * day,
        }}
      />,
    );

    const estimatedBar = container.querySelector(".token-bar.estimated");
    expect(estimatedBar).not.toBeNull();
    expect(screen.queryByText("估算")).not.toBeInTheDocument();

    fireEvent.mouseEnter(estimatedBar!);
    expect(screen.getByRole("tooltip")).toHaveTextContent("估算");
    expect(screen.getByRole("tooltip")).toHaveTextContent("60,000");
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
        mode="recentRate"
      />,
    );
    expect(screen.getByText("近期消耗率 · 10.0%/天")).toBeVisible();

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
        mode="recentRate"
      />,
    );
    expect(screen.getByText("近期消耗率 · 18.0%/天")).toBeVisible();
  });

  test("keeps the reset calendar day as the first label and token bar", () => {
    const resetStart = new Date(2026, 6, 29, 20).getTime() / 1_000;
    const { container } = render(
      <UsageQuotaChart
        buckets={[{ startDate: "2026-07-29", tokens: 123_000 }]}
        history={[
          { observedAt: resetStart + 3_600, remainingPercent: 98 },
          { observedAt: resetStart + 7_200, remainingPercent: 96 },
        ]}
        observedAt={resetStart + 7_200}
        pace={null}
        quota={{
          ...quota,
          resetsAt: resetStart + 7 * day,
        }}
      />,
    );

    expect(screen.getByText("07/29")).toBeVisible();
    expect(container.querySelectorAll(".token-bar")).toHaveLength(1);
    expect(container.querySelector(".token-bar")).toHaveAttribute(
      "aria-label",
      expect.stringContaining("2026-07-29"),
    );
    expect(container.querySelector(".token-bar")).toHaveAttribute(
      "width",
      "26",
    );
  });
});
