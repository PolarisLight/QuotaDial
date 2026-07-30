import type {
  QuotaHistoryPoint,
  QuotaView,
} from "../types/dashboard";

interface TokenEstimateInput {
  buckets: Array<{ startDate: string; tokens: number }>;
  history: QuotaHistoryPoint[];
  quota: Pick<
    QuotaView,
    "resetsAt" | "windowDurationMins" | "remainingPercent"
  >;
  observedAt: number;
}

const DAY_SECONDS = 86_400;

function localMidnight(timestamp: number) {
  const date = new Date(timestamp * 1_000);
  return new Date(
    date.getFullYear(),
    date.getMonth(),
    date.getDate(),
  ).getTime() / 1_000;
}

function localDateKey(timestamp: number) {
  const date = new Date(timestamp * 1_000);
  return [
    date.getFullYear(),
    String(date.getMonth() + 1).padStart(2, "0"),
    String(date.getDate()).padStart(2, "0"),
  ].join("-");
}

function dateStart(date: string) {
  const [year, month, day] = date.split("-").map(Number);
  return new Date(year, month - 1, day).getTime() / 1_000;
}

export function estimateCurrentDayTokens({
  buckets,
  history,
  quota,
  observedAt,
}: TokenEstimateInput) {
  const todayStart = localMidnight(observedAt);
  const todayKey = localDateKey(observedAt);
  if (
    buckets.some(bucket => bucket.startDate === todayKey) ||
    observedAt <= todayStart
  ) {
    return null;
  }

  const cycleStart = quota.resetsAt - quota.windowDurationMins * 60;
  if (cycleStart >= todayStart) return null;

  const points = [
    { observedAt: cycleStart, remainingPercent: 100 },
    ...history.filter(
      point =>
        point.observedAt > cycleStart && point.observedAt < observedAt,
    ),
    { observedAt, remainingPercent: quota.remainingPercent },
  ].sort((left, right) => left.observedAt - right.observedAt);

  const remainingAt = (timestamp: number) => {
    const afterIndex = points.findIndex(
      point => point.observedAt >= timestamp,
    );
    if (afterIndex < 0) return null;
    const after = points[afterIndex];
    if (after.observedAt === timestamp || afterIndex === 0) {
      return after.remainingPercent;
    }
    const before = points[afterIndex - 1];
    const ratio =
      (timestamp - before.observedAt) /
      (after.observedAt - before.observedAt);
    return (
      before.remainingPercent +
      (after.remainingPercent - before.remainingPercent) * ratio
    );
  };

  let referenceTokens = 0;
  let referenceBurn = 0;
  for (const bucket of buckets) {
    const bucketStart = dateStart(bucket.startDate);
    const overlapStart = Math.max(bucketStart, cycleStart);
    const overlapEnd = Math.min(bucketStart + DAY_SECONDS, todayStart);
    if (overlapEnd <= overlapStart) continue;

    const startRemaining = remainingAt(overlapStart);
    const endRemaining = remainingAt(overlapEnd);
    if (startRemaining === null || endRemaining === null) continue;

    const overlapRatio = (overlapEnd - overlapStart) / DAY_SECONDS;
    referenceTokens += bucket.tokens * overlapRatio;
    referenceBurn += Math.max(0, startRemaining - endRemaining);
  }

  const midnightRemaining = remainingAt(todayStart);
  if (
    midnightRemaining === null ||
    referenceTokens <= 0 ||
    referenceBurn < 0.1
  ) {
    return null;
  }
  const todayBurn = Math.max(
    0,
    midnightRemaining - quota.remainingPercent,
  );
  if (todayBurn < 0.05) return null;

  return Math.round((referenceTokens / referenceBurn) * todayBurn);
}
