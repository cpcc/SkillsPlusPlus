# skills++

## 项目概览

AI Skill 桌面端安装与管理工具（Tauri 桌面应用）。统一聚合多个 AI skill 来源站，自动识别本机 AI 工具目录，提供图形化的一键安装/卸载/重装能力。

**灵感来源**: Raycast、Linear、Warp 等现代开发者工具  
**当前阶段**: P1-P7 已完成（基础框架 → 来源聚合 → 安装链路 → 已安装管理 → HTTP bridge）

### 技术栈

| 层级 | 技术 |
|------|------|
| 桌面壳层 | Tauri 2.x |
| 前端框架 | React 19 + TypeScript 5.8 |
| 构建工具 | Vite 7 |
| 样式 | Tailwind CSS v4 + CSS 自定义属性 |
| 组件库 | Radix UI（Dialog, DropdownMenu, Toast, Tooltip 等） |
| 状态管理 | TanStack Query v5 (React Query) |
| 路由 | React Router v7 |
| 图标 | Lucide React |
| 字体 | Geist Sans / Geist Mono |
| 后端 | Rust 2021 Edition |
| HTTP | reqwest 0.12 (async) |
| 数据库 | SQLite via rusqlite 0.31 |
| HTTP 服务 | Axum 0.7（开发用 HTTP bridge） |
| 测试 | Vitest + Testing Library + Playwright( e2e) |
| Monorepo | pnpm workspace |

## 项目结构

```
aiskills/
├── apps/
│   └── desktop/                 # React + Tauri 前端
│       ├── src/
│       │   ├── main.tsx          # 应用入口（providers 包装）
│       │   ├── App.tsx           # 根组件
│       │   ├── index.css         # 设计令牌（@theme）+ 全局样式
│       │   ├── routes/
│       │   │   ├── index.tsx     # 路由定义
│       │   │   ├── discover/     # 发现页（搜索/筛选/卡片列表）
│       │   │   ├── skill/        # Skill 详情页（含安装入口）
│       │   │   ├── installed/    # 已安装管理页
│       │   │   ├── tools/        # 工具与目录页
│       │   │   └── settings/     # 设置页
│       │   ├── components/
│       │   │   ├── layout/       # AppShell, SideNav
│       │   │   ├── ui/           # Toast
│       │   │   ├── ErrorBoundary.tsx
│       │   │   └── install/      # InstallDialog, InstallLogPanel
│       │   ├── hooks/            # React Query hooks（目录、skill、安装、来源）
│       │   └── lib/
│       │       ├── ipc.ts        # Tauri invoke 类型封装
│       │       └── query-client.ts
│       └── src-tauri/            # Rust 后端
│           └── src/
│               ├── lib.rs        # Tauri Builder + 命令注册
│               ├── commands/     # IPC 命令层
│               │   ├── app.rs    # get_app_info
│               │   ├── directory.rs  # scan, list, add, toggle, delete
│               │   ├── source.rs     # list_sources, refresh, list_skills, get_skill
│               │   └── install.rs    # install, uninstall, reinstall, preview
│               ├── services/     # 业务逻辑层
│               │   ├── directory.rs  # 工具规则、路径展开、扫描
│               │   ├── source.rs     # SourceAdapter trait
│               │   ├── source_registry.rs  # 适配器注册
│               │   ├── adapters/   # GitHub Search / LobeHub / Stub
│               │   ├── install.rs  # 安装引擎（git clone、校验）
│               │   └── http_bridge.rs  # Axum HTTP bridge（dev 模式）
│               ├── repositories/  # 数据访问层（SQLite）
│               │   └── db.rs      # DB 初始化、migrations、seed_sources
│               └── models/        # Rust 数据模型（AppInfo, DirectoryRow, SkillItem 等）
├── packages/
│   └── shared/                   # 共享 TypeScript 类型
│       └── src/
│           ├── types.ts          # SkillItem, InstalledSkill, AiToolDirectory, InstallTask 等
│           └── index.ts
├── docs/
│   ├── PRD.md                    # 产品需求文档
│   ├── design-system.md          # 设计系统规范
│   ├── 开发计划.md               # 开发计划与里程碑
│   └── superpowers/plans/        # P1/P2/P3 实现计划
├── package.json                  # pnpm workspace root
└── pnpm-workspace.yaml
```

## 开发命令

```bash
# 开发模式（Tauri 桌面应用）
pnpm dev
pnpm tauri dev          # 等价快捷方式
pnpm --filter desktop tauri dev  # 指定 filter

# 生产构建
pnpm build

# 类型检查
pnpm --filter desktop exec tsc --noEmit

# 测试
pnpm test

# 单元测试（仅前端）
pnpm --filter desktop test:run

# E2E 测试
pnpm --filter desktop test:e2e

# Rust 编译检查
cd apps/desktop/src-tauri && cargo check

# Rust 构建
cd apps/desktop/src-tauri && cargo build
```

