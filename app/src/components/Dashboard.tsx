import { ArrowClockwise, CloudSlash, WarningCircle } from "@phosphor-icons/react";
import { useDashboard } from "../hooks/useDashboard";
import type { DashboardSnapshot } from "../types/dashboard";
import { AppSidebar } from "./AppSidebar";
import { QuotaCard } from "./QuotaCard";
import { SessionDetails } from "./SessionDetails";
import { UsageForecastPanel } from "./UsageForecastPanel";

interface DashboardViewProps {
  snapshot: DashboardSnapshot | null;
  loading: boolean;
  refreshing: boolean;
  error: string | null;
  onRefresh: () => void;
}

export function DashboardView({
  snapshot,
  loading,
  refreshing,
  error,
  onRefresh,
}: DashboardViewProps) {
  return (
    <div className="app-window">
      <AppSidebar />
      <main className="content">
        <header className="page-header">
          <div>
            <span className="eyebrow">Codex 使用情况</span>
            <h1>使用概览</h1>
            <p>账号额度与 Token，覆盖所有设备。</p>
          </div>
          {snapshot && (
            <div className={`connection-status ${snapshot.isStale ? "stale" : ""}`}>
              <span />
              {snapshot.isStale ? "数据已过期" : "账号已连接"}
            </div>
          )}
        </header>

        {loading && !snapshot ? (
          <DashboardSkeleton />
        ) : !snapshot ? (
          <ConnectionError error={error} onRefresh={onRefresh} />
        ) : (
          <>
            {(snapshot.connectionError || error) && (
              <div className="status-banner" role="status">
                <WarningCircle size={18} />
                <span>{snapshot.connectionError ?? error}</span>
                <button type="button" onClick={onRefresh} disabled={refreshing}>
                  <ArrowClockwise size={16} />
                  重试
                </button>
              </div>
            )}

            <section className="account-grid" aria-label="账号使用概览">
              {snapshot.primaryQuota ? (
                <QuotaCard
                  quota={snapshot.primaryQuota}
                  observedAt={snapshot.observedAt}
                  refreshing={refreshing}
                  onRefresh={onRefresh}
                />
              ) : (
                <div className="panel inline-empty">
                  <strong>账号额度暂不可用</strong>
                </div>
              )}
              <UsageForecastPanel
                usage={snapshot.accountUsage}
                usageError={snapshot.accountUsageError}
                forecast={snapshot.forecast}
              />
            </section>
            <SessionDetails view={snapshot.localSessions} />
          </>
        )}
      </main>
    </div>
  );
}

function DashboardSkeleton() {
  return (
    <div className="dashboard-skeleton" aria-label="正在加载账号数据">
      <div className="skeleton-card">
        <i />
        <b />
        <span />
        <span />
      </div>
      <div className="skeleton-card">
        <i />
        <b />
        <span />
        <span />
      </div>
      <div className="skeleton-wide" />
    </div>
  );
}

function ConnectionError({
  error,
  onRefresh,
}: {
  error: string | null;
  onRefresh: () => void;
}) {
  return (
    <section className="connection-error">
      <span>
        <CloudSlash size={28} />
      </span>
      <h2>无法读取 Codex 账号</h2>
      <p>{error ?? "请确认 Codex CLI 已安装并登录。"}</p>
      <button className="primary-button" type="button" onClick={onRefresh}>
        重新连接
      </button>
    </section>
  );
}

export function Dashboard() {
  const dashboard = useDashboard();
  return (
    <DashboardView
      snapshot={dashboard.snapshot}
      loading={dashboard.loading}
      refreshing={dashboard.refreshing}
      error={dashboard.error}
      onRefresh={() => void dashboard.refresh()}
    />
  );
}
