export type ForecastStatus =
  | "depletesBeforeReset"
  | "survivesWindow"
  | "noMeasurableBurn";

export type ForecastConfidence = "low" | "medium" | "high";

export interface QuotaView {
  limitId: string;
  label: string;
  windowKind: string;
  usedPercent: number;
  remainingPercent: number;
  windowDurationMins: number;
  resetsAt: number;
  planType: string | null;
}

export interface AccountUsageView {
  lifetimeTokens: number | null;
  peakDailyTokens: number | null;
  dailyUsageBuckets: Array<{ startDate: string; tokens: number }>;
}

export interface ExhaustionForecast {
  status: ForecastStatus;
  ratePercentPerHour: number;
  exhaustsAt: number | null;
  confidence: ForecastConfidence;
  sampleCount: number;
  spanSeconds: number;
}

export type QuotaPaceStatus = "slow" | "normal" | "fast";

export interface QuotaHistoryPoint {
  observedAt: number;
  remainingPercent: number;
}

export interface QuotaPace {
  percentPerDay: number;
  idealPercentPerDay: number;
  status: QuotaPaceStatus;
  sampleCount: number;
}

export interface TokenBreakdown {
  inputTokens: number;
  cachedInputTokens: number;
  outputTokens: number;
  reasoningOutputTokens: number;
}

export interface SessionSummary {
  sessionId: string;
  title: string;
  projectPath: string | null;
  lastActiveAt: number;
  primaryModel: string | null;
  tokens: TokenBreakdown;
  monthlyTokens: TokenBreakdown;
  equivalentCostUsd: number | null;
  monthlyEquivalentCostUsd: number | null;
  pricedTokens: number;
  unpricedTokens: number;
  monthlyPricedTokens: number;
  monthlyUnpricedTokens: number;
  childSessionCount: number;
}

export interface SessionDiagnostics {
  scannedFiles: number;
  skippedLines: number;
  lastImportedAt: number | null;
  lastError: string | null;
}

export interface MonthlyUsageSummary {
  periodStart: number;
  periodEnd: number;
  tokens: TokenBreakdown;
  equivalentCostUsd: number | null;
  pricedTokens: number;
  unpricedTokens: number;
}

export interface LocalSessionView {
  sessions: SessionSummary[];
  monthlySummary: MonthlyUsageSummary;
  diagnostics: SessionDiagnostics;
}

export interface DashboardSnapshot {
  observedAt: number;
  isStale: boolean;
  connectionError: string | null;
  accountUsageError: string | null;
  primaryQuota: QuotaView | null;
  otherQuotas: QuotaView[];
  accountUsage: AccountUsageView | null;
  forecast: ExhaustionForecast | null;
  quotaHistory: QuotaHistoryPoint[];
  quotaPace: QuotaPace | null;
  localSessions: LocalSessionView;
}

export interface TrayPanelSnapshot {
  observedAt: number;
  isStale: boolean;
  connectionError: string | null;
  primaryQuota: QuotaView | null;
  forecastStatus: ForecastStatus | null;
  latestDailyTokens: number | null;
  projectCount: number;
  sessionCount: number;
}
