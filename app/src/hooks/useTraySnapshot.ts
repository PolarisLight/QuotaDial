import { useCallback, useEffect, useState } from "react";
import { backend } from "../lib/backend";
import type { TrayPanelSnapshot } from "../types/dashboard";

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

export function useTraySnapshot() {
  const [snapshot, setSnapshot] = useState<TrayPanelSnapshot | null>(null);
  const [loading, setLoading] = useState(true);
  const [refreshing, setRefreshing] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let active = true;
    let unlisten: (() => void) | undefined;

    const applySnapshot = (value: TrayPanelSnapshot) => {
      if (!active) return;
      setSnapshot(current =>
        current && current.observedAt > value.observedAt ? current : value,
      );
      setError(null);
      setLoading(false);
    };

    const syncFromMemory = () => {
      void backend
        .getTraySnapshot()
        .then(applySnapshot)
        .catch(cause => {
          if (active) setError(errorMessage(cause));
        })
        .finally(() => {
          if (active) setLoading(false);
        });
    };

    const syncWhenVisible = () => {
      if (document.visibilityState === "visible") syncFromMemory();
    };

    syncFromMemory();
    window.addEventListener("focus", syncFromMemory);
    document.addEventListener("visibilitychange", syncWhenVisible);

    void backend
      .onTrayUpdated(value => {
        applySnapshot(value);
      })
      .then(cleanup => {
        if (active) unlisten = cleanup;
        else cleanup();
      })
      .catch(cause => {
        if (active) setError(errorMessage(cause));
      });

    return () => {
      active = false;
      window.removeEventListener("focus", syncFromMemory);
      document.removeEventListener("visibilitychange", syncWhenVisible);
      unlisten?.();
    };
  }, []);

  const refresh = useCallback(async () => {
    setRefreshing(true);
    setError(null);
    try {
      setSnapshot(await backend.refreshTraySnapshot());
    } catch (cause) {
      setError(errorMessage(cause));
    } finally {
      setRefreshing(false);
    }
  }, []);

  return { snapshot, loading, refreshing, error, refresh };
}
