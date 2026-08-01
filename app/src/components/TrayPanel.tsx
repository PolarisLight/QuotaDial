import {
  ArrowClockwise,
  ArrowSquareOut,
  ClockCountdown,
  GearSix,
  Power,
} from "@phosphor-icons/react";
import { type CSSProperties, useEffect } from "react";
import { useTraySnapshot } from "../hooks/useTraySnapshot";
import { backend } from "../lib/backend";
import { BrandMark } from "./BrandMark";

const compactNumber = new Intl.NumberFormat("zh-CN", {
  notation: "compact",
  maximumFractionDigits: 1,
});

function formatReset(timestamp: number) {
  return new Intl.DateTimeFormat("zh-CN", {
    month: "numeric",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  }).format(new Date(timestamp * 1_000));
}

function forecastLabel(status: string | undefined) {
  if (status === "depletesBeforeReset") return "按当前速度，可能在重置前耗尽";
  if (status === "survivesWindow") return "按当前速度，可以使用到本次重置";
  if (status === "noMeasurableBurn") return "当前消耗平稳";
  return "正在积累消耗趋势";
}

export function TrayPanel() {
  const tray = useTraySnapshot();
  const snapshot = tray.snapshot;
  const quota = snapshot?.primaryQuota;
  const remaining = quota?.remainingPercent;

  useEffect(() => {
    void backend.getAppSettings().then(settings => {
      if (settings.theme === "system") {
        delete document.documentElement.dataset.theme;
      } else {
        document.documentElement.dataset.theme = settings.theme;
      }
    });
  }, []);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") void backend.hideTrayPanel();
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);

  const ringStyle = {
    "--quota-remaining": `${Math.max(0, Math.min(100, remaining ?? 0)) * 3.6}deg`,
  } as CSSProperties;

  return (
    <main className="tray-flyout" aria-label="QuotaDial 额度面板">
      <header className="tray-header">
        <div className="tray-brand">
          <span className="tray-brand-mark">
            <BrandMark />
          </span>
          <div className="tray-brand-copy">
            <strong>QuotaDial</strong>
            <span className={snapshot?.isStale ? "stale" : ""}>
              {snapshot?.isStale ? "数据已过期" : "Codex 已连接"}
            </span>
          </div>
        </div>
        <button
          className="tray-icon-button"
          type="button"
          aria-label="刷新额度"
          title="刷新额度"
          onClick={() => void tray.refresh()}
          disabled={tray.refreshing}
        >
          <ArrowClockwise
            className={tray.refreshing ? "refreshing" : undefined}
            size={17}
          />
        </button>
      </header>

      {tray.loading && !snapshot ? (
        <TrayPanelSkeleton />
      ) : (
        <>
          <section className="tray-quota-overview">
            <div className="tray-quota-ring" style={ringStyle}>
              <div>
                <strong>{remaining === undefined ? "--" : Math.round(remaining)}</strong>
                <span>%</span>
                <small>剩余</small>
              </div>
            </div>
            <div className="tray-quota-copy">
              <span>{quota?.label ?? "Codex 额度"}</span>
              <strong>
                {quota ? `已使用 ${Math.round(quota.usedPercent)}%` : "额度暂不可用"}
              </strong>
              <p>{forecastLabel(snapshot?.forecastStatus ?? undefined)}</p>
            </div>
          </section>

          <section className="tray-reset-row">
            <ClockCountdown size={18} />
            <div>
              <span>下次重置</span>
              <strong>{quota ? formatReset(quota.resetsAt) : "等待额度数据"}</strong>
            </div>
          </section>

          <section className="tray-metrics" aria-label="本机用量摘要">
            <div>
              <span>最近一日 Token</span>
              <strong>{snapshot?.latestDailyTokens == null ? "--" : compactNumber.format(snapshot.latestDailyTokens)}</strong>
            </div>
            <div>
              <span>本机项目</span>
              <strong>{snapshot?.projectCount ?? 0}</strong>
              <small>{snapshot?.sessionCount ?? 0} 个会话</small>
            </div>
          </section>

          {(tray.error || snapshot?.connectionError) && (
            <p className="tray-inline-error">
              {snapshot?.connectionError ?? tray.error}
            </p>
          )}
        </>
      )}

      <button
        className="tray-primary-action"
        type="button"
        onClick={() => void backend.openDashboard()}
      >
        <ArrowSquareOut size={17} />
        打开完整界面
      </button>

      <footer className="tray-footer">
        <button
          type="button"
          onClick={() => void backend.openDashboard("settings")}
        >
          <GearSix size={16} />
          设置
        </button>
        <button type="button" onClick={() => void backend.quitApp()}>
          <Power size={16} />
          退出
        </button>
      </footer>
    </main>
  );
}

function TrayPanelSkeleton() {
  return (
    <div className="tray-panel-skeleton" aria-label="正在读取额度">
      <span />
      <div>
        <i />
        <i />
        <i />
      </div>
    </div>
  );
}
