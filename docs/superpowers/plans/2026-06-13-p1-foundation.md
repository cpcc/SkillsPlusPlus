# skills++ P1 基础框架与数据层 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 搭建 skills++ 桌面应用骨架，完成 Tauri + React monorepo 初始化、SQLite 数据层、IPC 通信层和四个主页面空态，为后续功能开发提供稳定底座。

**Architecture:** pnpm workspace monorepo，`apps/desktop` 承载 React 前端，`src-tauri/src` 分层为 commands / services / repositories / models，`packages/shared` 共享 TypeScript 类型。前端通过 TanStack Query + Tauri `invoke` 调用后端命令，SQLite 通过 rusqlite 提供本地持久化。

**Tech Stack:** Tauri 2.x · React 18 · TypeScript 5 · Vite 7 · Tailwind CSS 3 · Radix UI · TanStack Query v5 · React Router v6 · Rust 2021 Edition · rusqlite · pnpm workspaces

---

## 文件结构

```
aiskills/
├── apps/
│   └── desktop/
│       ├── src/
│       │   ├── main.tsx
│       │   ├── App.tsx
│       │   ├── routes/
│       │   │   ├── index.tsx          # router definition
│       │   │   ├── discover/
│       │   │   │   └── index.tsx
│       │   │   ├── installed/
│       │   │   │   └── index.tsx
│       │   │   ├── tools/
│       │   │   │   └── index.tsx
│       │   │   └── settings/
│       │   │       └── index.tsx
│       │   ├── components/
│       │   │   └── layout/
│       │   │       ├── AppShell.tsx
│       │   │       └── SideNav.tsx
│       │   ├── lib/
│       │   │   ├── ipc.ts             # typed invoke wrappers
│       │   │   └── query-client.ts
│       │   └── hooks/
│       │       └── use-app-info.ts
│       ├── index.html
│       ├── vite.config.ts
│       ├── tailwind.config.ts
│       ├── tsconfig.json
│       └── package.json
├── src-tauri/
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       ├── lib.rs
│       ├── commands/
│       │   ├── mod.rs
│       │   └── app.rs               # get_app_info, db_ping
│       ├── services/
│       │   └── mod.rs
│       ├── repositories/
│       │   ├── mod.rs
│       │   └── db.rs                # DB init + migrations
│       └── models/
│           └── mod.rs
├── packages/
│   └── shared/
│       ├── src/
│       │   ├── index.ts
│       │   └── types.ts             # SkillItem, InstalledSkill, etc.
│       ├── tsconfig.json
│       └── package.json
├── package.json                      # pnpm workspace root
├── pnpm-workspace.yaml
└── Cargo.toml                        # workspace root
```

---

## Task 1: 初始化 monorepo 骨架

**Files:**
- Create: `package.json`
- Create: `pnpm-workspace.yaml`
- Create: `.gitignore`
- Create: `apps/desktop/` (空目录占位)
- Create: `packages/shared/` (空目录占位)

- [ ] **Step 1: 创建 workspace 根文件**

```bash
cd /Users/ckj/dev/test/aiskills
```

创建 `package.json`:
```json
{
  "name": "skills-plus-plus",
  "version": "0.1.0",
  "private": true,
  "scripts": {
    "dev": "pnpm --filter desktop tauri dev",
    "build": "pnpm --filter desktop tauri build",
    "test": "pnpm --filter desktop test"
  },
  "devDependencies": {
    "typescript": "^5.4.0"
  }
}
```

创建 `pnpm-workspace.yaml`:
```yaml
packages:
  - "apps/*"
  - "packages/*"
```

- [ ] **Step 2: 创建 .gitignore**

```
node_modules/
dist/
.turbo/
target/
*.local
.env
apps/desktop/src-tauri/target/
```

- [ ] **Step 3: 初始化 git 仓库并首次提交**

```bash
cd /Users/ckj/dev/test/aiskills
git init
git add .
git commit -m "chore: initialize monorepo skeleton"
```

Expected: git repo created, initial commit made.

---

## Task 2: 创建 Tauri + React + TypeScript 前端

**Files:**
- Create: `apps/desktop/package.json`
- Create: `apps/desktop/index.html`
- Create: `apps/desktop/vite.config.ts`
- Create: `apps/desktop/tsconfig.json`
- Create: `apps/desktop/src/main.tsx`
- Create: `apps/desktop/src/App.tsx`
- Create: `src-tauri/Cargo.toml`
- Create: `src-tauri/src/main.rs`
- Create: `src-tauri/src/lib.rs`
- Create: `src-tauri/tauri.conf.json`

