import {
  ChartDonut,
  Moon,
  SlidersHorizontal,
  Sun,
  Waveform,
} from "@phosphor-icons/react";
import { useEffect, useState } from "react";

type Theme = "light" | "dark" | "system";

function initialTheme(): Theme {
  const saved = localStorage.getItem("codex-monitor-theme");
  return saved === "light" || saved === "dark" ? saved : "system";
}

export function AppSidebar() {
  const [theme, setTheme] = useState<Theme>(initialTheme);

  useEffect(() => {
    if (theme === "system") {
      delete document.documentElement.dataset.theme;
      localStorage.removeItem("codex-monitor-theme");
    } else {
      document.documentElement.dataset.theme = theme;
      localStorage.setItem("codex-monitor-theme", theme);
    }
  }, [theme]);

  const dark =
    theme === "dark" ||
    (theme === "system" &&
      window.matchMedia?.("(prefers-color-scheme: dark)").matches);

  return (
    <aside className="sidebar">
      <div className="brand" aria-label="Codex Monitor">
        <span className="brand-mark">
          <Waveform size={18} weight="bold" />
        </span>
        <span>
          <strong>Codex</strong>
          <small>Monitor</small>
        </span>
      </div>

      <nav className="sidebar-nav" aria-label="主导航">
        <button className="nav-item active" type="button">
          <ChartDonut size={18} weight="fill" />
          使用概览
        </button>
        <button className="nav-item" type="button" disabled>
          <SlidersHorizontal size={18} />
          设置
          <span className="nav-note">稍后</span>
        </button>
      </nav>

      <div className="sidebar-footer">
        <button
          className="theme-toggle"
          type="button"
          aria-label={dark ? "切换到浅色外观" : "切换到深色外观"}
          onClick={() => setTheme(dark ? "light" : "dark")}
        >
          {dark ? <Sun size={17} /> : <Moon size={17} />}
          {dark ? "浅色外观" : "深色外观"}
        </button>
        <p>账号级监控</p>
        <span>覆盖所有设备</span>
      </div>
    </aside>
  );
}
