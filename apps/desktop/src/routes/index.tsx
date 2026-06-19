import { Routes, Route, Navigate } from "react-router-dom";
import { AppShell } from "../components/layout/AppShell";
import DiscoverPage from "./discover/index";
import InstalledPage from "./installed/index";
import ToolsPage from "./tools/index";
import SettingsPage from "./settings/index";
import SkillDetailPage from "./skill/index";
import LocalSkillPage from "./local-skill/index";

export function AppRoutes() {
  return (
    <Routes>
      <Route element={<AppShell />}>
        <Route index element={<Navigate to="/discover" replace />} />
        <Route path="/discover" element={<DiscoverPage />} />
        <Route path="/skill/:id" element={<SkillDetailPage />} />
        <Route path="/local-skill" element={<LocalSkillPage />} />
        <Route path="/installed" element={<InstalledPage />} />
        <Route path="/tools" element={<ToolsPage />} />
        <Route path="/settings" element={<SettingsPage />} />
      </Route>
    </Routes>
  );
}
