import { NavLink } from "react-router-dom";
import { Compass, Package, Wrench, Settings } from "lucide-react";

const navItems = [
  { to: "/discover", icon: Compass, label: "发现" },
  { to: "/installed", icon: Package, label: "已安装" },
  { to: "/tools", icon: Wrench, label: "工具与目录" },
  { to: "/settings", icon: Settings, label: "设置" },
];

export function SideNav() {
  return (
    <nav className="flex h-full w-[200px] shrink-0 flex-col border-r border-[var(--color-border-subtle)] bg-[var(--color-surface-sidebar)]">
      {/* Brand */}
      <div className="flex h-14 items-center gap-2.5 px-5">
        <div className="flex h-7 w-7 items-center justify-center rounded-lg bg-[var(--color-accent-subtle)]">
          <span className="text-sm font-bold text-[var(--color-accent)]">S</span>
        </div>
        <span className="text-[15px] font-semibold tracking-tight text-[var(--color-text-primary)]">
          skills++
        </span>
      </div>

      {/* Nav items */}
      <ul className="flex flex-col gap-0.5 px-3 pt-2">
        {navItems.map(({ to, icon: Icon, label }) => (
          <li key={to}>
            <NavLink
              to={to}
              className={({ isActive }) =>
                `group flex items-center gap-2.5 rounded-[var(--radius-md)] px-3 py-[7px] text-[13px] font-medium transition-colors ${
                  isActive
                    ? "bg-[var(--color-accent-subtle)] text-[var(--color-accent-text)]"
                    : "text-[var(--color-text-secondary)] hover:bg-[var(--color-surface-hover)] hover:text-[var(--color-text-primary)]"
                }`
              }
            >
              <Icon className="h-[15px] w-[15px] opacity-70" strokeWidth={1.75} />
              {label}
            </NavLink>
          </li>
        ))}
      </ul>

      {/* Footer */}
      <div className="mt-auto px-5 pb-4">
        <p className="text-[11px] text-[var(--color-text-tertiary)]">
          v0.1.0
        </p>
      </div>
    </nav>
  );
}
