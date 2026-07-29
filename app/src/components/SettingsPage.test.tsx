import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, test, vi } from "vitest";
import { DEFAULT_APP_SETTINGS } from "../types/settings";
import { SettingsPage } from "./SettingsPage";

describe("SettingsPage", () => {
  test("saves startup, refresh, pace, and notification preferences", async () => {
    const onSave = vi.fn().mockResolvedValue(undefined);
    render(
      <SettingsPage
        settings={DEFAULT_APP_SETTINGS}
        onBack={() => undefined}
        onSave={onSave}
      />,
    );

    fireEvent.change(screen.getByLabelText("默认节奏指标"), {
      target: { value: "recentRate" },
    });
    fireEvent.change(screen.getByLabelText("账号额度刷新"), {
      target: { value: "5" },
    });
    fireEvent.click(screen.getByLabelText("开机启动"));
    fireEvent.click(screen.getByRole("button", { name: "保存设置" }));

    expect(onSave).toHaveBeenCalledWith(
      expect.objectContaining({
        paceMode: "recentRate",
        accountRefreshMins: 5,
        launchAtLogin: true,
      }),
    );
  });

  test("rejects a critical threshold above the warning threshold", async () => {
    const onSave = vi.fn();
    render(
      <SettingsPage
        settings={DEFAULT_APP_SETTINGS}
        onBack={() => undefined}
        onSave={onSave}
      />,
    );
    fireEvent.change(screen.getByLabelText("紧急提醒阈值"), {
      target: { value: "30" },
    });
    fireEvent.click(screen.getByRole("button", { name: "保存设置" }));
    expect(screen.getByText("紧急阈值必须低于提醒阈值")).toBeVisible();
    expect(onSave).not.toHaveBeenCalled();
  });
});
