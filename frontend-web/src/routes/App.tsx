import React, { useEffect } from "react";
import { Route, Routes } from "react-router-dom";
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

  return (
    <Layout>
      <Routes>
        <Route path="/dashboard" element={<DashboardPage />} />
        <Route path="/canvas" element={<CanvasPage />} />
        <Route path="/chat/:agentId?/:threadId?" element={<ChatPage />} />
        <Route path="*" element={<DashboardPage />} />
      </Routes>
    </Layout>
  );
}
