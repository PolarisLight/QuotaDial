import type {
  QuotaHistoryPoint,
  QuotaPace,
  QuotaView,
} from "../types/dashboard";
import { recentRateCopy, suggestedPace } from "../lib/pace";
import type { PaceMode } from "../types/settings";

interface UsageQuotaChartProps {
  buckets: Array<{ startDate: string; tokens: number }>;
  history: QuotaHistoryPoint[];
  pace: QuotaPace | null;
  quota: QuotaView | null;
  observedAt?: number;
  mode?: PaceMode;
  onModeChange?: (mode: PaceMode) => void;
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

function nextCalendarBoundary(timestamp: number) {
  const date = new Date(timestamp * 1_000);
  return (
    new Date(
      date.getFullYear(),
      date.getMonth(),
      date.getDate() + 1,
    ).getTime() / 1_000
  );
}

function calendarBoundaries(start: number, end: number) {
  const boundaries = [start];
  let cursor = nextCalendarBoundary(start);
  while (cursor < end && boundaries.length < 10) {
    boundaries.push(cursor);
    cursor = nextCalendarBoundary(cursor);
  }
  if (boundaries.at(-1) !== end) boundaries.push(end);
  return boundaries;
}

function shortDate(timestamp: number) {
  const date = new Date(timestamp * 1_000);
  return `${String(date.getMonth() + 1).padStart(2, "0")}/${String(
    date.getDate(),
  ).padStart(2, "0")}`;
}

function localDateKey(timestamp: number) {
  const date = new Date(timestamp * 1_000);
  return [
    date.getFullYear(),
    String(date.getMonth() + 1).padStart(2, "0"),
    String(date.getDate()).padStart(2, "0"),
  ].join("-");
}

function clampPercent(value: number) {
  return Math.min(100, Math.max(0, value));
}

function quotaY(remainingPercent: number) {
  return TOP + (1 - clampPercent(remainingPercent) / 100) * PLOT_HEIGHT;
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
  observedAt = Math.floor(Date.now() / 1_000),
  mode = "suggested",
  onModeChange = () => undefined,
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
  const xAt = (timestamp: number) => {
    if (rangeStart === null || rangeSeconds <= 0) return LEFT;
    const ratio = Math.min(
      1,
      Math.max(0, (timestamp - rangeStart) / rangeSeconds),
    );
    return LEFT + ratio * (RIGHT - LEFT);
  };
  const timeBoundaries =
    rangeStart !== null && rangeEnd !== null && rangeSeconds > 0
      ? calendarBoundaries(rangeStart, rangeEnd)
      : [];
  const chartSlots =
    timeBoundaries.length >= 2
      ? timeBoundaries.slice(0, -1).map((start, index) => {
          const end = timeBoundaries[index + 1];
          const slotDate = localDateKey(start);
          const matching = buckets.filter(
            bucket => bucket.startDate === slotDate,
          );
          return {
            key: `${start}`,
            label: shortDate(start),
            start,
            end,
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
          start: localMidnight(bucket.startDate),
          end: nextLocalMidnight(bucket.startDate),
          tokens: bucket.tokens,
          hasData: true,
          title: bucket.startDate,
        }));
  const maxTokens = Math.max(...chartSlots.map(slot => slot.tokens), 1);

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
  const guidance =
    quota === null
      ? null
      : suggestedPace({
          remainingPercent: quota.remainingPercent,
          observedAt,
          periodStart: quota.resetsAt - quota.windowDurationMins * 60,
          resetsAt: quota.resetsAt,
        });
  const paceStatus = mode === "suggested" ? guidance?.status : pace?.status;
  const paceText =
    mode === "suggested"
      ? guidance?.copy ?? "正在读取额度周期"
      : recentRateCopy(pace);

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
        <div className="pace-toolbar-actions">
          <div className="pace-mode-control" aria-label="节奏指标">
            <button
              type="button"
              aria-pressed={mode === "suggested"}
              onClick={() => onModeChange("suggested")}
            >
              配速建议
            </button>
            <button
              type="button"
              aria-pressed={mode === "recentRate"}
              onClick={() => onModeChange("recentRate")}
            >
              近期消耗率
            </button>
          </div>
          <strong className={`quota-pace-pill ${paceStatus ?? "pending"}`}>
            {paceText}
          </strong>
        </div>
      </div>
      <svg viewBox={`0 0 ${WIDTH} ${HEIGHT}`} role="img">
        <title>最近 7 日 Token 柱形与剩余额度折线</title>
        {timeBoundaries.map((timestamp, index) => {
          const x = xAt(timestamp);
          return (
            <line
              className="usage-day-boundary"
              key={`day-${index}`}
              x1={x}
              x2={x}
              y1={TOP}
              y2={BOTTOM}
            />
          );
        })}
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
        {chartSlots.map(slot => {
          const height = Math.max(
            7,
            (slot.tokens / maxTokens) * PLOT_HEIGHT,
          );
          const slotLeft = xAt(slot.start);
          const slotRight = xAt(slot.end);
          const width = Math.min(26, (slotRight - slotLeft) * 0.48);
          const center = (slotLeft + slotRight) / 2;
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
