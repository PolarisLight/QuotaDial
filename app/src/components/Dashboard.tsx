import { ArrowClockwise, CloudSlash, WarningCircle } from "@phosphor-icons/react";
import { useEffect, useState } from "react";
import { useDashboard } from "../hooks/useDashboard";
import { backend } from "../lib/backend";
import type { DashboardSnapshot } from "../types/dashboard";
import {
  DEFAULT_APP_SETTINGS,
  type AppSettings,
} from "../types/settings";
import { AppSidebar } from "./AppSidebar";
import { QuotaCard } from "./QuotaCard";
import { SessionDetails } from "./SessionDetails";
import { SettingsPage } from "./SettingsPage";
import { UsageForecastPanel } from "./UsageForecastPanel";

interface DashboardViewProps {
  snapshot: DashboardSnapshot | null;
  loading: boolean;
  refreshing: boolean;
  error: string | null;
  onRefresh: () => void;
  settings?: AppSettings;
  version?: string;
  destination?: "overview" | "settings";
  onNavigate?: (destination: "overview" | "settings") => void;
  onSaveSettings?: (settings: AppSettings) => Promise<void>;
}

export function DashboardView({
  snapshot,
  loading,
  refreshing,
  error,
  onRefresh,
  settings = DEFAULT_APP_SETTINGS,
  version = "0.1.0",
  destination = "overview",
  onNavigate = () => undefined,
  onSaveSettings = async () => undefined,
}: DashboardViewProps) {
  return (
    <div
      className="app-window"
      style={{ width: "100vw", height: "100vh", margin: 0, overflow: "hidden" }}
    >
      <AppSidebar
        destination={destination}
        version={version}
        onNavigate={onNavigate}
      />
      <main
        className="content"
        style={{
          height: "100%",
          overflowY: "auto",
          overscrollBehaviorY: "none",
        }}
      >
        {destination === "settings" ? (
          <SettingsPage
            settings={settings}
            onBack={() => onNavigate("overview")}
            onSave={onSaveSettings}
          />
        ) : (
          <>
            <header className="page-header">
              <div>
                <span className="eyebrow">Codex 使用情况</span>
                <h1>使用概览</h1>
                <p>账号额度与 Token，覆盖所有设备。</p>
              </div>
              {snapshot && (
                <div
                  className={`connection-status ${snapshot.isStale ? "stale" : ""}`}
                >
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
                    <button
                      type="button"
                      onClick={onRefresh}
                      disabled={refreshing}
                    >
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
                    history={snapshot.quotaHistory}
                    pace={snapshot.quotaPace}
                    quota={snapshot.primaryQuota}
                    observedAt={snapshot.observedAt}
                    paceMode={settings.paceMode}
                    onPaceModeChange={paceMode =>
                      void onSaveSettings({ ...settings, paceMode })
                    }
                  />
                </section>
                <SessionDetails view={snapshot.localSessions} />
              </>
            )}
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
  const [destination, setDestination] = useState<"overview" | "settings">(
    "overview",
  );
  const [settings, setSettings] = useState(DEFAULT_APP_SETTINGS);
  const [version, setVersion] = useState("0.1.0");
  useEffect(() => {
    void backend.getAppSettings().then(setSettings);
    void backend.getAppVersion().then(setVersion);
  }, []);
  useEffect(() => {
    if (settings.theme === "system") {
      delete document.documentElement.dataset.theme;
    } else {
      document.documentElement.dataset.theme = settings.theme;
    }
  }, [settings.theme]);
  useEffect(() => {
    let active = true;
    let unlisten: (() => void) | undefined;
    void backend.onOpenSettings(() => setDestination("settings")).then(value => {
      if (active) unlisten = value;
      else value();
    });
    return () => {
      active = false;
      unlisten?.();
    };
  }, []);
  useEffect(() => {
    let active = true;
    let unlisten: (() => void) | undefined;
    void backend.onFocusSection(section => focusDashboardSection(section)).then(
      dispose => {
        if (active) {
          unlisten = dispose;
        } else {
          dispose();
        }
      },
    );
    return () => {
      active = false;
      unlisten?.();
    };
  }, []);

  return (
    <DashboardView
      snapshot={dashboard.snapshot}
      loading={dashboard.loading}
      refreshing={dashboard.refreshing}
      error={dashboard.error}
      onRefresh={() => void dashboard.refresh()}
      settings={settings}
      version={version}
      destination={destination}
      onNavigate={setDestination}
      onSaveSettings={async value => {
        const saved = await backend.saveAppSettings(value);
        setSettings(saved);
      }}
    />
  );
}

export function focusDashboardSection(section: string) {
  const target = document.querySelector<HTMLElement>(
    `[data-section="${section}"]`,
  );
  if (!target) return;
  target.scrollIntoView?.({ behavior: "smooth", block: "center" });
  target.classList.add("section-focused");
  window.setTimeout(() => target.classList.remove("section-focused"), 1_400);
}
