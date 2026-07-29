import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { DashboardSnapshot } from "../types/dashboard";

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
  sessionDetailsAvailable: false,
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
};
