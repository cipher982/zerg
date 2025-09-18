import { Route, Routes } from "react-router-dom";
import DashboardPage from "../pages/DashboardPage";

export default function App() {
  return (
    <Routes>
      <Route path="/dashboard" element={<DashboardPage />} />
      <Route path="*" element={<DashboardPage />} />
    </Routes>
  );
}
