import type {
  QuotaHistoryPoint,
  QuotaPace,
  QuotaView,
} from "../types/dashboard";

interface UsageQuotaChartProps {
  buckets: Array<{ startDate: string; tokens: number }>;
  history: QuotaHistoryPoint[];
  pace: QuotaPace | null;
  quota: QuotaView | null;
}

const WIDTH = 700;
const HEIGHT = 142;
const LEFT = 30;
const RIGHT = 670;
const TOP = 10;
const BOTTOM = 104;
const PLOT_HEIGHT = BOTTOM - TOP;

function localMidnight(date: string) {
  const [year, month, day] = date.split("-").map(Number);
  return new Date(year, month - 1, day).getTime() / 1_000;
}

function nextLocalMidnight(date: string) {
  const [year, month, day] = date.split("-").map(Number);
  return new Date(year, month - 1, day + 1).getTime() / 1_000;
}

function bucketMidpoint(date: string) {
  return (localMidnight(date) + nextLocalMidnight(date)) / 2;
}

function shortDate(timestamp: number) {
  const date = new Date(timestamp * 1_000);
  return `${String(date.getMonth() + 1).padStart(2, "0")}/${String(
    date.getDate(),
  ).padStart(2, "0")}`;
}

function clampPercent(value: number) {
  return Math.min(100, Math.max(0, value));
}

function quotaY(remainingPercent: number) {
  return TOP + (1 - clampPercent(remainingPercent) / 100) * PLOT_HEIGHT;
}

function paceCopy(pace: QuotaPace | null) {
  if (!pace) return "正在积累额度样本";
  const label =
    pace.status === "fast" ? "过快" : pace.status === "slow" ? "偏慢" : "正常";
  return `${label} · ${pace.percentPerDay.toFixed(1)}%/天`;
}

function idealRemainingAt(
  timestamp: number,
  periodStart: number,
  ratePerDay: number,
) {
  const elapsedDays = Math.max(0, timestamp - periodStart) / 86_400;
  return clampPercent(100 - elapsedDays * ratePerDay);
}

function smoothPath(points: Array<{ x: number; y: number }>) {
  const first = points.at(0);
  if (!first) return null;
  let path = `M ${first.x.toFixed(1)} ${first.y.toFixed(1)}`;
  for (let index = 1; index < points.length; index += 1) {
    const previous = points[index - 1];
    const current = points[index];
    const controlOffset = (current.x - previous.x) * 0.42;
    path += [
      " C",
      (previous.x + controlOffset).toFixed(1),
      previous.y.toFixed(1),
      (current.x - controlOffset).toFixed(1),
      current.y.toFixed(1),
      current.x.toFixed(1),
      current.y.toFixed(1),
    ].join(" ");
  }
  return path;
}

