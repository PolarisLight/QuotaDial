import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { useState } from "react";
import { afterEach, describe, expect, test, vi } from "vitest";
import { DEFAULT_APP_SETTINGS } from "../types/settings";
import { SettingsPage } from "./SettingsPage";

describe("SettingsPage", () => {
  afterEach(() => vi.useRealTimers());

  function SettingsHarness() {
    const [settings, setSettings] = useState(DEFAULT_APP_SETTINGS);
    const [open, setOpen] = useState(true);
    return open ? (
      <SettingsPage
        settings={settings}
        onBack={() => setOpen(false)}
        onSave={async value => setSettings(value)}
      />
    ) : (
      <button type="button" onClick={() => setOpen(true)}>
        重新打开设置
      </button>
    );
  }

  test("persists a startup toggle immediately without a separate save button", async () => {
    const onSave = vi.fn().mockResolvedValue(undefined);
    render(
      <SettingsPage
        settings={DEFAULT_APP_SETTINGS}
        onBack={() => undefined}
        onSave={onSave}
      />,
    );

    fireEvent.click(screen.getByLabelText("开机启动"));

    await waitFor(() =>
      expect(onSave).toHaveBeenCalledWith(
        expect.objectContaining({ launchAtLogin: true }),
      ),
    );
    expect(screen.queryByRole("button", { name: "保存设置" })).toBeNull();
  });

  test("shows save confirmation only after a real save and then hides it", async () => {
    vi.useFakeTimers();
    const onSave = vi.fn().mockResolvedValue(undefined);
    render(
      <SettingsPage
        settings={DEFAULT_APP_SETTINGS}
        onBack={() => undefined}
        onSave={onSave}
      />,
    );
    expect(screen.queryByText("已保存")).toBeNull();
    fireEvent.change(screen.getByLabelText("月订阅价格"), {
      target: { value: "200" },
    });
    await act(async () => {
      await Promise.resolve();
      await Promise.resolve();
      await Promise.resolve();
    });
    expect(onSave).toHaveBeenCalledWith(
      expect.objectContaining({ monthlySubscriptionUsd: 200 }),
    );
    expect(screen.getByText("已保存")).toBeVisible();

    act(() => vi.advanceTimersByTime(2_000));
    expect(screen.queryByText("已保存")).toBeNull();
  });

  test("keeps launch at login enabled after leaving and reopening settings", async () => {
    render(<SettingsHarness />);
    fireEvent.click(screen.getByLabelText("开机启动"));
    await waitFor(() =>
      expect(screen.getByText("已保存")).toBeVisible(),
    );
    fireEvent.click(screen.getByRole("button", { name: "返回概览" }));
    fireEvent.click(screen.getByRole("button", { name: "重新打开设置" }));
    expect(screen.getByLabelText("开机启动")).toBeChecked();
  });

  test("reverts the launch toggle when the system rejects the change", async () => {
    render(
      <SettingsPage
        settings={DEFAULT_APP_SETTINGS}
        onBack={() => undefined}
        onSave={async () => {
          throw new Error("无法写入登录项");
        }}
      />,
    );
    fireEvent.click(screen.getByLabelText("开机启动"));
    await waitFor(() =>
      expect(screen.getByText("保存失败：无法写入登录项")).toBeVisible(),
    );
    expect(screen.getByLabelText("开机启动")).not.toBeChecked();
  });

  test("rejects a critical threshold above the warning threshold", () => {
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
    expect(screen.getByText("紧急阈值必须低于提醒阈值")).toBeVisible();
    expect(onSave).not.toHaveBeenCalled();
  });
});
