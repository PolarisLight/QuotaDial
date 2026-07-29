import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, test, vi } from "vitest";
import type { DashboardSnapshot } from "../types/dashboard";
import { DashboardView } from "./Dashboard";

const snapshot: DashboardSnapshot = {
  observedAt: 1_785_330_000,
  isStale: false,
  connectionError: null,
  accountUsageError: null,
  primaryQuota: {
    limitId: "codex",
    label: "7 日额度",
    windowKind: "primary",
    usedPercent: 18,
    remainingPercent: 82,
    windowDurationMins: 10_080,
    resetsAt: 1_785_900_000,
    planType: "plus",
  },
  otherQuotas: [],
  accountUsage: {
    lifetimeTokens: 2_400_000,
    peakDailyTokens: 680_000,
    dailyUsageBuckets: [
      { startDate: "2026-07-23", tokens: 120_000 },
      { startDate: "2026-07-24", tokens: 260_000 },
      { startDate: "2026-07-25", tokens: 180_000 },
      { startDate: "2026-07-26", tokens: 420_000 },
      { startDate: "2026-07-27", tokens: 340_000 },
      { startDate: "2026-07-28", tokens: 530_000 },
      { startDate: "2026-07-29", tokens: 550_000 },
    ],
  },
  forecast: {
    status: "depletesBeforeReset",
    ratePercentPerHour: 1.7,
    exhaustsAt: 1_785_700_000,
    confidence: "medium",
    sampleCount: 6,
    spanSeconds: 10_800,
  },
  sessionDetailsAvailable: false,
};

function renderDashboard(value = snapshot, onRefresh = vi.fn()) {
  render(
    <DashboardView
      snapshot={value}
      loading={false}
      refreshing={false}
      error={null}
      onRefresh={onRefresh}
    />,
  );
  return onRefresh;
}

describe("Dashboard", () => {
  test("renders the approved account overview and terminology", () => {
    renderDashboard();

    expect(screen.getByText("82%")).toBeVisible();
    expect(screen.getAllByText("所有设备").length).toBeGreaterThan(0);
    expect(screen.getByText("预计额度耗尽")).toBeVisible();
    expect(
      screen.getByRole("heading", { name: "会话详情" }),
    ).toBeVisible();
    expect(screen.queryByText("根会话")).not.toBeInTheDocument();
    expect(screen.queryByText("子代理")).not.toBeInTheDocument();
  });

  test("uses a skeleton instead of a circular spinner while loading", () => {
    render(
      <DashboardView
        snapshot={null}
        loading
        refreshing={false}
        error={null}
        onRefresh={vi.fn()}
      />,
    );

    expect(screen.getByLabelText("正在加载账号数据")).toBeVisible();
    expect(screen.queryByRole("progressbar")).not.toBeInTheDocument();
  });

  test("offers retry when no account data can be loaded", () => {
    const onRefresh = vi.fn();
    render(
      <DashboardView
        snapshot={null}
        loading={false}
        refreshing={false}
        error="Codex app-server disconnected"
        onRefresh={onRefresh}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: "重新连接" }));
    expect(onRefresh).toHaveBeenCalledOnce();
  });

  test("retains values and marks stale data", () => {
    renderDashboard({
      ...snapshot,
      isStale: true,
      connectionError: "Codex app-server disconnected",
    });

    expect(screen.getByText("数据已过期")).toBeVisible();
    expect(screen.getByText("82%")).toBeVisible();
  });

  test("shows an honest account usage unavailable state", () => {
    renderDashboard({
      ...snapshot,
      accountUsage: null,
      accountUsageError: "usage unavailable",
    });

    expect(screen.getByText("账号 Token 数据暂不可用")).toBeVisible();
  });

  test("explains when current pace survives the quota window", () => {
    renderDashboard({
      ...snapshot,
      forecast: {
        ...snapshot.forecast!,
        status: "survivesWindow",
        exhaustsAt: null,
      },
    });

    expect(screen.getByText("按当前速率，本周期不会耗尽")).toBeVisible();
  });

  test("does not fabricate session rows in phase one", () => {
    renderDashboard();

    expect(screen.getByText("本机数据接入后显示")).toBeVisible();
    expect(screen.queryByRole("row")).not.toBeInTheDocument();
  });
});