- [ ] **Step 1: 使用 create-tauri-app 初始化**

```bash
cd /Users/ckj/dev/test/aiskills
mkdir -p apps
cd apps
pnpm create tauri-app desktop -- --template react-ts --manager pnpm --tauri-version 2
```

Expected output: `apps/desktop/` 目录创建完成，包含 `src-tauri/`、`src/`、`package.json`。

- [ ] **Step 2: 验证应用可启动**

```bash
cd /Users/ckj/dev/test/aiskills/apps/desktop
pnpm install --registry https://registry.npmjs.org/
pnpm tauri dev
```

Expected: 应用窗口弹出，前端显示默认 Tauri 欢迎页。按 Ctrl+C 停止。

- [ ] **Step 3: 调整 tsconfig.json 启用严格模式**

修改 `apps/desktop/tsconfig.json`，确保包含:
```json
{
  "compilerOptions": {
    "target": "ES2022",
    "useDefineForClassFields": true,
    "lib": ["ES2022", "DOM", "DOM.Iterable"],
    "module": "ESNext",
    "skipLibCheck": true,
    "moduleResolution": "bundler",
    "allowImportingTsExtensions": true,
    "resolveJsonModule": true,
    "isolatedModules": true,
    "noEmit": true,
    "jsx": "react-jsx",
    "strict": true,
    "noUnusedLocals": true,
    "noUnusedParameters": true,
    "noFallthroughCasesInSwitch": true,
    "paths": {
      "@shared/*": ["../../packages/shared/src/*"]
    }
  },
  "include": ["src"],
  "references": [{ "path": "./tsconfig.node.json" }]
}
```

- [ ] **Step 4: 提交**

```bash
cd /Users/ckj/dev/test/aiskills
git add apps/ src-tauri/
git commit -m "feat: initialize Tauri + React + TypeScript app"
```

---

## Task 3: 安装并配置 Tailwind CSS + Radix UI

**Files:**
- Create: `apps/desktop/tailwind.config.ts`
- Create: `apps/desktop/postcss.config.js`
- Modify: `apps/desktop/src/main.tsx` (add global CSS import)
- Create: `apps/desktop/src/index.css`

- [ ] **Step 1: 安装 Tailwind CSS**

```bash
cd /Users/ckj/dev/test/aiskills/apps/desktop
pnpm add -D tailwindcss postcss autoprefixer --registry https://registry.npmjs.org/
pnpm add @radix-ui/react-dialog @radix-ui/react-dropdown-menu @radix-ui/react-tooltip @radix-ui/react-toast @radix-ui/react-separator @radix-ui/react-scroll-area --registry https://registry.npmjs.org/
pnpm add clsx tailwind-merge lucide-react --registry https://registry.npmjs.org/
```

- [ ] **Step 2: 创建 tailwind.config.ts**

```typescript
import type { Config } from "tailwindcss";

export default {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  theme: {
    extend: {
      colors: {
        brand: {
          50: "#f0f9ff",
          500: "#0ea5e9",
          600: "#0284c7",
          700: "#0369a1",
        },
      },
    },
  },
  plugins: [],
} satisfies Config;
```

- [ ] **Step 3: 创建 postcss.config.js**

```javascript
export default {
  plugins: {
    tailwindcss: {},
    autoprefixer: {},
  },
};
```

- [ ] **Step 4: 创建 src/index.css**

```css
@tailwind base;
@tailwind components;
@tailwind utilities;

:root {
  font-family: Inter, system-ui, -apple-system, sans-serif;
  -webkit-font-smoothing: antialiased;
}

body {
  @apply bg-gray-50 text-gray-900;
}
```

- [ ] **Step 5: 在 main.tsx 中引入 CSS**

修改 `apps/desktop/src/main.tsx`:
```tsx
import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App.tsx";
import "./index.css";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);
```

- [ ] **Step 6: 验证 Tailwind 工作**

修改 `apps/desktop/src/App.tsx` 为:
```tsx
export default function App() {
  return (
    <div className="flex h-screen items-center justify-center bg-gray-50">
      <h1 className="text-2xl font-bold text-brand-600">skills++</h1>
    </div>
  );
}
```

```bash
cd /Users/ckj/dev/test/aiskills/apps/desktop
pnpm tauri dev
```

Expected: 窗口显示蓝色 "skills++" 标题。按 Ctrl+C 停止。

- [ ] **Step 7: 提交**

```bash
cd /Users/ckj/dev/test/aiskills
git add apps/desktop/
git commit -m "feat: add Tailwind CSS and Radix UI"
```

