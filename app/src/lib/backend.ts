import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { DashboardSnapshot, LocalSessionView } from "../types/dashboard";

const PREVIEW_DAY_SECONDS = 86_400;
const previewNow = Math.floor(Date.now() / 1_000);
const previewDate = new Date(previewNow * 1_000);
const previewMonthDay = `${previewDate.getMonth() + 1}月${previewDate.getDate()}日`;
const previewDateKey = (daysAgo: number) => {
  const date = new Date(previewNow * 1_000);
  date.setDate(date.getDate() - daysAgo);
  return [
    date.getFullYear(),
    String(date.getMonth() + 1).padStart(2, "0"),
    String(date.getDate()).padStart(2, "0"),
  ].join("-");
};

const previewSnapshot: DashboardSnapshot = {
  observedAt: previewNow,
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
    resetsAt: previewNow + 4 * PREVIEW_DAY_SECONDS,
    planType: "plus",
  },
  otherQuotas: [],
  accountUsage: {
    lifetimeTokens: 2_400_000,
    peakDailyTokens: 680_000,
    dailyUsageBuckets: [120_000, 260_000, 180_000, 420_000, 340_000, 530_000, 550_000].map(
      (tokens, index) => ({
        startDate: previewDateKey(6 - index),
        tokens,
      }),
    ),
  },
  forecast: {
    status: "depletesBeforeReset",
    ratePercentPerHour: 1.7,
    exhaustsAt: previewNow + 36 * 3_600,
    confidence: "medium",
    sampleCount: 6,
    spanSeconds: 10_800,
  },
  quotaHistory: [
    {
      observedAt: previewNow - 3 * PREVIEW_DAY_SECONDS + 3_600,
      remainingPercent: 98,
    },
    {
      observedAt: previewNow - 2 * PREVIEW_DAY_SECONDS,
      remainingPercent: 93,
    },
    {
      observedAt: previewNow - PREVIEW_DAY_SECONDS,
      remainingPercent: 87,
    },
    {
      observedAt: previewNow,
      remainingPercent: 82,
    },
  ],
  quotaPace: {
    percentPerDay: 15,
    idealPercentPerDay: 14.2857,
    status: "normal",
    sampleCount: 7,
  },
  localSessions: {
    sessions: [
      {
        sessionId: "preview-session-1",
        title: `codex-monitor · ${previewMonthDay}`,
        projectPath: "/Users/demo/Projects/codex-monitor",
        lastActiveAt: previewNow - 320,
        primaryModel: "gpt-5.6-terra",
        tokens: {
          inputTokens: 184_200,
          cachedInputTokens: 122_000,
          outputTokens: 23_400,
          reasoningOutputTokens: 8_900,
        },
        equivalentCostUsd: 0.49,
        pricedTokens: 207_600,
        unpricedTokens: 0,
        childSessionCount: 2,
      },
      {
        sessionId: "preview-session-2",
        title: `research-notes · ${previewMonthDay}`,
        projectPath: "/Users/demo/Research/research-notes",
        lastActiveAt: previewNow - 2_400,
        primaryModel: "gpt-5.6-sol",
        tokens: {
          inputTokens: 91_500,
          cachedInputTokens: 50_300,
          outputTokens: 14_800,
          reasoningOutputTokens: 5_200,
        },
        equivalentCostUsd: 0.67,
        pricedTokens: 106_300,
        unpricedTokens: 0,
        childSessionCount: 0,
      },
    ],
    diagnostics: {
      scannedFiles: 8,
      skippedLines: 0,
      lastImportedAt: previewNow,
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
