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

export interface DashboardSnapshot {
  observedAt: number;
  isStale: boolean;
  connectionError: string | null;
  accountUsageError: string | null;
  primaryQuota: QuotaView | null;
  otherQuotas: QuotaView[];
  accountUsage: AccountUsageView | null;
  forecast: ExhaustionForecast | null;
  sessionDetailsAvailable: boolean;
}