---

## Task 4: 安装 TanStack Query v5 + React Router

**Files:**
- Create: `apps/desktop/src/lib/query-client.ts`
- Modify: `apps/desktop/src/main.tsx`

- [ ] **Step 1: 安装依赖**

```bash
cd /Users/ckj/dev/test/aiskills/apps/desktop
pnpm add @tanstack/react-query react-router-dom --registry https://registry.npmjs.org/
pnpm add -D @tanstack/react-query-devtools --registry https://registry.npmjs.org/
```

- [ ] **Step 2: 创建 src/lib/query-client.ts**

```typescript
import { QueryClient } from "@tanstack/react-query";

export const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 1000 * 60 * 5, // 5 minutes
      retry: 1,
    },
  },
});
```

- [ ] **Step 3: 更新 main.tsx 包装 providers**

```tsx
import React from "react";
import ReactDOM from "react-dom/client";
import { QueryClientProvider } from "@tanstack/react-query";
import { ReactQueryDevtools } from "@tanstack/react-query-devtools";
import { BrowserRouter } from "react-router-dom";
import App from "./App.tsx";
import { queryClient } from "./lib/query-client.ts";
import "./index.css";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <QueryClientProvider client={queryClient}>
      <BrowserRouter>
        <App />
      </BrowserRouter>
      <ReactQueryDevtools initialIsOpen={false} />
    </QueryClientProvider>
  </React.StrictMode>
);
```

- [ ] **Step 4: 提交**

```bash
cd /Users/ckj/dev/test/aiskills
git add apps/desktop/
git commit -m "feat: add TanStack Query v5 and React Router"
```

---

## Task 5: 创建 shared 包（共享 TypeScript 类型）

**Files:**
- Create: `packages/shared/package.json`
- Create: `packages/shared/tsconfig.json`
- Create: `packages/shared/src/index.ts`
- Create: `packages/shared/src/types.ts`

- [ ] **Step 1: 创建 packages/shared/package.json**

```json
{
  "name": "@skills-pp/shared",
  "version": "0.1.0",
  "private": true,
  "main": "./src/index.ts",
  "types": "./src/index.ts",
  "exports": {
    ".": "./src/index.ts"
  }
}
```

- [ ] **Step 2: 创建 packages/shared/tsconfig.json**

```json
{
  "compilerOptions": {
    "target": "ES2022",
    "module": "ESNext",
    "moduleResolution": "bundler",
    "strict": true,
    "noEmit": true,
    "isolatedModules": true
  },
  "include": ["src"]
}
```

- [ ] **Step 3: 创建 packages/shared/src/types.ts**

从 PRD 中的数据模型 + 开发计划中的补充模型：

```typescript
// ===== 来源站 =====
export type SkillSource = {
  id: string;
  name: string;
  baseUrl: string;
  enabled: boolean;
};

// ===== Skill 条目（来源聚合后标准化） =====
export type SkillItem = {
  id: string;
  name: string;
  author?: string;
  description?: string;
  tags: string[];
  sourceId: string;
  repoUrl?: string;
  detailUrl: string;
  updatedAt?: string;
  compatibleTools?: string[];
};

// ===== AI 工具目录 =====
export type AiToolDirectory = {
  id: string;
  toolName: string;
  path: string;
  isDefault: boolean;
  isDetected: boolean;
  writable: boolean;
  enabled: boolean;
  skillCount?: number;
};

// ===== 已安装 Skill =====
export type InstalledSkill = {
  id: string;
  skillId?: string;
  name: string;
  toolName: string;
  directoryId: string;
  sourceId?: string;
  repoUrl?: string;
  installedAt: string;
  status: "ok" | "missing" | "changed" | "update_available";
};

// ===== 安装任务 =====
export type InstallTask = {
  id: string;
  skillId?: string;
  skillName: string;
  toolName: string;
  directoryId: string;
  action: "install" | "reinstall" | "uninstall" | "scan";
  status: "pending" | "running" | "success" | "failed" | "cancelled";
  startedAt?: string;
  finishedAt?: string;
  errorMessage?: string;
};

// ===== 工具目录规则（Rust 侧镜像类型） =====
export type ToolRule = {
  toolName: string;
  platform: "macos" | "windows" | "linux" | "all";
  candidatePaths: string[];
  detectionHints?: string[];
  installStrategy: "copy" | "git" | "archive" | "skills_cli";
};

// ===== 应用信息（IPC 响应类型） =====
export type AppInfo = {
  version: string;
  dbPath: string;
  logPath: string;
  platform: string;
};

// ===== 错误码 =====
export const ErrorCode = {
  DIR_NOT_FOUND: "DIR_NOT_FOUND",
  DIR_NOT_WRITABLE: "DIR_NOT_WRITABLE",
  NETWORK_ERROR: "NETWORK_ERROR",
  SOURCE_FETCH_FAILED: "SOURCE_FETCH_FAILED",
  INSTALL_CONFLICT: "INSTALL_CONFLICT",
  INSTALL_FAILED: "INSTALL_FAILED",
  UNINSTALL_FAILED: "UNINSTALL_FAILED",
  DB_ERROR: "DB_ERROR",
} as const;

export type ErrorCode = (typeof ErrorCode)[keyof typeof ErrorCode];

export type AppError = {
  code: ErrorCode;
  message: string;
  detail?: string;
};
```

