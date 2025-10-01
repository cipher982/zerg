import { useEffect } from "react";
import { useRoutes } from "react-router-dom";
import Layout from "../components/Layout";
import DashboardPage from "../pages/DashboardPage";
import ChatPage from "../pages/ChatPage";
import CanvasPage from "../pages/CanvasPage";
import ProfilePage from "../pages/ProfilePage";
import AdminPage from "../pages/AdminPage";
import { AuthGuard } from "../lib/auth";
import { ErrorBoundary } from "../components/ErrorBoundary";
import { usePerformanceMonitoring, useBundleSizeWarning } from "../lib/usePerformance";
import config from "../lib/config";

export default function App() {
  // Performance monitoring
  usePerformanceMonitoring('App');
  useBundleSizeWarning();

  useEffect(() => {
    // Signal to Playwright/legacy helpers that the React app finished booting.
    if (typeof window !== "undefined") {
      (window as typeof window & { __APP_READY__?: boolean }).__APP_READY__ = true;
    }
  }, []);

  const routes = useRoutes([
    {
      path: "/dashboard",
      element: (
        <ErrorBoundary>
          <DashboardPage />
        </ErrorBoundary>
      )
    },
    {
      path: "/canvas",
      element: (
        <ErrorBoundary>
          <CanvasPage />
        </ErrorBoundary>
      )
    },
    {
      path: "/chat/:agentId?/:threadId?",
      element: (
        <ErrorBoundary>
          <ChatPage />
        </ErrorBoundary>
      )
    },
    {
      path: "/profile",
      element: (
        <ErrorBoundary>
          <ProfilePage />
        </ErrorBoundary>
      )
    },
    {
      path: "/admin",
      element: (
        <ErrorBoundary>
          <AdminPage />
        </ErrorBoundary>
      )
    },
    {
      path: "*",
      element: (
        <ErrorBoundary>
          <DashboardPage />
        </ErrorBoundary>
      )
    },
  ]);

  // Get Google Client ID from environment or use default from .env
  const googleClientId = import.meta.env.VITE_GOOGLE_CLIENT_ID || "658453123272-gt664mlo8q3pra3u1h3oflbmrdi94lld.apps.googleusercontent.com";

  return (
    <AuthGuard clientId={googleClientId}>
      <Layout>{routes}</Layout>
    </AuthGuard>
  );
}
