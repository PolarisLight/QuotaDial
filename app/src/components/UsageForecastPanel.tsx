import { ChartLineUp, Gauge } from "@phosphor-icons/react";
import type {
  AccountUsageView,
  ExhaustionForecast,
  QuotaHistoryPoint,
  QuotaPace,
  QuotaView,
} from "../types/dashboard";
import type { PaceMode } from "../types/settings";
import { UsageQuotaChart } from "./UsageQuotaChart";

interface UsageForecastPanelProps {
  usage: AccountUsageView | null;
  usageError: string | null;
  forecast: ExhaustionForecast | null;
  history: QuotaHistoryPoint[];
  pace: QuotaPace | null;
  quota: QuotaView | null;
  observedAt: number;
  paceMode: PaceMode;
  onPaceModeChange: (mode: PaceMode) => void;
}

const tokenFormatter = new Intl.NumberFormat("zh-CN", {
  notation: "compact",
  maximumFractionDigits: 1,
});

function localDateKey(timestamp: number) {
  const date = new Date(timestamp * 1_000);
  return [
    date.getFullYear(),
    String(date.getMonth() + 1).padStart(2, "0"),
    String(date.getDate()).padStart(2, "0"),
  ].join("-");
}

function bucketLabel(startDate: string, observedAt: number) {
  if (startDate === localDateKey(observedAt)) return "今日 Token";
  const [, month, day] = startDate.split("-").map(Number);
  return `${month} 月 ${day} 日 Token`;
}

function forecastCopy(forecast: ExhaustionForecast | null) {
  if (!forecast) {
    return {
      value: "正在积累样本",
      note: "至少需要 3 个观测点和 30 分钟跨度",
    };
  }
  if (forecast.status === "survivesWindow") {
    return {
      value: "按当前速率，本周期不会耗尽",
      note: `当前消耗 ${forecast.ratePercentPerHour.toFixed(2)}% / 小时`,
    };
  }
  if (forecast.status === "noMeasurableBurn") {
    return {
      value: "当前没有明显消耗",
      note: "样本中的额度变化低于测量阈值",
    };
  }
  return {
    value: forecast.exhaustsAt
      ? new Intl.DateTimeFormat("zh-CN", {
          month: "numeric",
          day: "numeric",
          hour: "2-digit",
          minute: "2-digit",
        }).format(new Date(forecast.exhaustsAt * 1_000))
      : "—",
    note: `所有设备 · ${forecast.confidence === "high" ? "高" : forecast.confidence === "medium" ? "中" : "低"}可信度`,
  };
}

export function UsageForecastPanel({
  usage,
  usageError,
  forecast,
  history,
  pace,
  quota,
  observedAt,
  paceMode,
  onPaceModeChange,
}: UsageForecastPanelProps) {
  const copy = forecastCopy(forecast);
  const dailyBuckets = usage?.dailyUsageBuckets ?? [];
  const latestBucket = dailyBuckets.reduce<(typeof dailyBuckets)[number] | null>(
    (latest, bucket) =>
      latest === null || bucket.startDate > latest.startDate ? bucket : latest,
    null,
  );
  const latestTokens = latestBucket?.tokens ?? null;
  const latestTokenLabel = latestBucket
    ? bucketLabel(latestBucket.startDate, observedAt)
    : "今日 Token";

  return (
    <section className="panel usage-panel" aria-labelledby="usage-heading">
      <div className="panel-heading">
        <div>
          <span className="eyebrow">消耗与预测</span>
          <h2 id="usage-heading">Token 与额度趋势</h2>
        </div>
        <span className="scope-pill">所有设备</span>
      </div>

      {usage ? (
        <>
          <div className="usage-summary">
            <div>
              <span>{latestTokenLabel}</span>
              <strong>
                {latestTokens === null
                  ? "—"
                  : tokenFormatter.format(latestTokens)}
              </strong>
            </div>
            <div>
              <span>历史峰值</span>
              <strong>
                {usage.peakDailyTokens === null
                  ? "—"
                  : tokenFormatter.format(usage.peakDailyTokens)}
              </strong>
            </div>
          </div>

          <UsageQuotaChart
            buckets={dailyBuckets}
            history={history}
            pace={pace}
            quota={quota}
            observedAt={observedAt}
            mode={paceMode}
            onModeChange={onPaceModeChange}
          />
        </>
      ) : (
        <div className="inline-empty">
          <ChartLineUp size={21} />
          <div>
            <strong>账号 Token 数据暂不可用</strong>
            <span>{usageError ?? "Codex 尚未返回账号使用记录"}</span>
          </div>
        </div>
      )}

      <div className="forecast-row">
        <div className="forecast-icon">
          <Gauge size={20} />
        </div>
        <div>
          <span>预计额度耗尽</span>
          <strong>{copy.value}</strong>
          <small>{copy.note}</small>
        </div>
      </div>
    </section>
  );
}