- [ ] **Step 4: 创建 packages/shared/src/index.ts**

```typescript
export * from "./types.ts";
```

- [ ] **Step 5: 在 apps/desktop 中引用 shared 包**

```bash
cd /Users/ckj/dev/test/aiskills/apps/desktop
pnpm add @skills-pp/shared@workspace:* --registry https://registry.npmjs.org/
```

- [ ] **Step 6: 验证类型可用**

在 `apps/desktop/src/App.tsx` 中加一行临时 import 验证不报错:
```tsx
import type { SkillItem } from "@skills-pp/shared";
// 验证后删掉该行
```

运行 `pnpm --filter desktop tsc --noEmit`，Expected: 无 TypeScript 错误。

- [ ] **Step 7: 提交**

```bash
cd /Users/ckj/dev/test/aiskills
git add packages/
git add apps/desktop/package.json apps/desktop/src/
git commit -m "feat: add shared TypeScript types package"
```

---

## Task 6: 创建应用布局与导航

**Files:**
- Create: `apps/desktop/src/components/layout/AppShell.tsx`
- Create: `apps/desktop/src/components/layout/SideNav.tsx`
- Create: `apps/desktop/src/routes/index.tsx`
- Create: `apps/desktop/src/routes/discover/index.tsx`
- Create: `apps/desktop/src/routes/installed/index.tsx`
- Create: `apps/desktop/src/routes/tools/index.tsx`
- Create: `apps/desktop/src/routes/settings/index.tsx`
- Modify: `apps/desktop/src/App.tsx`

- [ ] **Step 1: 创建 SideNav.tsx**

```tsx
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
```

- [ ] **Step 2: 创建 AppShell.tsx**

```tsx
import { Outlet } from "react-router-dom";
import { SideNav } from "./SideNav.tsx";

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
```

- [ ] **Step 3: 创建四个页面空态**

`apps/desktop/src/routes/discover/index.tsx`:
```tsx
export default function DiscoverPage() {
  return (
    <div>
      <h2 className="text-xl font-semibold text-gray-900">发现</h2>
      <p className="mt-2 text-gray-500">浏览来自多个来源站的 skill。</p>
    </div>
  );
}
```

`apps/desktop/src/routes/installed/index.tsx`:
```tsx
export default function InstalledPage() {
  return (
    <div>
      <h2 className="text-xl font-semibold text-gray-900">已安装</h2>
      <p className="mt-2 text-gray-500">查看和管理已安装到本机的 skill。</p>
    </div>
  );
}
```

`apps/desktop/src/routes/tools/index.tsx`:
```tsx
export default function ToolsPage() {
  return (
    <div>
      <h2 className="text-xl font-semibold text-gray-900">工具与目录</h2>
      <p className="mt-2 text-gray-500">管理 AI 工具及其 skill 安装目录。</p>
    </div>
  );
}
```

`apps/desktop/src/routes/settings/index.tsx`:
```tsx
export default function SettingsPage() {
  return (
    <div>
      <h2 className="text-xl font-semibold text-gray-900">设置</h2>
      <p className="mt-2 text-gray-500">来源站配置、缓存管理与日志。</p>
    </div>
  );
}
```

- [ ] **Step 4: 创建路由定义 routes/index.tsx**

```tsx
import { Routes, Route, Navigate } from "react-router-dom";
import { AppShell } from "../components/layout/AppShell.tsx";
import DiscoverPage from "./discover/index.tsx";
import InstalledPage from "./installed/index.tsx";
import ToolsPage from "./tools/index.tsx";
import SettingsPage from "./settings/index.tsx";

export function AppRoutes() {
  return (
    <Routes>
      <Route element={<AppShell />}>
        <Route index element={<Navigate to="/discover" replace />} />
        <Route path="/discover" element={<DiscoverPage />} />
        <Route path="/installed" element={<InstalledPage />} />
        <Route path="/tools" element={<ToolsPage />} />
        <Route path="/settings" element={<SettingsPage />} />
      </Route>
    </Routes>
  );
}
```

