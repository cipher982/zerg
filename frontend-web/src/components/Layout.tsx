import clsx from "clsx";
import type { PropsWithChildren } from "react";
import { useLocation, useNavigate } from "react-router-dom";
import "../styles/layout.css";

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
      className={clsx({ "canvas-view": isCanvasRoute })}
      data-testid="app-container"
    >
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
    </div>
  );
}
