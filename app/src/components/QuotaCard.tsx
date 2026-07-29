import { ArrowClockwise, Devices, Timer } from "@phosphor-icons/react";
import type { QuotaView } from "../types/dashboard";

interface QuotaCardProps {
  quota: QuotaView;
  observedAt: number;
  refreshing: boolean;
  onRefresh: () => void;
}

function formatDate(timestamp: number) {
  return new Intl.DateTimeFormat("zh-CN", {
    month: "numeric",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  }).format(new Date(timestamp * 1_000));
}

function formatTime(timestamp: number) {
  return new Intl.DateTimeFormat("zh-CN", {
    hour: "2-digit",
    minute: "2-digit",
  }).format(new Date(timestamp * 1_000));
}

export function QuotaCard({
  quota,
  observedAt,
  refreshing,
  onRefresh,
}: QuotaCardProps) {
  return (
    <section className="panel quota-card" aria-labelledby="quota-heading">
      <div className="panel-heading">
        <div>
          <span className="eyebrow">账号额度</span>
          <h2 id="quota-heading">{quota.label}</h2>
        </div>
        <button
          className="icon-button"
          type="button"
          aria-label="刷新账号额度"
          disabled={refreshing}
          onClick={onRefresh}
        >
          <ArrowClockwise
            className={refreshing ? "refreshing" : undefined}
            size={18}
          />
        </button>
      </div>

      <div className="quota-value">
        <strong>{Math.round(quota.remainingPercent)}%</strong>
        <span>剩余</span>
      </div>

      <div
        className="quota-track"
        aria-label={`已使用 ${Math.round(quota.usedPercent)}%`}
      >
        <span style={{ width: `${quota.usedPercent}%` }} />
      </div>
      <div className="quota-scale">
        <span>已使用 {Math.round(quota.usedPercent)}%</span>
        <span>上限 100%</span>
      </div>

      <dl className="quota-meta">
        <div>
          <dt>
            <Timer size={16} />
            计划恢复
          </dt>
          <dd>{formatDate(quota.resetsAt)}</dd>
        </div>
        <div>
          <dt>
            <Devices size={16} />
            统计范围
          </dt>
          <dd>所有设备</dd>
        </div>
      </dl>

      <p className="last-updated">最近刷新 {formatTime(observedAt)}</p>
    </section>
  );
}
