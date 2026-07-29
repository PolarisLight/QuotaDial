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

export function UsageQuotaChart({
  buckets,
  history,
  pace,
  quota,
}: UsageQuotaChartProps) {
  const visibleBuckets = buckets.slice(-7);
  const firstDate = visibleBuckets.at(0)?.startDate;
  const lastDate = visibleBuckets.at(-1)?.startDate;
  const rangeStart = firstDate ? localMidnight(firstDate) : null;
  const rangeEnd = lastDate ? nextLocalMidnight(lastDate) : null;
  const rangeSeconds =
    rangeStart !== null && rangeEnd !== null ? rangeEnd - rangeStart : 0;
  const maxTokens = Math.max(...visibleBuckets.map(bucket => bucket.tokens), 1);
  const slotWidth =
    visibleBuckets.length > 0 ? (RIGHT - LEFT) / visibleBuckets.length : 0;

  const xAt = (timestamp: number) => {
    if (rangeStart === null || rangeSeconds <= 0) return LEFT;
    const ratio = Math.min(
      1,
      Math.max(0, (timestamp - rangeStart) / rangeSeconds),
    );
    return LEFT + ratio * (RIGHT - LEFT);
  };

  const quotaPoints =
    rangeStart === null || rangeEnd === null
      ? []
      : history
          .filter(
            point =>
              point.observedAt >= rangeStart && point.observedAt <= rangeEnd,
          )
          .sort((left, right) => left.observedAt - right.observedAt)
          .map(point => ({
            ...point,
            x: xAt(point.observedAt),
            y: quotaY(point.remainingPercent),
          }));
  const remainingPath =
    quotaPoints.length >= 2
      ? quotaPoints
          .map(
            (point, index) =>
              `${index === 0 ? "M" : "L"} ${point.x.toFixed(1)} ${point.y.toFixed(1)}`,
          )
          .join(" ")
      : null;

  const periodStart =
    quota === null
      ? null
      : quota.resetsAt - quota.windowDurationMins * 60;
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
        <span>最近 7 日</span>
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
        {visibleBuckets.map((bucket, index) => {
          const height = Math.max(
            7,
            (bucket.tokens / maxTokens) * PLOT_HEIGHT,
          );
          const width = Math.min(26, slotWidth * 0.48);
          const center = LEFT + slotWidth * (index + 0.5);
          return (
            <g key={bucket.startDate}>
              <rect
                className="token-bar"
                x={center - width / 2}
                y={BOTTOM - height}
                width={width}
                height={height}
                rx={5}
              >
                <title>
                  {bucket.startDate}: {bucket.tokens.toLocaleString("zh-CN")} Token
                </title>
              </rect>
              <text className="usage-date-label" x={center} y={126}>
                {bucket.startDate.slice(5).replace("-", "/")}
              </text>
            </g>
          );
        })}
        {remainingPath && (
          <>
            <path
              className="remaining-quota-line"
              data-testid="remaining-quota-line"
              d={remainingPath}
            >
              <title>账号剩余额度</title>
            </path>
            {quotaPoints.map(point => (
              <circle
                className="remaining-quota-point"
                key={point.observedAt}
                cx={point.x}
                cy={point.y}
                r={3.2}
              >
                <title>
                  剩余 {point.remainingPercent.toFixed(1)}%
                </title>
              </circle>
            ))}
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
