import { useEffect } from "react";
import { useRoutes } from "react-router-dom";
import Layout from "../components/Layout";
import DashboardPage from "../pages/DashboardPage";
import ChatPage from "../pages/ChatPage";
import CanvasPage from "../pages/CanvasPage";

export default function App() {
  useEffect(() => {
    // Signal to Playwright/legacy helpers that the React app finished booting.
    if (typeof window !== "undefined") {
      (window as typeof window & { __APP_READY__?: boolean }).__APP_READY__ = true;
    }
  }, []);

  const routes = useRoutes([
    { path: "/dashboard", element: <DashboardPage /> },
    { path: "/canvas", element: <CanvasPage /> },
    { path: "/chat/:agentId?/:threadId?", element: <ChatPage /> },
    { path: "*", element: <DashboardPage /> },
  ]);

  return <Layout>{routes}</Layout>;
}
