import { ChartLineUp, CurrencyDollar, Gauge } from "@phosphor-icons/react";
import type {
  AccountUsageView,
  ExhaustionForecast,
} from "../types/dashboard";

interface UsageForecastPanelProps {
  usage: AccountUsageView | null;
  usageError: string | null;
  forecast: ExhaustionForecast | null;
}

const tokenFormatter = new Intl.NumberFormat("zh-CN", {
  notation: "compact",
  maximumFractionDigits: 1,
});

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
}: UsageForecastPanelProps) {
  const copy = forecastCopy(forecast);
  const buckets = usage?.dailyUsageBuckets ?? [];
  const maxTokens = Math.max(...buckets.map(bucket => bucket.tokens), 1);
  const today = buckets.at(-1)?.tokens ?? null;

  return (
    <section className="panel usage-panel" aria-labelledby="usage-heading">
      <div className="panel-heading">
        <div>
          <span className="eyebrow">消耗与预测</span>
          <h2 id="usage-heading">Token 趋势</h2>
        </div>
        <span className="scope-pill">所有设备</span>
      </div>

      {usage ? (
        <>
          <div className="usage-summary">
            <div>
              <span>今日 Token</span>
              <strong>{today === null ? "—" : tokenFormatter.format(today)}</strong>
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

          <div className="token-chart" aria-label="最近每日 Token">
            {buckets.map(bucket => (
              <div className="token-column" key={bucket.startDate}>
                <span
                  className="token-bar"
                  style={{ height: `${Math.max(8, (bucket.tokens / maxTokens) * 100)}%` }}
                  title={`${bucket.startDate}: ${bucket.tokens.toLocaleString("zh-CN")} Token`}
                />
                <small>{bucket.startDate.slice(5).replace("-", "/")}</small>
              </div>
            ))}
          </div>
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

      <div className="cost-note">
        <CurrencyDollar size={16} />
        <span>等效费用</span>
        <strong>本机统计接入后显示</strong>
      </div>
    </section>
  );
}
