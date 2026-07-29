import { useCallback, useEffect, useState } from "react";
import { backend } from "../lib/backend";
import type { DashboardSnapshot } from "../types/dashboard";

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

export function useDashboard() {
  const [snapshot, setSnapshot] = useState<DashboardSnapshot | null>(null);
  const [loading, setLoading] = useState(true);
  const [refreshing, setRefreshing] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let active = true;
    let unlisten: (() => void) | undefined;

    void backend
      .getDashboardSnapshot()
      .then(value => {
        if (active) {
          setSnapshot(value);
          setError(null);
        }
      })
      .catch(cause => {
        if (active) {
          setError(errorMessage(cause));
        }
      })
      .finally(() => {
        if (active) {
          setLoading(false);
        }
      });

    void backend
      .onDashboardUpdated(value => {
        if (active) {
          setSnapshot(value);
          setError(null);
          setLoading(false);
        }
      })
      .then(cleanup => {
        if (active) {
          unlisten = cleanup;
        } else {
          cleanup();
        }
      })
      .catch(cause => {
        if (active) {
          setError(errorMessage(cause));
        }
      });

    return () => {
      active = false;
      unlisten?.();
    };
  }, []);

  const refresh = useCallback(async () => {
    setRefreshing(true);
    setError(null);
    try {
      setSnapshot(await backend.refreshAccount());
    } catch (cause) {
      setError(errorMessage(cause));
    } finally {
      setRefreshing(false);
    }
  }, []);

  return {
    snapshot,
    loading,
    refreshing,
    error,
    refresh,
  };
}
