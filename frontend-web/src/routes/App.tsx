import { Route, Routes } from "react-router-dom";
import DashboardPage from "../pages/DashboardPage";
import ChatPage from "../pages/ChatPage";

export default function App() {
  return (
    <Routes>
      <Route path="/dashboard" element={<DashboardPage />} />
      <Route path="/chat/:agentId?/:threadId?" element={<ChatPage />} />
      <Route path="*" element={<DashboardPage />} />
    </Routes>
  );
}
