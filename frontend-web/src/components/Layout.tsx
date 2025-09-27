import clsx from "clsx";
import type { PropsWithChildren } from "react";
import { useLocation, useNavigate } from "react-router-dom";
import "../styles/layout.css";

const STATUS_ITEMS = [
  { label: "Runs", value: "0" },
  { label: "Cost", value: "--" },
  { label: "Err", value: "0" },
  { label: "Budget", value: "0%" },
];

function WelcomeHeader() {
  return (
    <header className="welcome-header" data-testid="welcome-header">
      <div className="welcome-copy">
        <span className="welcome-greeting">Welcome, dev@local!</span>
        <div className="status-indicators" aria-label="System status">
          {STATUS_ITEMS.map((item) => (
            <span key={item.label} className="status-indicator">
              <span className="status-label">{item.label}:</span> {item.value}
            </span>
          ))}
        </div>
      </div>
      <div className="user-avatar" aria-label="User avatar" role="img">
        <span>DL</span>
      </div>
    </header>
  );
}

function StatusFooter() {
  return (
    <footer className="status-footer" data-testid="status-footer" aria-live="polite">
      <span className="status-footer-label">Status:</span> Connected
    </footer>
  );
}

export default function Layout({ children }: PropsWithChildren) {
  const navigate = useNavigate();
  const location = useLocation();

  const isDashboardRoute =
    location.pathname === "/" || location.pathname.startsWith("/dashboard");
  const isCanvasRoute = location.pathname.startsWith("/canvas");

  const handleTabClick = (path: string) => {
    navigate(path);
  };

  return (
    <div
      id="app-container"
      className={clsx("app-shell", { "canvas-view": isCanvasRoute })}
      data-testid="app-container"
    >
      <WelcomeHeader />
      <nav id="global-tabs-container" className="tabs-container">
        <button
          id="global-dashboard-tab"
          type="button"
          data-testid="global-dashboard-tab"
          className={clsx("tab-button", { active: isDashboardRoute })}
          onClick={() => handleTabClick("/dashboard")}
        >
          Agent Dashboard
        </button>
        <button
          id="global-canvas-tab"
          type="button"
          data-testid="global-canvas-tab"
          className={clsx("tab-button", { active: isCanvasRoute })}
          onClick={() => handleTabClick("/canvas")}
        >
          Canvas Editor
        </button>
      </nav>
      <main id="main-content" className="main-content-area">
        {children}
      </main>
      <StatusFooter />
    </div>
  );
}
