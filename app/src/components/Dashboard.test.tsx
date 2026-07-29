import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, test, vi } from "vitest";
import "../styles/app.css";
import type { DashboardSnapshot } from "../types/dashboard";
import { DashboardView, focusDashboardSection } from "./Dashboard";

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
  localSessions: {
    sessions: [],
    diagnostics: {
      scannedFiles: 0,
      skippedLines: 0,
      lastImportedAt: null,
      lastError: null,
    },
  },
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
  test("uses the remaining-window brand instead of a waveform", () => {
    renderDashboard();

    expect(screen.getByLabelText("Codex Monitor 余量窗口")).toBeVisible();
    expect(screen.queryByTestId("waveform-brand")).not.toBeInTheDocument();
  });

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

  test("limits the token chart to the seven most recent daily buckets", () => {
    const dailyUsageBuckets = Array.from({ length: 12 }, (_, index) => ({
      startDate: `2026-07-${String(index + 18).padStart(2, "0")}`,
      tokens: (index + 1) * 1_000,
    }));
    const { container } = render(
      <DashboardView
        snapshot={{
          ...snapshot,
          accountUsage: {
            ...snapshot.accountUsage!,
            dailyUsageBuckets,
          },
        }}
        loading={false}
        refreshing={false}
        error={null}
        onRefresh={vi.fn()}
      />,
    );

    expect(container.querySelectorAll(".token-column")).toHaveLength(7);
    expect(screen.queryByText("07/18")).not.toBeInTheDocument();
    expect(screen.getByText("07/29")).toBeVisible();
  });

  test("fills the quota bar left to right using consumed percentage", () => {
    const { container } = render(
      <DashboardView
        snapshot={{
          ...snapshot,
          primaryQuota: {
            ...snapshot.primaryQuota!,
            usedPercent: 25,
            remainingPercent: 75,
          },
        }}
        loading={false}
        refreshing={false}
        error={null}
        onRefresh={vi.fn()}
      />,
    );

    const fill = container.querySelector<HTMLElement>(".quota-track > span");
    expect(fill).toHaveStyle({ width: "25%" });
    expect(screen.getByText("已消耗 25%")).toBeVisible();
    expect(screen.getByText("剩余 75%")).toBeVisible();
  });

  test("renders one row per top-level session with child usage already included", () => {
    const { container } = render(
      <DashboardView
        snapshot={{
          ...snapshot,
          localSessions: {
            diagnostics: {
              scannedFiles: 4,
              skippedLines: 0,
              lastImportedAt: 1_785_330_000,
              lastError: null,
            },
            sessions: [
              {
                sessionId: "root-1",
                title: "example-project · 7月29日",
                projectPath: "/tmp/example-project",
                lastActiveAt: 1_785_330_000,
                primaryModel: "gpt-5.6-codex",
                tokens: {
                  inputTokens: 1_300,
                  cachedInputTokens: 500,
                  outputTokens: 280,
                  reasoningOutputTokens: 70,
                },
                equivalentCostUsd: 0.02,
                childSessionCount: 1,
              },
            ],
          },
        } as DashboardSnapshot}
        loading={false}
        refreshing={false}
        error={null}
        onRefresh={vi.fn()}
      />,
    );

    expect(screen.getByText("example-project · 7月29日")).toBeVisible();
    expect(container.querySelectorAll("tbody .session-row")).toHaveLength(1);
    expect(screen.getByText("含 1 个子任务")).toBeVisible();
    expect(screen.queryByText("根会话")).not.toBeInTheDocument();
  });

  test("shows only the eight most recent sessions", () => {
    const sessions = Array.from({ length: 12 }, (_, index) => ({
      sessionId: `session-${index}`,
      title: `会话 ${index}`,
      projectPath: `/tmp/project-${index}`,
      lastActiveAt: 1_785_330_000 - index,
      primaryModel: "gpt-5.6-sol",
      tokens: {
        inputTokens: 1_000,
        cachedInputTokens: 400,
        outputTokens: 200,
        reasoningOutputTokens: 50,
      },
      equivalentCostUsd: 0.01,
      childSessionCount: 0,
    }));

    const { container } = render(
      <DashboardView
        snapshot={{
          ...snapshot,
          localSessions: {
            sessions,
            diagnostics: {
              scannedFiles: 12,
              skippedLines: 0,
              lastImportedAt: 1_785_330_000,
              lastError: null,
            },
          },
        }}
        loading={false}
        refreshing={false}
        error={null}
        onRefresh={vi.fn()}
      />,
    );

    expect(container.querySelectorAll("tbody .session-row")).toHaveLength(8);
    expect(screen.getByText("最近 8 个会话")).toBeVisible();
    expect(screen.queryByText("会话 8")).not.toBeInTheDocument();
  });

  test("uses the native window as the only scroll boundary", () => {
    renderDashboard();
    const frame = document.querySelector(".app-window")!;
    const content = document.querySelector(".content")!;

    expect((frame as HTMLElement).style.width).toBe("100vw");
    expect((frame as HTMLElement).style.height).toBe("100vh");
    expect(frame).toHaveStyle({ margin: "0px", overflow: "hidden" });
    expect(content).toHaveStyle({
      overflowY: "auto",
      overscrollBehaviorY: "none",
    });
  });

  test("distinguishes import failure from a genuinely empty local history", () => {
    renderDashboard({
      ...snapshot,
      localSessions: {
        sessions: [],
        diagnostics: {
          scannedFiles: 0,
          skippedLines: 0,
          lastImportedAt: null,
          lastError: "permission denied",
        },
      },
    } as DashboardSnapshot);

    expect(screen.getByText("无法读取本机会话记录")).toBeVisible();
    expect(screen.getByRole("button", { name: "重新扫描" })).toBeVisible();
  });

  test("focuses the session section requested by the menu bar", () => {
    vi.useFakeTimers();
    renderDashboard();
    const section = screen
      .getByRole("heading", { name: "会话详情" })
      .closest("section")!;

    focusDashboardSection("sessions");

    expect(section).toHaveClass("section-focused");
    vi.runAllTimers();
    expect(section).not.toHaveClass("section-focused");
    vi.useRealTimers();
  });
});
