import { Outlet } from "react-router-dom";
import { SideNav } from "./SideNav";

export function AppShell() {
  return (
    <div className="flex h-screen overflow-hidden bg-[var(--color-surface-base)]">
      <SideNav />
      <main className="flex-1 overflow-auto px-8 py-6">
        <Outlet />
      </main>
    </div>
  );
}
