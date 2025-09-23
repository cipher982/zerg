import React from 'react';
import { useLocation, useNavigate } from 'react-router-dom';
import '../styles/layout.css';

interface LayoutProps {
  children: React.ReactNode;
}

export default function Layout({ children }: LayoutProps) {
  const navigate = useNavigate();
  const location = useLocation();

  const isCanvasPage = location.pathname.startsWith('/canvas');
  const isDashboardPage = location.pathname === '/' || location.pathname.startsWith('/dashboard');

  const handleTabClick = (path: string) => {
    navigate(path);
  };

  return (
    <div className="app-layout">
      <nav className="global-nav">
        <div className="nav-tabs">
          <button
            data-testid="global-dashboard-tab"
            className={`nav-tab ${isDashboardPage ? 'active' : ''}`}
            onClick={() => handleTabClick('/dashboard')}
          >
            Agent Dashboard
          </button>
          <button
            data-testid="global-canvas-tab"
            className={`nav-tab ${isCanvasPage ? 'active' : ''}`}
            onClick={() => handleTabClick('/canvas')}
          >
            Canvas
          </button>
        </div>
      </nav>
      <main className="main-content">
        {children}
      </main>
    </div>
  );
}