- [ ] **Step 5: 更新 App.tsx**

```tsx
import { AppRoutes } from "./routes/index.tsx";

export default function App() {
  return <AppRoutes />;
}
```

- [ ] **Step 6: 验证导航工作**

```bash
cd /Users/ckj/dev/test/aiskills/apps/desktop
pnpm tauri dev
```

Expected: 左侧导航栏可见，点击各导航项切换页面内容。按 Ctrl+C 停止。

- [ ] **Step 7: 提交**

```bash
cd /Users/ckj/dev/test/aiskills
git add apps/desktop/src/
git commit -m "feat: add app shell with sidebar navigation and page skeletons"
```

---

## Task 7: 配置 Rust / Tauri 后端分层结构

**Files:**
- Create: `src-tauri/src/commands/mod.rs`
- Create: `src-tauri/src/commands/app.rs`
- Create: `src-tauri/src/services/mod.rs`
- Create: `src-tauri/src/repositories/mod.rs`
- Create: `src-tauri/src/repositories/db.rs`
- Create: `src-tauri/src/models/mod.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/Cargo.toml`

- [ ] **Step 1: 添加 Rust 依赖到 Cargo.toml**

在 `src-tauri/Cargo.toml` 的 `[dependencies]` 中添加：
```toml
rusqlite = { version = "0.31", features = ["bundled"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
dirs = "5"
log = "0.4"
tauri-plugin-log = "2"
thiserror = "1"
```

- [ ] **Step 2: 创建 src/models/mod.rs**

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct AppInfo {
    pub version: String,
    pub db_path: String,
    pub log_path: String,
    pub platform: String,
}
```

- [ ] **Step 3: 创建 src/repositories/db.rs**

```rust
use rusqlite::{Connection, Result as SqliteResult};
use std::path::PathBuf;

pub fn open(db_path: &PathBuf) -> SqliteResult<Connection> {
    let conn = Connection::open(db_path)?;
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
    Ok(conn)
}

pub fn migrate(conn: &Connection) -> SqliteResult<()> {
    conn.execute_batch(SCHEMA_V1)?;
    Ok(())
}