## 架构与模式

### 分层架构（Rust 后端）
```
commands → services → repositories → SQLite
      ↘          ↘           ↘
   IPC 接口    业务逻辑    数据访问
```

### 数据流
1. 前端通过 `@tauri-apps/api/core` 的 `invoke()` 调用 IPC 命令
2. IPC 命令由 Rust `commands/` 层处理，各命令接收 `State<DbState>` 访问 DB
3. `services/` 层承载业务逻辑（目录扫描、安装、来源抓取）
4. `repositories/db.rs` 处理 SQLite 读写 + migrations
5. `packages/shared/types.ts` 中定义前后端共享的 TS 类型

### IPC 封装模式
- `src/lib/ipc.ts` 统一封装 `invoke()` 调用，导出类型安全 API
- `hooks/` 目录使用 TanStack Query 包装 IPC 调用（query/mutation）
- 全部 IPC 命令注册在 `lib.rs` 的 `invoke_handler` 中

### 前端状态管理
- `TanStack Query`：管理服务端状态（目录列表、skill 缓存、安装任务）
- 组合式 hooks（`useDirectories`, `useSkills`, `useInstalledSkills` 等）
- Mutation 成功后自动 `invalidateQueries` 刷新相关列表
- Query keys 使用常量（`INSTALLED_KEY`, `SKILLS_KEY` 等）

### 样式系统
- Tailwind CSS v4 + `@theme` directive 定义设计令牌
- CSS 自定义属性（`--color-accent`, `--color-surface-*` 等）
- Radix UI 组件 + Tailwind 样式覆盖
- 详见 `docs/design-system.md`

## 设计系统

| Token | 值 | 用途 |
|-------|------|------|
| `--color-accent` | `#818cf8` | 主强调色（Indigo） |
| `--color-accent-muted` | `#6366f1` | 按钮背景 |
| `--color-surface-base` | `#0c0c10` | 主背景 |
| `--color-surface-raised` | `#16161c` | 卡片/输入框 |
| `--color-surface-overlay` | `#1c1c24` | 对话框 |
| `--color-text-primary` | `#e4e4e9` | 标题/正文 |
| `--color-text-secondary` | `#8e8e9a` | 次要信息 |

### 避坑清单
- ❌ 不使用渐变、玻璃态、纯白背景、无差别阴影
- ❌ 不添加滚动视差或大型动画
- ❌ 卡片不用等大三列，用网格变化内容密度

### 设计参数
| 维度 | 值 |
|------|-----|
| 变化度 | 6/10 |
| 动效强度 | 3/10 |
| 视觉密度 | 5/10 |

## 开发进度

| 阶段 | 状态 | 关键交付 |
|------|------|----------|
| P0 调研 | ✅ | 目录规则 + 来源适配方案 |
| P1 基础框架 | ✅ | Tauri+React+SQLite 骨架、导航、IPC |
| P2 工具目录 | ✅ | 目录扫描、权限检测、新增/启用/禁用/默认 |
| P3 来源聚合 | ✅ | GitHub Search / LobeHub 适配器、发现页、详情页 |
| P4 安装链路 | ✅ | git clone 安装、预览、冲突处理、日志 |
| P5 已安装管理 | ✅ | 列表、状态检测、重装/卸载、更新检查 |
| P6 测试与跨平台 | ✅ | 单元测试、组件测试、跨平台回归 |
| P7 HTTP Bridge | ✅ | 开发用 HTTP 服务（真实后端数据） |
| CI/CD | ✅ | 跨平台 Tauri release workflow |

## 核心数据模型

定义在 `packages/shared/src/types.ts`：

```typescript
SkillSource    // 来源站（id, name, baseUrl, enabled）
SkillItem      // Skill 条目（标准化后）
AiToolDirectory // AI 工具目录
InstalledSkill  // 已安装 skill
InstallTask     // 安装任务
ToolRule        // 工具目录规则
InstallPreview  // 安装预览
```

### 预置 AI 工具目录规则（Rust `services/directory.rs`）
Codex (`~/.codex/skills`, `~/.agents/skills`) · Claude (`~/.claude/skills`) · Cursor (`~/.cursor/rules`) · OpenCode (`~/.opencode/skills`) · GitHub Copilot · Antigravity · Gemini CLI · Kimi Code CLI · OpenClaw · CodeBuddy

### 预置来源站
skills.sh · LobeHub · SkillHub.cn · ClawHub.ai · SkillsMP
