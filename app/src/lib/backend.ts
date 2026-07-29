import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { DashboardSnapshot, LocalSessionView } from "../types/dashboard";

const previewSnapshot: DashboardSnapshot = {
  observedAt: Math.floor(Date.now() / 1_000),
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
    resetsAt: Math.floor(Date.now() / 1_000) + 4 * 86_400,
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
    exhaustsAt: Math.floor(Date.now() / 1_000) + 36 * 3_600,
    confidence: "medium",
    sampleCount: 6,
    spanSeconds: 10_800,
  },
  localSessions: {
    sessions: [
      {
        sessionId: "preview-session-1",
        title: "codex-monitor · 7月29日",
        projectPath: "/Users/demo/Projects/codex-monitor",
        lastActiveAt: Math.floor(Date.now() / 1_000) - 320,
        primaryModel: "gpt-5.6-terra",
        tokens: {
          inputTokens: 184_200,
          cachedInputTokens: 122_000,
          outputTokens: 23_400,
          reasoningOutputTokens: 8_900,
        },
        equivalentCostUsd: 0.49,
        childSessionCount: 2,
      },
      {
        sessionId: "preview-session-2",
        title: "research-notes · 7月29日",
        projectPath: "/Users/demo/Research/research-notes",
        lastActiveAt: Math.floor(Date.now() / 1_000) - 2_400,
        primaryModel: "gpt-5.6-sol",
        tokens: {
          inputTokens: 91_500,
          cachedInputTokens: 50_300,
          outputTokens: 14_800,
          reasoningOutputTokens: 5_200,
        },
        equivalentCostUsd: 0.67,
        childSessionCount: 0,
      },
    ],
    diagnostics: {
      scannedFiles: 8,
      skippedLines: 0,
      lastImportedAt: Math.floor(Date.now() / 1_000),
      lastError: null,
    },
  },
};

function isWebPreview() {
  return (
    import.meta.env.DEV &&
    typeof window !== "undefined" &&
    !("__TAURI_INTERNALS__" in window)
  );
}

export const backend = {
  getDashboardSnapshot: () => {
    if (isWebPreview()) {
      return Promise.resolve(previewSnapshot);
    }
    return invoke<DashboardSnapshot>("get_dashboard_snapshot");
  },
  refreshAccount: () => {
    if (isWebPreview()) {
      return Promise.resolve({
        ...previewSnapshot,
        observedAt: Math.floor(Date.now() / 1_000),
      });
    }
    return invoke<DashboardSnapshot>("refresh_account");
  },
  rescanSessions: () => {
    if (isWebPreview()) {
      return Promise.resolve(previewSnapshot.localSessions);
    }
    return invoke<LocalSessionView>("rescan_sessions");
  },
  onDashboardUpdated: (
    handler: (snapshot: DashboardSnapshot) => void,
  ): Promise<UnlistenFn> => {
    if (isWebPreview()) {
      return Promise.resolve(() => undefined);
    }
    return listen<DashboardSnapshot>("dashboard://updated", event =>
      handler(event.payload),
    );
  },
  onFocusSection: (handler: (section: string) => void): Promise<UnlistenFn> => {
    if (isWebPreview()) {
      return Promise.resolve(() => undefined);
    }
    return listen<string>("dashboard://focus-section", event =>
      handler(event.payload),
    );
  },
};
