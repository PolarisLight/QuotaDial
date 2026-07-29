export type ThemePreference = "system" | "light" | "dark";
export type PaceMode = "suggested" | "recentRate";

export interface AppSettings {
  theme: ThemePreference;
  paceMode: PaceMode;
  accountRefreshMins: 1 | 5 | 15;
  sessionScanMins: 5 | 10 | 30;
  monthlySubscriptionUsd: number;
  launchAtLogin: boolean;
  quotaWarningEnabled: boolean;
  warningRemainingPercent: number;
  quotaCriticalEnabled: boolean;
  criticalRemainingPercent: number;
  resetNotificationEnabled: boolean;
  staleNotificationEnabled: boolean;
  staleAfterMins: number;
}

export const DEFAULT_APP_SETTINGS: AppSettings = {
  theme: "system",
  paceMode: "suggested",
  accountRefreshMins: 1,
  sessionScanMins: 10,
  monthlySubscriptionUsd: 20,
  launchAtLogin: false,
  quotaWarningEnabled: true,
  warningRemainingPercent: 25,
  quotaCriticalEnabled: true,
  criticalRemainingPercent: 10,
  resetNotificationEnabled: true,
  staleNotificationEnabled: true,
  staleAfterMins: 15,
};
