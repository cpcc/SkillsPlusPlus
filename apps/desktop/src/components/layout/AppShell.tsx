import { Outlet } from "react-router-dom";
import { SideNav } from "./SideNav";

export function AppShell() {
  return (
    <div className="flex h-screen overflow-hidden">
      <SideNav />
      <main className="flex-1 overflow-auto bg-gray-50 p-6">
        <Outlet />
      </main>
    </div>
  );
}
