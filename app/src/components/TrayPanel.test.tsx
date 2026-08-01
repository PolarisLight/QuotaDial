import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, describe, expect, test, vi } from "vitest";
import { useTraySnapshot } from "../hooks/useTraySnapshot";
import { backend } from "../lib/backend";
import type { TrayPanelSnapshot } from "../types/dashboard";
import { DEFAULT_APP_SETTINGS } from "../types/settings";
import { TrayPanel } from "./TrayPanel";

vi.mock("../hooks/useTraySnapshot", () => ({
  useTraySnapshot: vi.fn(),
}));

vi.mock("../lib/backend", () => ({
  backend: {
    getAppSettings: vi.fn(),
    hideTrayPanel: vi.fn(),
    openDashboard: vi.fn(),
    quitApp: vi.fn(),
  },
}));

const snapshot: TrayPanelSnapshot = {
  observedAt: 1_785_330_000,
  isStale: false,
  connectionError: null,
  primaryQuota: {
    limitId: "codex",
    label: "7 日额度",
    windowKind: "primary",
    usedPercent: 18,
    remainingPercent: 82,
    windowDurationMins: 10_080,
    resetsAt: 1_785_903_626,
    planType: "plus",
  },
  forecastStatus: "survivesWindow",
  latestDailyTokens: 550_000,
  projectCount: 1,
  sessionCount: 2,
};

const mockedTraySnapshot = vi.mocked(useTraySnapshot);
const mockedBackend = vi.mocked(backend);

beforeEach(() => {
  vi.clearAllMocks();
  mockedBackend.getAppSettings.mockResolvedValue(DEFAULT_APP_SETTINGS);
  mockedBackend.openDashboard.mockResolvedValue();
  mockedBackend.hideTrayPanel.mockResolvedValue();
  mockedBackend.quitApp.mockResolvedValue();
  mockedTraySnapshot.mockReturnValue({
    snapshot,
    loading: false,
    refreshing: false,
    error: null,
    refresh: vi.fn(),
  });
});

describe("TrayPanel", () => {
  test("shows brand, quota and the precomputed project summary", async () => {
    render(<TrayPanel />);

    expect(screen.getByRole("img", { name: "QuotaDial 额度表盘" })).toBeVisible();
    expect(screen.getByText("82")).toBeVisible();
    expect(screen.getByText("已使用 18%")).toBeVisible();
    expect(screen.getByText("2 个会话")).toBeVisible();
    const projectsMetric = screen.getByText("本机项目").parentElement!;
    expect(within(projectsMetric).getByText("1")).toBeVisible();
    await waitFor(() => expect(mockedBackend.getAppSettings).toHaveBeenCalledOnce());
  });

  test("routes primary and secondary actions through the backend", () => {
    render(<TrayPanel />);

    fireEvent.click(screen.getByRole("button", { name: "打开完整界面" }));
    fireEvent.click(screen.getByRole("button", { name: "设置" }));
    fireEvent.click(screen.getByRole("button", { name: "退出" }));

    expect(mockedBackend.openDashboard).toHaveBeenNthCalledWith(1);
    expect(mockedBackend.openDashboard).toHaveBeenNthCalledWith(2, "settings");
    expect(mockedBackend.quitApp).toHaveBeenCalledOnce();
  });
});
