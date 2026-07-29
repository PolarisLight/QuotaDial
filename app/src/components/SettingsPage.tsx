import { ArrowLeft, Bell, Monitor, Power, Timer } from "@phosphor-icons/react";
import { useEffect, useRef, useState } from "react";
import type { AppSettings } from "../types/settings";

export function SettingsPage({
  settings,
  onSave,
  onBack,
}: {
  settings: AppSettings;
  onSave: (settings: AppSettings) => Promise<void>;
  onBack: () => void;
}) {
  const [draft, setDraft] = useState(settings);
  const [message, setMessage] = useState("已自动保存");
  const draftRef = useRef(settings);
  const saveQueue = useRef(Promise.resolve());
  const revision = useRef(0);
  useEffect(() => {
    draftRef.current = settings;
    setDraft(settings);
  }, [settings]);

  const validationMessage = (value: AppSettings) => {
    if (
      value.quotaWarningEnabled &&
      value.quotaCriticalEnabled &&
      value.criticalRemainingPercent >= value.warningRemainingPercent
    ) {
      return "紧急阈值必须低于提醒阈值";
    }
    return null;
  };

  const patch = <K extends keyof AppSettings>(key: K, value: AppSettings[K]) => {
    const previous = draftRef.current;
    const next = { ...previous, [key]: value };
    draftRef.current = next;
    setDraft(next);
    const invalid = validationMessage(next);
    if (invalid) {
      setMessage(invalid);
      return;
    }
    const currentRevision = ++revision.current;
    setMessage("正在保存…");
    saveQueue.current = saveQueue.current
      .catch(() => undefined)
      .then(() => onSave(next))
      .then(() => {
        if (revision.current === currentRevision) setMessage("已自动保存");
      })
      .catch(error => {
        if (revision.current === currentRevision) {
          draftRef.current = previous;
          setDraft(previous);
          setMessage(
            `保存失败：${error instanceof Error ? error.message : String(error)}`,
          );
        }
      });
  };

  return (
    <div className="settings-page">
      <header className="settings-header">
        <button className="settings-back" type="button" onClick={onBack}>
          <ArrowLeft size={17} /> 返回概览
        </button>
        <span className="eyebrow">Codex Monitor</span>
        <h1>设置</h1>
        <p>控制显示方式、刷新频率与系统通知。</p>
      </header>

      <SettingsSection icon={<Monitor size={19} />} title="外观与额度">
        <SettingRow label="外观">
          <select
            aria-label="外观"
            value={draft.theme}
            onChange={event =>
              patch("theme", event.target.value as AppSettings["theme"])
            }
          >
            <option value="system">跟随系统</option>
            <option value="light">浅色</option>
            <option value="dark">深色</option>
          </select>
        </SettingRow>
        <SettingRow label="默认节奏指标">
          <select
            aria-label="默认节奏指标"
            value={draft.paceMode}
            onChange={event =>
              patch("paceMode", event.target.value as AppSettings["paceMode"])
            }
          >
            <option value="suggested">配速建议</option>
            <option value="recentRate">近期消耗率</option>
          </select>
        </SettingRow>
        <SettingRow label="月订阅价格">
          <label className="subscription-price">
            <span>US$</span>
            <input
              aria-label="月订阅价格"
              type="number"
              min={0.01}
              max={10_000}
              step={1}
              value={draft.monthlySubscriptionUsd}
              onChange={event =>
                patch("monthlySubscriptionUsd", Number(event.target.value))
              }
            />
            <span>/月</span>
          </label>
        </SettingRow>
      </SettingsSection>

      <SettingsSection icon={<Timer size={19} />} title="自动刷新">
        <SettingRow label="账号额度刷新">
          <select
            aria-label="账号额度刷新"
            value={draft.accountRefreshMins}
            onChange={event =>
              patch(
                "accountRefreshMins",
                Number(event.target.value) as 1 | 5 | 15,
              )
            }
          >
            <option value={1}>每 1 分钟</option>
            <option value={5}>每 5 分钟</option>
            <option value={15}>每 15 分钟</option>
          </select>
        </SettingRow>
        <SettingRow label="本机会话扫描">
          <select
            aria-label="本机会话扫描"
            value={draft.sessionScanMins}
            onChange={event =>
              patch(
                "sessionScanMins",
                Number(event.target.value) as 5 | 10 | 30,
              )
            }
          >
            <option value={5}>每 5 分钟</option>
            <option value={10}>每 10 分钟</option>
            <option value={30}>每 30 分钟</option>
          </select>
        </SettingRow>
      </SettingsSection>

      <SettingsSection icon={<Power size={19} />} title="启动">
        <Toggle
          label="开机启动"
          checked={draft.launchAtLogin}
          onChange={value => patch("launchAtLogin", value)}
        />
      </SettingsSection>

      <SettingsSection icon={<Bell size={19} />} title="通知">
        <Threshold
          label="额度不足提醒"
          inputLabel="额度提醒阈值"
          checked={draft.quotaWarningEnabled}
          value={draft.warningRemainingPercent}
          onChecked={value => patch("quotaWarningEnabled", value)}
          onValue={value => patch("warningRemainingPercent", value)}
        />
        <Threshold
          label="额度紧急提醒"
          inputLabel="紧急提醒阈值"
          checked={draft.quotaCriticalEnabled}
          value={draft.criticalRemainingPercent}
          onChecked={value => patch("quotaCriticalEnabled", value)}
          onValue={value => patch("criticalRemainingPercent", value)}
        />
        <Toggle
          label="额度重置完成"
          checked={draft.resetNotificationEnabled}
          onChange={value => patch("resetNotificationEnabled", value)}
        />
        <Toggle
          label={`数据超过 ${draft.staleAfterMins} 分钟未更新`}
          checked={draft.staleNotificationEnabled}
          onChange={value => patch("staleNotificationEnabled", value)}
        />
      </SettingsSection>

      <div className="settings-save-row">
        <span role="status">{message}</span>
      </div>
    </div>
  );
}

function SettingsSection({
  icon,
  title,
  children,
}: {
  icon: React.ReactNode;
  title: string;
  children: React.ReactNode;
}) {
  return (
    <section className="settings-section">
      <h2>{icon}{title}</h2>
      <div>{children}</div>
    </section>
  );
}

function SettingRow({
  label,
  children,
}: {
  label: string;
  children: React.ReactNode;
}) {
  return (
    <div className="setting-row">
      <span>{label}</span>
      {children}
    </div>
  );
}

function Toggle({
  label,
  checked,
  onChange,
}: {
  label: string;
  checked: boolean;
  onChange: (checked: boolean) => void;
}) {
  return (
    <label className="setting-row setting-toggle">
      <span>{label}</span>
      <input
        aria-label={label}
        type="checkbox"
        checked={checked}
        onChange={event => onChange(event.target.checked)}
      />
    </label>
  );
}

function Threshold({
  label,
  inputLabel,
  checked,
  value,
  onChecked,
  onValue,
}: {
  label: string;
  inputLabel: string;
  checked: boolean;
  value: number;
  onChecked: (checked: boolean) => void;
  onValue: (value: number) => void;
}) {
  return (
    <div className="setting-row notification-threshold">
      <label>
        <input
          type="checkbox"
          checked={checked}
          onChange={event => onChecked(event.target.checked)}
        />
        <span>{label}</span>
      </label>
      <span>
        剩余
        <input
          aria-label={inputLabel}
          type="number"
          min={1}
          max={99}
          value={value}
          onChange={event => onValue(Number(event.target.value))}
        />
        %
      </span>
    </div>
  );
}