const SCHEMA_V1: &str = "
CREATE TABLE IF NOT EXISTS skill_sources (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    base_url    TEXT NOT NULL,
    enabled     INTEGER NOT NULL DEFAULT 1,
    created_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS skill_cache (
    id              TEXT PRIMARY KEY,
    source_id       TEXT NOT NULL REFERENCES skill_sources(id),
    name            TEXT NOT NULL,
    author          TEXT,
    description     TEXT,
    tags            TEXT NOT NULL DEFAULT '[]',
    repo_url        TEXT,
    detail_url      TEXT NOT NULL,
    updated_at      TEXT,
    compatible_tools TEXT NOT NULL DEFAULT '[]',
    cached_at       TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS ai_tool_directories (
    id          TEXT PRIMARY KEY,
    tool_name   TEXT NOT NULL,
    path        TEXT NOT NULL,
    is_default  INTEGER NOT NULL DEFAULT 0,
    is_detected INTEGER NOT NULL DEFAULT 0,
    writable    INTEGER NOT NULL DEFAULT 0,
    enabled     INTEGER NOT NULL DEFAULT 1,
    skill_count INTEGER NOT NULL DEFAULT 0,
    created_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS installed_skills (
    id           TEXT PRIMARY KEY,
    skill_id     TEXT,
    name         TEXT NOT NULL,
    tool_name    TEXT NOT NULL,
    directory_id TEXT NOT NULL REFERENCES ai_tool_directories(id),
    source_id    TEXT,
    repo_url     TEXT,
    installed_at TEXT NOT NULL DEFAULT (datetime('now')),
    status       TEXT NOT NULL DEFAULT 'ok'
);

CREATE TABLE IF NOT EXISTS install_tasks (
    id            TEXT PRIMARY KEY,
    skill_id      TEXT,
    skill_name    TEXT NOT NULL,
    tool_name     TEXT NOT NULL,
    directory_id  TEXT NOT NULL,
    action        TEXT NOT NULL,
    status        TEXT NOT NULL DEFAULT 'pending',
    started_at    TEXT,
    finished_at   TEXT,
    error_message TEXT,
    created_at    TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS app_settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
";
```

- [ ] **Step 4: 创建 src/repositories/mod.rs**

```rust
pub mod db;
```

- [ ] **Step 5: 创建 src/services/mod.rs**

```rust
// Services 层占位 — P2+ 填充业务逻辑
```

- [ ] **Step 6: 创建 src/commands/app.rs**

```rust
use crate::models::AppInfo;
use tauri::State;
use std::sync::Mutex;
use rusqlite::Connection;

pub struct DbState(pub Mutex<Connection>);

#[tauri::command]
pub fn get_app_info(
    app: tauri::AppHandle,
    db: State<DbState>,
) -> Result<AppInfo, String> {
    let version = app.package_info().version.to_string();
    let db_path = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("skills_pp.db")
        .to_string_lossy()
        .to_string();

    // verify DB is accessible
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let _: i64 = conn
        .query_row("SELECT COUNT(*) FROM app_settings", [], |row| row.get(0))
        .unwrap_or(0);

    Ok(AppInfo {
        version,
        db_path,
        log_path: String::from("(see app data dir)"),
        platform: std::env::consts::OS.to_string(),
    })
}
```

- [ ] **Step 7: 创建 src/commands/mod.rs**

```rust
pub mod app;
pub use app::*;
```

- [ ] **Step 8: 更新 src/lib.rs**

```rust
pub mod commands;
pub mod models;
pub mod repositories;
pub mod services;

use commands::app::DbState;
use repositories::db;
use std::sync::Mutex;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let db_path = data_dir.join("skills_pp.db");

            let conn = db::open(&db_path)
                .expect("Failed to open database");
            db::migrate(&conn)
                .expect("Failed to run database migrations");

            app.manage(DbState(Mutex::new(conn)));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![commands::app::get_app_info])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 9: 构建验证**

```bash
cd /Users/ckj/dev/test/aiskills/apps/desktop
pnpm tauri build --no-bundle 2>&1 | tail -20
```

Expected: `Finished release [optimized]` — Rust 编译通过。

- [ ] **Step 10: 提交**

```bash
cd /Users/ckj/dev/test/aiskills
git add src-tauri/
git commit -m "feat: add Rust backend layers with SQLite init and get_app_info command"
```

---

## Task 8: 前端 IPC 封装 + AppInfo hook

**Files:**
- Create: `apps/desktop/src/lib/ipc.ts`
- Create: `apps/desktop/src/hooks/use-app-info.ts`
- Modify: `apps/desktop/src/routes/settings/index.tsx`

- [ ] **Step 1: 创建 src/lib/ipc.ts**

```typescript
import { invoke } from "@tauri-apps/api/core";
import type { AppInfo } from "@skills-pp/shared";

export const ipc = {
  getAppInfo: (): Promise<AppInfo> => invoke("get_app_info"),
};
```

- [ ] **Step 2: 创建 src/hooks/use-app-info.ts**

```typescript
import { useQuery } from "@tanstack/react-query";
import { ipc } from "../lib/ipc.ts";

export function useAppInfo() {
  return useQuery({
    queryKey: ["app-info"],
    queryFn: () => ipc.getAppInfo(),
    staleTime: Infinity,
  });
}
```

- [ ] **Step 3: 在设置页展示 AppInfo**

更新 `apps/desktop/src/routes/settings/index.tsx`:
```tsx
import { useAppInfo } from "../../hooks/use-app-info.ts";

export default function SettingsPage() {
  const { data, isLoading, error } = useAppInfo();

  return (
    <div>
      <h2 className="text-xl font-semibold text-gray-900">设置</h2>
      <p className="mt-2 text-gray-500">来源站配置、缓存管理与日志。</p>

      <div className="mt-6 rounded-lg border border-gray-200 bg-white p-4">
        <h3 className="text-sm font-medium text-gray-700">应用信息</h3>
        {isLoading && <p className="mt-2 text-sm text-gray-400">加载中...</p>}
        {error && (
          <p className="mt-2 text-sm text-red-500">
            加载失败：{String(error)}
          </p>
        )}
        {data && (
          <dl className="mt-2 space-y-1 text-sm text-gray-600">
            <div className="flex gap-2">
              <dt className="font-medium">版本：</dt>
              <dd>{data.version}</dd>
            </div>
            <div className="flex gap-2">
              <dt className="font-medium">平台：</dt>
              <dd>{data.platform}</dd>
            </div>
            <div className="flex gap-2">
              <dt className="font-medium">数据库：</dt>
              <dd className="truncate font-mono text-xs">{data.dbPath}</dd>
            </div>
          </dl>
        )}
      </div>
    </div>
  );
}
```

- [ ] **Step 4: 写单元测试验证 ipc 封装类型**

创建 `apps/desktop/src/lib/__tests__/ipc.test.ts`:
```typescript
import { describe, it, expect, vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockResolvedValue({
    version: "0.1.0",
    dbPath: "/tmp/test.db",
    logPath: "/tmp/test.log",
    platform: "macos",
  }),
}));

describe("ipc", () => {
  it("getAppInfo returns AppInfo shape", async () => {
    const { ipc } = await import("../ipc.ts");
    const result = await ipc.getAppInfo();
    expect(result).toHaveProperty("version");
    expect(result).toHaveProperty("platform");
    expect(result).toHaveProperty("dbPath");
  });
});
```

- [ ] **Step 5: 安装 Vitest 并运行测试**

```bash
cd /Users/ckj/dev/test/aiskills/apps/desktop
pnpm add -D vitest @vitest/ui jsdom @testing-library/react @testing-library/user-event msw --registry https://registry.npmjs.org/
```

在 `vite.config.ts` 中添加 vitest 配置:
```typescript
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  test: {
    globals: true,
    environment: "jsdom",
    setupFiles: ["./src/test-setup.ts"],
  },
});
```

创建 `apps/desktop/src/test-setup.ts`:
```typescript
import "@testing-library/jest-dom";
```

```bash
cd /Users/ckj/dev/test/aiskills/apps/desktop
pnpm add -D @testing-library/jest-dom --registry https://registry.npmjs.org/
pnpm test run
```

Expected: `1 test passed`.

- [ ] **Step 6: 运行完整开发验证**

```bash
cd /Users/ckj/dev/test/aiskills/apps/desktop
pnpm tauri dev
```

Expected: 
- 导航到"设置"页显示应用版本、平台、数据库路径。
- 其他3个页面均有标题和说明文本。
- 无控制台错误。

按 Ctrl+C 停止。

- [ ] **Step 7: 提交**

```bash
cd /Users/ckj/dev/test/aiskills
git add apps/desktop/src/
git commit -m "feat: add IPC layer, AppInfo hook and settings page integration"
```

---

## Task 9: 通知与错误边界基础设施

**Files:**
- Create: `apps/desktop/src/components/ui/toast.tsx`
- Create: `apps/desktop/src/components/ErrorBoundary.tsx`
- Modify: `apps/desktop/src/main.tsx`

- [ ] **Step 1: 创建简单 Toast 通知组件**

创建 `apps/desktop/src/components/ui/toast.tsx`:
```tsx
import * as Toast from "@radix-ui/react-toast";
import { createContext, useContext, useState, useCallback } from "react";

type ToastItem = { id: string; title: string; description?: string; variant?: "default" | "error" };
type ToastFn = (title: string, description?: string, variant?: ToastItem["variant"]) => void;

const ToastContext = createContext<ToastFn>(() => {});

export function useToast() {
  return useContext(ToastContext);
}

export function ToastProvider({ children }: { children: React.ReactNode }) {
  const [toasts, setToasts] = useState<ToastItem[]>([]);

  const addToast = useCallback<ToastFn>((title, description, variant = "default") => {
    const id = crypto.randomUUID();
    setToasts((prev) => [...prev, { id, title, description, variant }]);
  }, []);

  return (
    <ToastContext.Provider value={addToast}>
      <Toast.Provider swipeDirection="right">
        {children}
        {toasts.map((t) => (
          <Toast.Root
            key={t.id}
            className={`rounded-lg border p-4 shadow-lg ${
              t.variant === "error"
                ? "border-red-200 bg-red-50 text-red-900"
                : "border-gray-200 bg-white text-gray-900"
            }`}
            onOpenChange={(open) => {
              if (!open) setToasts((prev) => prev.filter((x) => x.id !== t.id));
            }}
          >
            <Toast.Title className="text-sm font-semibold">{t.title}</Toast.Title>
            {t.description && (
              <Toast.Description className="mt-1 text-xs text-gray-500">
                {t.description}
              </Toast.Description>
            )}
          </Toast.Root>
        ))}
        <Toast.Viewport className="fixed bottom-4 right-4 flex flex-col gap-2 w-80 z-50" />
      </Toast.Provider>
    </ToastContext.Provider>
  );
}
```

- [ ] **Step 2: 创建 ErrorBoundary**

创建 `apps/desktop/src/components/ErrorBoundary.tsx`:
```tsx
import { Component, type ReactNode } from "react";

interface Props {
  children: ReactNode;
  fallback?: ReactNode;
}

interface State {
  hasError: boolean;
  error?: Error;
}

export class ErrorBoundary extends Component<Props, State> {
  state: State = { hasError: false };

  static getDerivedStateFromError(error: Error): State {
    return { hasError: true, error };
  }

  render() {
    if (this.state.hasError) {
      return (
        this.props.fallback ?? (
          <div className="flex flex-col items-center justify-center h-full gap-4 p-8 text-center">
            <p className="text-lg font-semibold text-red-700">出错了</p>
            <p className="text-sm text-gray-500">{this.state.error?.message}</p>
          </div>
        )
      );
    }
    return this.props.children;
  }
}
```

- [ ] **Step 3: 更新 main.tsx 加入 ToastProvider 和 ErrorBoundary**

```tsx
import React from "react";
import ReactDOM from "react-dom/client";
import { QueryClientProvider } from "@tanstack/react-query";
import { ReactQueryDevtools } from "@tanstack/react-query-devtools";
import { BrowserRouter } from "react-router-dom";
import App from "./App.tsx";
import { queryClient } from "./lib/query-client.ts";
import { ToastProvider } from "./components/ui/toast.tsx";
import { ErrorBoundary } from "./components/ErrorBoundary.tsx";
import "./index.css";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <ErrorBoundary>
      <QueryClientProvider client={queryClient}>
        <BrowserRouter>
          <ToastProvider>
            <App />
          </ToastProvider>
        </BrowserRouter>
        <ReactQueryDevtools initialIsOpen={false} />
      </QueryClientProvider>
    </ErrorBoundary>
  </React.StrictMode>
);
```

- [ ] **Step 4: 验证应用正常运行**

```bash
cd /Users/ckj/dev/test/aiskills/apps/desktop
pnpm tauri dev
```

Expected: 应用正常启动，导航、设置页数据展示均正常，无控制台错误。

- [ ] **Step 5: 提交**

```bash
cd /Users/ckj/dev/test/aiskills
git add apps/desktop/src/
git commit -m "feat: add Toast notification and ErrorBoundary infrastructure"
```

---

## Task 10: P1 交付验证

**验收标准（来自 roadmap P1 Checkpoints）：**

- [ ] **Checkpoint 1: 本地数据库初始化成功**

```bash
cd /Users/ckj/dev/test/aiskills/apps/desktop
pnpm tauri dev &
sleep 5
# 在设置页检查 dbPath 路径存在
ls "$(cat ~/.local/share/skills-pp/skills_pp.db 2>/dev/null || echo 'check settings page')"
kill %1
```

Expected: 设置页中 `dbPath` 显示有效路径。

- [ ] **Checkpoint 2: 前后端基础命令调用成功**

启动 `pnpm tauri dev`，导航到设置页，确认：
- 版本号显示（如 `0.1.0`）
- 平台字段显示（如 `macos`）
- dbPath 字段显示完整路径

Expected: 所有字段正常显示，无 IPC 错误。

- [ ] **Checkpoint 3: 四个主页面可以稳定访问**

点击侧边栏每个导航项，确认：
- 发现页：显示 "发现" 标题
- 已安装页：显示 "已安装" 标题
- 工具与目录页：显示 "工具与目录" 标题
- 设置页：显示 AppInfo 数据

Expected: 四页均可访问，无白屏，无控制台错误。

- [ ] **Checkpoint 4: 测试通过**

```bash
cd /Users/ckj/dev/test/aiskills/apps/desktop
pnpm test run
```

Expected: `All tests passed`.

- [ ] **最终提交**

```bash
cd /Users/ckj/dev/test/aiskills
git add .
git commit -m "feat: complete P1 foundation - Tauri+React app with SQLite, IPC and page skeletons"
git tag p1-complete
```

---

## 自审（Spec Coverage）

| 开发计划 P1 任务 | 对应 Task |
|---|---|
| 初始化 Tauri + React + TypeScript + Tailwind + Radix UI + TanStack Query | Task 2, 3, 4 |
| 建立路由、布局、导航、通知、对话框 | Task 6, 9 |
| 建立 SQLite 表结构、迁移、Repository 层 | Task 7 |
| 建立共享类型、前后端通信 schema 和错误码 | Task 5 |
| 建立日志系统（基础） | Task 7（tauri-plugin-log） |
| 输出前端页面骨架：发现、已安装、工具与目录、设置 | Task 6 |
| 可启动并完成本地数据库初始化 | Task 7, 10 |
| 前后端 IPC 可用，基础查询/写入 | Task 8, 10 |
| 核心页面空态可访问 | Task 6, 10 |
