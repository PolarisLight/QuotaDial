export type PaceStatus = "fast" | "normal" | "slow";

export function classifySuggestedPace(ratioPercent: number): PaceStatus {
  if (ratioPercent < 85) return "fast";
  if (ratioPercent > 115) return "slow";
  return "normal";
}

export function suggestedPace({
  remainingPercent,
  observedAt,
  periodStart,
  resetsAt,
}: {
  remainingPercent: number;
  observedAt: number;
  periodStart: number;
  resetsAt: number;
}) {
  const total = resetsAt - periodStart;
  const remainingTimePercent =
    total > 0
      ? Math.min(
          100,
          Math.max(0, ((resetsAt - observedAt) / total) * 100),
        )
      : 0;
  const ratioPercent =
    remainingTimePercent > 0
      ? Math.round((remainingPercent / remainingTimePercent) * 100)
      : remainingPercent > 0
        ? 1_000
        : 100;
  const status = classifySuggestedPace(ratioPercent);
  const copy =
    status === "fast"
      ? `明显偏快 · 建议降至 ${ratioPercent}%`
      : status === "slow"
        ? `使用偏慢 · 可提升至 ${ratioPercent}%`
        : `速度正常 · 建议保持 ${ratioPercent}%`;
  return { ratioPercent, status, copy };
}

export function recentRateCopy(pace: { percentPerDay: number } | null) {
  return pace
    ? `近期消耗率 · ${pace.percentPerDay.toFixed(1)}%/天`
    : "正在积累额度样本";
}
