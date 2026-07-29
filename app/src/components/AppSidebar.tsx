import { ChartDonut, SlidersHorizontal } from "@phosphor-icons/react";
import { BrandMark } from "./BrandMark";

export function AppSidebar({
  destination,
  version,
  onNavigate,
}: {
  destination: "overview" | "settings";
  version: string;
  onNavigate: (destination: "overview" | "settings") => void;
}) {
  return (
    <aside className="sidebar">
      <div className="brand">
        <span className="brand-mark">
          <BrandMark />
        </span>
        <span>
          <strong>Codex</strong>
          <small>Monitor</small>
        </span>
      </div>

      <nav className="sidebar-nav" aria-label="主导航">
        <button
          className={`nav-item ${destination === "overview" ? "active" : ""}`}
          type="button"
          onClick={() => onNavigate("overview")}
        >
          <ChartDonut size={18} weight="fill" />
          使用概览
        </button>
        <button
          className={`nav-item ${destination === "settings" ? "active" : ""}`}
          type="button"
          onClick={() => onNavigate("settings")}
        >
          <SlidersHorizontal size={18} />
          设置
        </button>
      </nav>

      <div className="sidebar-footer">
        <p>Codex Monitor v{version}</p>
        <a
          href="https://yetform.cyhao.space/"
          target="_blank"
          rel="noreferrer"
        >
          © 2026 yetform
        </a>
      </div>
    </aside>
  );
}