export function UsageQuotaChart({
  buckets,
  history,
  pace,
  quota,
}: UsageQuotaChartProps) {
  const visibleBuckets = buckets.slice(-7);
  const firstDate = visibleBuckets.at(0)?.startDate;
  const lastDate = visibleBuckets.at(-1)?.startDate;
  const periodStart =
    quota === null
      ? null
      : quota.resetsAt - quota.windowDurationMins * 60;
  const rangeStart = periodStart ?? (firstDate ? localMidnight(firstDate) : null);
  const rangeEnd =
    quota?.resetsAt ?? (lastDate ? nextLocalMidnight(lastDate) : null);
  const rangeSeconds =
    rangeStart !== null && rangeEnd !== null ? rangeEnd - rangeStart : 0;
  const chartSlots =
    quota && rangeStart !== null && rangeSeconds > 0
      ? Array.from({ length: 7 }, (_, index) => {
          const start = rangeStart + (rangeSeconds * index) / 7;
          const end = rangeStart + (rangeSeconds * (index + 1)) / 7;
          const matching = buckets.filter(bucket => {
            const midpoint = bucketMidpoint(bucket.startDate);
            return midpoint >= start && midpoint < end;
          });
          return {
            key: `${start}`,
            label: shortDate((start + end) / 2),
            tokens: matching.reduce((sum, bucket) => sum + bucket.tokens, 0),
            hasData: matching.length > 0,
            title:
              matching.length > 0
                ? matching.map(bucket => bucket.startDate).join("、")
                : "尚无数据",
          };
        })
      : visibleBuckets.map(bucket => ({
          key: bucket.startDate,
          label: bucket.startDate.slice(5).replace("-", "/"),
          tokens: bucket.tokens,
          hasData: true,
          title: bucket.startDate,
        }));
  const maxTokens = Math.max(...chartSlots.map(slot => slot.tokens), 1);
  const slotWidth =
    chartSlots.length > 0 ? (RIGHT - LEFT) / chartSlots.length : 0;

  const xAt = (timestamp: number) => {
    if (rangeStart === null || rangeSeconds <= 0) return LEFT;
    const ratio = Math.min(
      1,
      Math.max(0, (timestamp - rangeStart) / rangeSeconds),
    );
    return LEFT + ratio * (RIGHT - LEFT);
  };

  const observedQuotaPoints =
    rangeStart === null || rangeEnd === null
      ? []
      : history
          .filter(
            point =>
              point.observedAt >= rangeStart && point.observedAt <= rangeEnd,
          )
          .sort((left, right) => left.observedAt - right.observedAt)
          .filter(point => point.observedAt > rangeStart);
  let previousRemaining = 100;
  const quotaPoints =
    rangeStart !== null && observedQuotaPoints.length >= 2
      ? [
          { observedAt: rangeStart, remainingPercent: 100 },
          ...observedQuotaPoints,
        ].map(point => {
          const remainingPercent = Math.min(
            previousRemaining,
            clampPercent(point.remainingPercent),
          );
          previousRemaining = remainingPercent;
          return {
            ...point,
            remainingPercent,
            x: xAt(point.observedAt),
            y: quotaY(remainingPercent),
          };
        })
      : [];
  const remainingPath =
    quotaPoints.length >= 2 ? smoothPath(quotaPoints) : null;

  const idealRate =
    pace?.idealPercentPerDay ??
    (quota && quota.windowDurationMins > 0
      ? 100 / (quota.windowDurationMins / 1_440)
      : null);
  const idealGeometry =
    periodStart !== null &&
    idealRate !== null &&
    rangeStart !== null &&
    rangeEnd !== null
      ? {
          idealStart: quotaY(
            idealRemainingAt(rangeStart, periodStart, idealRate),
          ),
          idealEnd: quotaY(idealRemainingAt(rangeEnd, periodStart, idealRate)),
          fastStart: quotaY(
            idealRemainingAt(rangeStart, periodStart, idealRate * 1.2),
          ),
          fastEnd: quotaY(
            idealRemainingAt(rangeEnd, periodStart, idealRate * 1.2),
          ),
          slowStart: quotaY(
            idealRemainingAt(rangeStart, periodStart, idealRate * 0.8),
          ),
          slowEnd: quotaY(
            idealRemainingAt(rangeEnd, periodStart, idealRate * 0.8),
          ),
        }
      : null;

  return (
    <div
      className="usage-combo-chart"
      aria-label="最近 7 日 Token 与剩余额度"
    >
      <div className="usage-chart-toolbar">
        <span>当前额度周期</span>
        <strong className={`quota-pace-pill ${pace?.status ?? "pending"}`}>
          {paceCopy(pace)}
        </strong>
      </div>
      <svg viewBox={`0 0 ${WIDTH} ${HEIGHT}`} role="img">
        <title>最近 7 日 Token 柱形与剩余额度折线</title>
        {[TOP, TOP + PLOT_HEIGHT / 2, BOTTOM].map(y => (
          <line
            className="usage-grid-line"
            key={y}
            x1={LEFT}
            x2={RIGHT}
            y1={y}
            y2={y}
          />
        ))}
        {idealGeometry && (
          <>
            <path
              className="quota-normal-band"
              d={[
                `M ${LEFT} ${idealGeometry.fastStart}`,
                `L ${RIGHT} ${idealGeometry.fastEnd}`,
                `L ${RIGHT} ${idealGeometry.slowEnd}`,
                `L ${LEFT} ${idealGeometry.slowStart}`,
                "Z",
              ].join(" ")}
            />
            <path
              className="quota-ideal-line"
              d={`M ${LEFT} ${idealGeometry.idealStart} L ${RIGHT} ${idealGeometry.idealEnd}`}
            />
          </>
        )}
        {chartSlots.map((slot, index) => {
          const height = Math.max(
            7,
            (slot.tokens / maxTokens) * PLOT_HEIGHT,
          );
          const width = Math.min(26, slotWidth * 0.48);
          const center = LEFT + slotWidth * (index + 0.5);
          return (
            <g key={slot.key}>
              {slot.hasData ? (
                <rect
                  className="token-bar"
                  x={center - width / 2}
                  y={BOTTOM - height}
                  width={width}
                  height={height}
                  rx={5}
                >
                  <title>
                    {slot.title}: {slot.tokens.toLocaleString("zh-CN")} Token
                  </title>
                </rect>
              ) : (
                <line
                  className="token-slot-placeholder"
                  x1={center - width / 2}
                  x2={center + width / 2}
                  y1={BOTTOM}
                  y2={BOTTOM}
                />
              )}
              <text className="usage-date-label" x={center} y={126}>
                {slot.label}
              </text>
            </g>
          );
        })}
        {remainingPath && (
          <>
            <path
              className="remaining-quota-line"
              data-testid="remaining-quota-line"
              data-y-values={quotaPoints.map(point => point.y).join(",")}
              d={remainingPath}
            >
              <title>账号剩余额度</title>
            </path>
            <circle
              className="remaining-quota-point"
              cx={quotaPoints.at(-1)!.x}
              cy={quotaPoints.at(-1)!.y}
              r={3.5}
            >
              <title>
                剩余 {quotaPoints.at(-1)!.remainingPercent.toFixed(1)}%
              </title>
            </circle>
          </>
        )}
      </svg>
      <div className="usage-chart-legend" aria-hidden="true">
        <span className="legend-token">Token</span>
        <span className="legend-quota">剩余额度</span>
        <span className="legend-ideal">正常速度</span>
      </div>
    </div>
  );
}
