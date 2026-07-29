import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { DashboardSnapshot } from "../types/dashboard";

export const backend = {
  getDashboardSnapshot: () =>
    invoke<DashboardSnapshot>("get_dashboard_snapshot"),
  refreshAccount: () => invoke<DashboardSnapshot>("refresh_account"),
  onDashboardUpdated: (
    handler: (snapshot: DashboardSnapshot) => void,
  ): Promise<UnlistenFn> =>
    listen<DashboardSnapshot>("dashboard://updated", event =>
      handler(event.payload),
    ),
};
