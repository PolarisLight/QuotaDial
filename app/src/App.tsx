import { Dashboard } from "./components/Dashboard";
import { TrayPanel } from "./components/TrayPanel";

const windowView = new URLSearchParams(window.location.search).get("view");
document.documentElement.dataset.window =
  windowView === "tray" ? "tray" : "main";

export default function App() {
  return windowView === "tray" ? <TrayPanel /> : <Dashboard />;
}
