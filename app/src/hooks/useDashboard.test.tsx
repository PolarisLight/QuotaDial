import { act, renderHook, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, test, vi } from "vitest";
import type { DashboardSnapshot } from "../types/dashboard";
import { backend } from "../lib/backend";
import { useDashboard } from "./useDashboard";

vi.mock("../lib/backend", () => ({
  backend: {
    getDashboardSnapshot: vi.fn(),
    refreshAccount: vi.fn(),
    onDashboardUpdated: vi.fn(),
  },
}));

const snapshot: DashboardSnapshot = {
  observedAt: 1_000,
  isStale: false,
  connectionError: null,
  accountUsageError: null,
  primaryQuota: {
    limitId: "codex",
    label: "7 日额度",
    windowKind: "primary",
    usedPercent: 18,
    remainingPercent: 82,
    windowDurationMins: 10_080,
    resetsAt: 200_000,
    planType: null,
  },
  otherQuotas: [],
  accountUsage: null,
  forecast: null,
  sessionDetailsAvailable: false,
};

const mockedBackend = vi.mocked(backend);

afterEach(() => {
  vi.clearAllMocks();
});

describe("useDashboard", () => {
  test("starts in loading state and then exposes the initial snapshot", async () => {
    mockedBackend.getDashboardSnapshot.mockResolvedValue(snapshot);
    mockedBackend.onDashboardUpdated.mockResolvedValue(vi.fn(() => undefined));

    const { result } = renderHook(() => useDashboard());
    expect(result.current.loading).toBe(true);

    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.snapshot).toEqual(snapshot);
  });

  test("replaces the snapshot when the backend emits an update", async () => {
    let update: ((value: DashboardSnapshot) => void) | undefined;
    mockedBackend.getDashboardSnapshot.mockResolvedValue(snapshot);
    mockedBackend.onDashboardUpdated.mockImplementation(async handler => {
      update = handler;
      return vi.fn(() => undefined);
    });
    const { result } = renderHook(() => useDashboard());
    await waitFor(() => expect(update).toBeDefined());

    act(() => update?.({ ...snapshot, observedAt: 2_000 }));

    expect(result.current.snapshot?.observedAt).toBe(2_000);
  });

  test("keeps prior values when a manual refresh fails", async () => {
    mockedBackend.getDashboardSnapshot.mockResolvedValue(snapshot);
    mockedBackend.onDashboardUpdated.mockResolvedValue(vi.fn(() => undefined));
    mockedBackend.refreshAccount.mockRejectedValue(new Error("offline"));
    const { result } = renderHook(() => useDashboard());
    await waitFor(() => expect(result.current.snapshot).toEqual(snapshot));

    await act(() => result.current.refresh());

    expect(result.current.snapshot).toEqual(snapshot);
    expect(result.current.error).toBe("offline");
  });

  test("unsubscribes from Tauri events on cleanup", async () => {
    const unlisten = vi.fn(() => undefined);
    mockedBackend.getDashboardSnapshot.mockResolvedValue(snapshot);
    mockedBackend.onDashboardUpdated.mockResolvedValue(unlisten);
    const { unmount } = renderHook(() => useDashboard());
    await waitFor(() =>
      expect(mockedBackend.onDashboardUpdated).toHaveBeenCalledOnce(),
    );

    unmount();

    expect(unlisten).toHaveBeenCalledOnce();
  });
});
