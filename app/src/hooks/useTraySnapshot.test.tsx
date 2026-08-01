import { act, renderHook, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, test, vi } from "vitest";
import { backend } from "../lib/backend";
import type { TrayPanelSnapshot } from "../types/dashboard";
import { useTraySnapshot } from "./useTraySnapshot";

vi.mock("../lib/backend", () => ({
  backend: {
    getTraySnapshot: vi.fn(),
    refreshTraySnapshot: vi.fn(),
    onTrayUpdated: vi.fn(),
  },
}));

const snapshot: TrayPanelSnapshot = {
  observedAt: 1_000,
  isStale: false,
  connectionError: null,
  primaryQuota: null,
  forecastStatus: null,
  latestDailyTokens: null,
  projectCount: 68,
  sessionCount: 2_324,
};

const mockedBackend = vi.mocked(backend);

afterEach(() => {
  vi.clearAllMocks();
});

describe("useTraySnapshot", () => {
  test("loads and subscribes only to compact tray snapshots", async () => {
    let update: ((value: TrayPanelSnapshot) => void) | undefined;
    mockedBackend.getTraySnapshot.mockResolvedValue(snapshot);
    mockedBackend.onTrayUpdated.mockImplementation(async handler => {
      update = handler;
      return vi.fn(() => undefined);
    });

    const { result } = renderHook(() => useTraySnapshot());
    await waitFor(() => expect(result.current.snapshot).toEqual(snapshot));

    act(() => update?.({ ...snapshot, sessionCount: 2_325 }));
    expect(result.current.snapshot?.sessionCount).toBe(2_325);
  });

  test("refreshes through the compact tray command", async () => {
    mockedBackend.getTraySnapshot.mockResolvedValue(snapshot);
    mockedBackend.onTrayUpdated.mockResolvedValue(vi.fn(() => undefined));
    mockedBackend.refreshTraySnapshot.mockResolvedValue({
      ...snapshot,
      observedAt: 2_000,
    });
    const { result } = renderHook(() => useTraySnapshot());
    await waitFor(() => expect(result.current.loading).toBe(false));

    await act(() => result.current.refresh());

    expect(mockedBackend.refreshTraySnapshot).toHaveBeenCalledOnce();
    expect(result.current.snapshot?.observedAt).toBe(2_000);
  });

  test("re-synchronizes the in-memory snapshot whenever the flyout gains focus", async () => {
    mockedBackend.getTraySnapshot
      .mockResolvedValueOnce(snapshot)
      .mockResolvedValueOnce({ ...snapshot, observedAt: 3_000, projectCount: 69 });
    mockedBackend.onTrayUpdated.mockResolvedValue(vi.fn(() => undefined));
    const { result } = renderHook(() => useTraySnapshot());
    await waitFor(() => expect(result.current.snapshot).toEqual(snapshot));

    act(() => window.dispatchEvent(new Event("focus")));

    await waitFor(() => expect(result.current.snapshot?.observedAt).toBe(3_000));
    expect(result.current.snapshot?.projectCount).toBe(69);
    expect(mockedBackend.getTraySnapshot).toHaveBeenCalledTimes(2);
  });

  test("does not let an older invocation overwrite a newer pushed snapshot", async () => {
    let resolveInitial: ((value: TrayPanelSnapshot) => void) | undefined;
    mockedBackend.getTraySnapshot.mockReturnValue(
      new Promise(resolve => {
        resolveInitial = resolve;
      }),
    );
    let update: ((value: TrayPanelSnapshot) => void) | undefined;
    mockedBackend.onTrayUpdated.mockImplementation(async handler => {
      update = handler;
      return vi.fn(() => undefined);
    });
    const { result } = renderHook(() => useTraySnapshot());
    await waitFor(() => expect(update).toBeDefined());

    act(() => update?.({ ...snapshot, observedAt: 4_000 }));
    await act(async () => resolveInitial?.(snapshot));

    expect(result.current.snapshot?.observedAt).toBe(4_000);
  });
});
