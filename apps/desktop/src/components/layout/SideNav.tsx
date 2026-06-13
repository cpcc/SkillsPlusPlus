import { NavLink } from "react-router-dom";
import { Search, Package, Wrench, Settings } from "lucide-react";

const navItems = [
  { to: "/discover", icon: Search, label: "发现" },
  { to: "/installed", icon: Package, label: "已安装" },
  { to: "/tools", icon: Wrench, label: "工具与目录" },
  { to: "/settings", icon: Settings, label: "设置" },
];

export function SideNav() {
  return (
    <nav className="flex h-full w-48 flex-col border-r border-gray-200 bg-white px-2 py-4">
      <div className="mb-6 px-3 text-lg font-bold text-brand-600">skills++</div>
      <ul className="flex flex-col gap-1">
        {navItems.map(({ to, icon: Icon, label }) => (
          <li key={to}>
            <NavLink
              to={to}
              className={({ isActive }) =>
                `flex items-center gap-3 rounded-md px-3 py-2 text-sm font-medium transition-colors ${
                  isActive
                    ? "bg-brand-50 text-brand-700"
                    : "text-gray-600 hover:bg-gray-100 hover:text-gray-900"
                }`
              }
            >
              <Icon className="h-4 w-4" />
              {label}
            </NavLink>
          </li>
        ))}
      </ul>
    </nav>
  );
}
