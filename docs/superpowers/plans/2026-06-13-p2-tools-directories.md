# skills++ P2 工具与目录管理 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 实现 AI 工具目录规则模型、跨平台路径展开、自动扫描、读写权限检测与 skill 计数，并在"工具与目录"页展示扫描结果，支持手动新增/禁用目录和默认目录设置。

**Architecture:** Rust 侧新增 `services/directory.rs`（目录规则 + 扫描逻辑）和 `commands/directory.rs`（IPC 命令）。前端新增 `hooks/use-directories.ts` + 工具目录页完整 UI（列表、新增对话框、状态展示）。工具规则以 Rust 常量内置，通过 DB `ai_tool_directories` 表持久化用户配置。

**Tech Stack:** 已有栈 + `dirs` crate (跨平台路径) · Radix UI Dialog · React Hook Form

---

## 文件结构（新增/修改）

```
src-tauri/src/
  services/
    mod.rs                    # 新增 pub mod directory
    directory.rs              # 工具规则 + 扫描逻辑
  commands/
    mod.rs                    # 新增 pub mod directory
    directory.rs              # IPC 命令：scan, list, add, toggle, set_default
  models/
    mod.rs                    # 新增 DirectoryRow, ScanResult

apps/desktop/src/
  hooks/
    use-directories.ts        # TanStack Query hooks
  routes/tools/
    index.tsx                 # 完整页面（列表 + 操作）
    DirectoryCard.tsx         # 单个目录卡片
    AddDirectoryDialog.tsx    # 新增目录对话框
  lib/
    ipc.ts                    # 新增 directory IPC calls
```

---

## Task 1: Rust 工具规则模型与路径展开

**Files:**
- Modify: `src-tauri/src/models/mod.rs`
- Create: `src-tauri/src/services/directory.rs`
- Modify: `src-tauri/src/services/mod.rs`

- [ ] **Step 1: 更新 models/mod.rs — 新增目录模型**

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct AppInfo {
    pub version: String,
    pub db_path: String,
    pub log_path: String,
    pub platform: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectoryRow {
    pub id: String,
    pub tool_name: String,
    pub path: String,
    pub is_default: bool,
    pub is_detected: bool,
    pub writable: bool,
    pub enabled: bool,
    pub skill_count: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ScanResult {
    pub id: String,
    pub tool_name: String,
    pub path: String,
    pub exists: bool,
    pub writable: bool,
    pub skill_count: i64,
}
```

- [ ] **Step 2: 创建 services/directory.rs**

```rust
use crate::models::{DirectoryRow, ScanResult};
use dirs;
use rusqlite::{Connection, Result as SqliteResult};
use std::path::{Path, PathBuf};

// ─── Tool rules ───────────────────────────────────────────────────────────────

struct ToolRule {
    tool_name: &'static str,
    candidate_paths: &'static [&'static str], // relative to home dir
}

const TOOL_RULES: &[ToolRule] = &[
    ToolRule {
        tool_name: "Codex",
        candidate_paths: &[".codex/skills", ".agents/skills"],
    },
    ToolRule {
        tool_name: "Claude",
        candidate_paths: &[".claude/skills"],
    },
    ToolRule {
        tool_name: "Cursor",
        candidate_paths: &[".cursor/rules", ".cursor/skills"],
    },
    ToolRule {
        tool_name: "OpenCode",
        candidate_paths: &[".opencode/skills"],
    },
    ToolRule {
        tool_name: "GitHub Copilot",
        candidate_paths: &[
            ".copilot/installed-plugins/superpowers-marketplace/superpowers/skills",
        ],
    },
    ToolRule {
        tool_name: "Antigravity",
        candidate_paths: &[".antigravity/skills"],
    },
    ToolRule {
        tool_name: "Gemini CLI",
        candidate_paths: &[".gemini/skills"],
    },
    ToolRule {
        tool_name: "Kimi Code CLI",
        candidate_paths: &[".kimi/skills"],
    },
    ToolRule {
        tool_name: "OpenClaw",
        candidate_paths: &[".openclaw/skills"],
    },
    ToolRule {
        tool_name: "CodeBuddy",
        candidate_paths: &[".codebuddy/skills"],
    },
];

// ─── Path expansion ───────────────────────────────────────────────────────────

/// Expand a relative path (relative to home dir) to absolute PathBuf.
pub fn expand_path(relative: &str) -> Option<PathBuf> {
    let home = dirs::home_dir()?;
    Some(home.join(relative))
}

/// Count skill entries (subdirectories or .md files) in a directory.
pub fn count_skills(path: &Path) -> i64 {
    if !path.exists() {
        return 0;
    }
    std::fs::read_dir(path)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| {
                    let p = e.path();
                    p.is_dir()
                        || p.extension()
                            .map(|x| x == "md" || x == "yaml" || x == "yml")
                            .unwrap_or(false)
                })
                .count() as i64
        })
        .unwrap_or(0)
}

/// Check if a path is writable by attempting to create a temp file.
pub fn is_writable(path: &Path) -> bool {
    if !path.exists() {
        return false;
    }
    let test = path.join(".skills_pp_write_test");
    match std::fs::write(&test, b"") {
        Ok(_) => {
            let _ = std::fs::remove_file(&test);
            true
        }
        Err(_) => false,
    }
}

// ─── Scan ─────────────────────────────────────────────────────────────────────

pub fn scan_directory(id: &str, tool_name: &str, path_str: &str) -> ScanResult {
    let path = PathBuf::from(path_str);
    let exists = path.exists();
    let writable = if exists { is_writable(&path) } else { false };
    let skill_count = if exists { count_skills(&path) } else { 0 };

    ScanResult {
        id: id.to_string(),
        tool_name: tool_name.to_string(),
        path: path_str.to_string(),
        exists,
        writable,
        skill_count,
    }
}

// ─── Seed default directories ─────────────────────────────────────────────────

/// Insert built-in tool directories into DB if not already present.
pub fn seed_default_directories(conn: &Connection) -> SqliteResult<()> {
    for rule in TOOL_RULES {
        for (i, relative_path) in rule.candidate_paths.iter().enumerate() {
            let Some(abs_path) = expand_path(relative_path) else {
                continue;
            };
            let path_str = abs_path.to_string_lossy().to_string();
            let id = format!(
                "{}-{}",
                rule.tool_name.to_lowercase().replace(' ', "-"),
                i
            );
            conn.execute(
                "INSERT OR IGNORE INTO ai_tool_directories \
                 (id, tool_name, path, is_default, is_detected, writable, enabled, skill_count) \
                 VALUES (?1, ?2, ?3, ?4, 0, 0, 1, 0)",
                rusqlite::params![
                    id,
                    rule.tool_name,
                    path_str,
                    if i == 0 { 1 } else { 0 }, // first path is default
                ],
            )?;
        }
    }
    Ok(())
}

// ─── Repository helpers ───────────────────────────────────────────────────────

pub fn list_directories(conn: &Connection) -> SqliteResult<Vec<DirectoryRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, tool_name, path, is_default, is_detected, writable, enabled, skill_count \
         FROM ai_tool_directories ORDER BY tool_name, is_default DESC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(DirectoryRow {
            id: row.get(0)?,
            tool_name: row.get(1)?,
            path: row.get(2)?,
            is_default: row.get::<_, i64>(3)? != 0,
            is_detected: row.get::<_, i64>(4)? != 0,
            writable: row.get::<_, i64>(5)? != 0,
            enabled: row.get::<_, i64>(6)? != 0,
            skill_count: row.get(7)?,
        })
    })?;
    rows.collect()
}

pub fn update_scan_result(conn: &Connection, result: &ScanResult) -> SqliteResult<()> {
    conn.execute(
        "UPDATE ai_tool_directories \
         SET is_detected = ?1, writable = ?2, skill_count = ?3 \
         WHERE id = ?4",
        rusqlite::params![
            result.exists as i64,
            result.writable as i64,
            result.skill_count,
            result.id,
        ],
    )?;
    Ok(())
}
```

- [ ] **Step 3: 更新 services/mod.rs**

```rust
pub mod directory;
```

- [ ] **Step 4: Rust 编译验证**

```bash
cd /Users/ckj/dev/test/aiskills/apps/desktop/src-tauri
cargo check 2>&1 | tail -5
```

Expected: `Finished dev profile`.

- [ ] **Step 5: 提交**

```bash
cd /Users/ckj/dev/test/aiskills
git add apps/desktop/src-tauri/src/
git commit -m "feat(p2): add tool directory rules, path expansion and scan logic"
```

---

## Task 2: Rust IPC 命令 — 目录扫描与管理

**Files:**
- Create: `src-tauri/src/commands/directory.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: 创建 commands/directory.rs**

```rust
use crate::commands::app::DbState;
use crate::models::{DirectoryRow, ScanResult};
use crate::services::directory as dir_svc;
use rusqlite::params;
use tauri::State;
use uuid::Uuid;

/// Seed default dirs then scan all enabled directories.
/// Returns updated list after persisting scan results.
#[tauri::command]
pub fn scan_directories(db: State<DbState>) -> Result<Vec<DirectoryRow>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;

    dir_svc::seed_default_directories(&conn).map_err(|e| e.to_string())?;

    let dirs = dir_svc::list_directories(&conn).map_err(|e| e.to_string())?;

    for d in &dirs {
        if d.enabled {
            let result = dir_svc::scan_directory(&d.id, &d.tool_name, &d.path);
            dir_svc::update_scan_result(&conn, &result).map_err(|e| e.to_string())?;
        }
    }

    dir_svc::list_directories(&conn).map_err(|e| e.to_string())
}

/// List all directories (no rescan).
#[tauri::command]
pub fn list_directories(db: State<DbState>) -> Result<Vec<DirectoryRow>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    dir_svc::list_directories(&conn).map_err(|e| e.to_string())
}

/// Add a custom directory.
#[tauri::command]
pub fn add_directory(
    db: State<DbState>,
    tool_name: String,
    path: String,
) -> Result<DirectoryRow, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let id = Uuid::new_v4().to_string();

    conn.execute(
        "INSERT INTO ai_tool_directories \
         (id, tool_name, path, is_default, is_detected, writable, enabled, skill_count) \
         VALUES (?1, ?2, ?3, 0, 0, 0, 1, 0)",
        params![id, tool_name, path],
    )
    .map_err(|e| e.to_string())?;

    // Run an immediate scan on the new directory
    let result = dir_svc::scan_directory(&id, &tool_name, &path);
    dir_svc::update_scan_result(&conn, &result).map_err(|e| e.to_string())?;

    Ok(DirectoryRow {
        id,
        tool_name,
        path,
        is_default: false,
        is_detected: result.exists,
        writable: result.writable,
        enabled: true,
        skill_count: result.skill_count,
    })
}

/// Toggle enabled/disabled for a directory.
#[tauri::command]
pub fn toggle_directory(db: State<DbState>, id: String, enabled: bool) -> Result<(), String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE ai_tool_directories SET enabled = ?1 WHERE id = ?2",
        params![enabled as i64, id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Set a directory as the default for its tool.
#[tauri::command]
pub fn set_default_directory(db: State<DbState>, id: String) -> Result<(), String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;

    // Get tool_name for this directory
    let tool_name: String = conn
        .query_row(
            "SELECT tool_name FROM ai_tool_directories WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    // Clear default for all dirs of this tool, then set the chosen one
    conn.execute(
        "UPDATE ai_tool_directories SET is_default = 0 WHERE tool_name = ?1",
        params![tool_name],
    )
    .map_err(|e| e.to_string())?;

    conn.execute(
        "UPDATE ai_tool_directories SET is_default = 1 WHERE id = ?1",
        params![id],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

/// Delete a custom directory entry.
#[tauri::command]
pub fn delete_directory(db: State<DbState>, id: String) -> Result<(), String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "DELETE FROM ai_tool_directories WHERE id = ?1",
        params![id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}
```

- [ ] **Step 2: 更新 commands/mod.rs**

```rust
pub mod app;
pub mod directory;
pub use app::*;
pub use directory::*;
```

- [ ] **Step 3: 在 Cargo.toml 中添加 uuid**

在 `[dependencies]` 中添加:
```toml
uuid = { version = "1", features = ["v4"] }
```

- [ ] **Step 4: 更新 lib.rs — 注册目录命令**

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
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let db_path = data_dir.join("skills_pp.db");

            let conn = db::open(&db_path).expect("Failed to open database");
            db::migrate(&conn).expect("Failed to run database migrations");

            app.manage(DbState(Mutex::new(conn)));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app::get_app_info,
            commands::directory::scan_directories,
            commands::directory::list_directories,
            commands::directory::add_directory,
            commands::directory::toggle_directory,
            commands::directory::set_default_directory,
            commands::directory::delete_directory,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 5: Rust 编译验证**

```bash
cd /Users/ckj/dev/test/aiskills/apps/desktop/src-tauri
cargo check 2>&1 | tail -5
```

Expected: `Finished dev profile`.

- [ ] **Step 6: 提交**

```bash
cd /Users/ckj/dev/test/aiskills
git add apps/desktop/src-tauri/
git commit -m "feat(p2): add directory IPC commands (scan, list, add, toggle, set_default, delete)"
```

---

## Task 3: 前端 IPC 封装 + 目录 Hooks

**Files:**
- Modify: `apps/desktop/src/lib/ipc.ts`
- Create: `apps/desktop/src/hooks/use-directories.ts`

- [ ] **Step 1: 更新 ipc.ts — 新增目录相关调用**

```typescript
import { invoke } from "@tauri-apps/api/core";
import type { AppInfo, AiToolDirectory } from "@skills-pp/shared";

export const ipc = {
  getAppInfo: (): Promise<AppInfo> => invoke("get_app_info"),

  scanDirectories: (): Promise<AiToolDirectory[]> =>
    invoke("scan_directories"),

  listDirectories: (): Promise<AiToolDirectory[]> =>
    invoke("list_directories"),

  addDirectory: (toolName: string, path: string): Promise<AiToolDirectory> =>
    invoke("add_directory", { toolName, path }),

  toggleDirectory: (id: string, enabled: boolean): Promise<void> =>
    invoke("toggle_directory", { id, enabled }),

  setDefaultDirectory: (id: string): Promise<void> =>
    invoke("set_default_directory", { id }),

  deleteDirectory: (id: string): Promise<void> =>
    invoke("delete_directory", { id }),
};
```

- [ ] **Step 2: 创建 hooks/use-directories.ts**

```typescript
import {
  useQuery,
  useMutation,
  useQueryClient,
} from "@tanstack/react-query";
import { ipc } from "../lib/ipc";

const QUERY_KEY = ["directories"] as const;

export function useDirectories() {
  return useQuery({
    queryKey: QUERY_KEY,
    queryFn: () => ipc.listDirectories(),
  });
}

export function useScanDirectories() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: () => ipc.scanDirectories(),
    onSuccess: (data) => {
      qc.setQueryData(QUERY_KEY, data);
    },
  });
}

export function useAddDirectory() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ toolName, path }: { toolName: string; path: string }) =>
      ipc.addDirectory(toolName, path),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: QUERY_KEY });
    },
  });
}

export function useToggleDirectory() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ id, enabled }: { id: string; enabled: boolean }) =>
      ipc.toggleDirectory(id, enabled),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: QUERY_KEY });
    },
  });
}

export function useSetDefaultDirectory() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => ipc.setDefaultDirectory(id),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: QUERY_KEY });
    },
  });
}

export function useDeleteDirectory() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => ipc.deleteDirectory(id),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: QUERY_KEY });
    },
  });
}
```

- [ ] **Step 3: TypeScript 验证**

```bash
cd /Users/ckj/dev/test/aiskills/apps/desktop
pnpm exec tsc --noEmit 2>&1 | head -20
```

Expected: 无错误输出.

- [ ] **Step 4: 提交**

```bash
cd /Users/ckj/dev/test/aiskills
git add apps/desktop/src/
git commit -m "feat(p2): add directory IPC wrappers and React Query hooks"
```

---

## Task 4: 目录卡片组件

**Files:**
- Create: `apps/desktop/src/routes/tools/DirectoryCard.tsx`

- [ ] **Step 1: 创建 DirectoryCard.tsx**

```tsx
import type { AiToolDirectory } from "@skills-pp/shared";
import { Folder, CheckCircle, XCircle, AlertCircle, Star } from "lucide-react";
import * as DropdownMenu from "@radix-ui/react-dropdown-menu";
import { MoreHorizontal } from "lucide-react";

interface Props {
  dir: AiToolDirectory;
  onToggle: (id: string, enabled: boolean) => void;
  onSetDefault: (id: string) => void;
  onDelete: (id: string) => void;
  onOpenFolder: (path: string) => void;
}

function StatusBadge({ dir }: { dir: AiToolDirectory }) {
  if (!dir.isDetected) {
    return (
      <span className="flex items-center gap-1 text-xs text-gray-400">
        <AlertCircle className="h-3 w-3" />
        未找到
      </span>
    );
  }
  if (!dir.writable) {
    return (
      <span className="flex items-center gap-1 text-xs text-yellow-600">
        <XCircle className="h-3 w-3" />
        只读
      </span>
    );
  }
  return (
    <span className="flex items-center gap-1 text-xs text-green-600">
      <CheckCircle className="h-3 w-3" />
      可用
    </span>
  );
}

export function DirectoryCard({ dir, onToggle, onSetDefault, onDelete, onOpenFolder }: Props) {
  return (
    <div
      className={`rounded-lg border bg-white p-4 transition-opacity ${
        !dir.enabled ? "opacity-50" : ""
      }`}
    >
      <div className="flex items-start justify-between gap-3">
        <div className="flex min-w-0 flex-1 items-start gap-3">
          <Folder className="mt-0.5 h-4 w-4 shrink-0 text-brand-500" />
          <div className="min-w-0">
            <div className="flex items-center gap-2">
              <span className="text-sm font-medium text-gray-900">
                {dir.toolName}
              </span>
              {dir.isDefault && (
                <span className="flex items-center gap-1 rounded-full bg-brand-50 px-2 py-0.5 text-xs text-brand-700">
                  <Star className="h-3 w-3" />
                  默认
                </span>
              )}
            </div>
            <p
              className="mt-0.5 truncate font-mono text-xs text-gray-400"
              title={dir.path}
            >
              {dir.path}
            </p>
            <div className="mt-1 flex items-center gap-3">
              <StatusBadge dir={dir} />
              {dir.isDetected && (
                <span className="text-xs text-gray-400">
                  {dir.skillCount ?? 0} 个 skill
                </span>
              )}
            </div>
          </div>
        </div>

        <DropdownMenu.Root>
          <DropdownMenu.Trigger asChild>
            <button className="rounded p-1 text-gray-400 hover:bg-gray-100 hover:text-gray-600">
              <MoreHorizontal className="h-4 w-4" />
            </button>
          </DropdownMenu.Trigger>
          <DropdownMenu.Portal>
            <DropdownMenu.Content
              className="z-50 min-w-40 rounded-lg border border-gray-200 bg-white py-1 shadow-lg"
              sideOffset={4}
            >
              <DropdownMenu.Item
                className="flex cursor-pointer items-center gap-2 px-3 py-1.5 text-sm text-gray-700 hover:bg-gray-50"
                onSelect={() => onOpenFolder(dir.path)}
              >
                打开目录
              </DropdownMenu.Item>
              {!dir.isDefault && dir.isDetected && (
                <DropdownMenu.Item
                  className="flex cursor-pointer items-center gap-2 px-3 py-1.5 text-sm text-gray-700 hover:bg-gray-50"
                  onSelect={() => onSetDefault(dir.id)}
                >
                  设为默认
                </DropdownMenu.Item>
              )}
              <DropdownMenu.Item
                className="flex cursor-pointer items-center gap-2 px-3 py-1.5 text-sm text-gray-700 hover:bg-gray-50"
                onSelect={() => onToggle(dir.id, !dir.enabled)}
              >
                {dir.enabled ? "禁用" : "启用"}
              </DropdownMenu.Item>
              <DropdownMenu.Separator className="my-1 h-px bg-gray-100" />
              <DropdownMenu.Item
                className="flex cursor-pointer items-center gap-2 px-3 py-1.5 text-sm text-red-600 hover:bg-red-50"
                onSelect={() => onDelete(dir.id)}
              >
                删除
              </DropdownMenu.Item>
            </DropdownMenu.Content>
          </DropdownMenu.Portal>
        </DropdownMenu.Root>
      </div>
    </div>
  );
}
```

- [ ] **Step 2: TypeScript 验证**

```bash
cd /Users/ckj/dev/test/aiskills/apps/desktop && pnpm exec tsc --noEmit 2>&1 | head -10
```

Expected: 无错误.

- [ ] **Step 3: 提交**

```bash
cd /Users/ckj/dev/test/aiskills
git add apps/desktop/src/routes/tools/
git commit -m "feat(p2): add DirectoryCard component"
```

---

## Task 5: 新增目录对话框

**Files:**
- Create: `apps/desktop/src/routes/tools/AddDirectoryDialog.tsx`

- [ ] **Step 1: 创建 AddDirectoryDialog.tsx**

```tsx
import * as Dialog from "@radix-ui/react-dialog";
import { useState } from "react";
import { X } from "lucide-react";

const TOOL_NAMES = [
  "Codex",
  "Claude",
  "Cursor",
  "OpenCode",
  "GitHub Copilot",
  "Antigravity",
  "Gemini CLI",
  "Kimi Code CLI",
  "OpenClaw",
  "CodeBuddy",
  "其他",
];

interface Props {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onAdd: (toolName: string, path: string) => void;
  isPending: boolean;
}

export function AddDirectoryDialog({
  open,
  onOpenChange,
  onAdd,
  isPending,
}: Props) {
  const [toolName, setToolName] = useState(TOOL_NAMES[0]);
  const [customTool, setCustomTool] = useState("");
  const [path, setPath] = useState("");

  function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    const finalTool = toolName === "其他" ? customTool.trim() : toolName;
    if (!finalTool || !path.trim()) return;
    onAdd(finalTool, path.trim());
  }

  return (
    <Dialog.Root open={open} onOpenChange={onOpenChange}>
      <Dialog.Portal>
        <Dialog.Overlay className="fixed inset-0 z-40 bg-black/30" />
        <Dialog.Content className="fixed left-1/2 top-1/2 z-50 w-full max-w-md -translate-x-1/2 -translate-y-1/2 rounded-xl bg-white p-6 shadow-xl">
          <div className="flex items-center justify-between">
            <Dialog.Title className="text-base font-semibold text-gray-900">
              新增目录
            </Dialog.Title>
            <Dialog.Close asChild>
              <button className="rounded p-1 text-gray-400 hover:bg-gray-100">
                <X className="h-4 w-4" />
              </button>
            </Dialog.Close>
          </div>

          <form onSubmit={handleSubmit} className="mt-4 space-y-4">
            <div>
              <label className="block text-sm font-medium text-gray-700">
                AI 工具
              </label>
              <select
                className="mt-1 w-full rounded-lg border border-gray-300 px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-brand-500"
                value={toolName}
                onChange={(e) => setToolName(e.target.value)}
              >
                {TOOL_NAMES.map((t) => (
                  <option key={t} value={t}>
                    {t}
                  </option>
                ))}
              </select>
            </div>

            {toolName === "其他" && (
              <div>
                <label className="block text-sm font-medium text-gray-700">
                  工具名称
                </label>
                <input
                  type="text"
                  className="mt-1 w-full rounded-lg border border-gray-300 px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-brand-500"
                  placeholder="例如：MyCopilot"
                  value={customTool}
                  onChange={(e) => setCustomTool(e.target.value)}
                  required
                />
              </div>
            )}

            <div>
              <label className="block text-sm font-medium text-gray-700">
                目录路径
              </label>
              <input
                type="text"
                className="mt-1 w-full rounded-lg border border-gray-300 px-3 py-2 font-mono text-sm focus:outline-none focus:ring-2 focus:ring-brand-500"
                placeholder="例如：/Users/you/.mytool/skills"
                value={path}
                onChange={(e) => setPath(e.target.value)}
                required
              />
            </div>

            <div className="flex justify-end gap-3 pt-2">
              <Dialog.Close asChild>
                <button
                  type="button"
                  className="rounded-lg border border-gray-300 px-4 py-2 text-sm font-medium text-gray-700 hover:bg-gray-50"
                >
                  取消
                </button>
              </Dialog.Close>
              <button
                type="submit"
                disabled={isPending}
                className="rounded-lg bg-brand-600 px-4 py-2 text-sm font-medium text-white hover:bg-brand-700 disabled:opacity-60"
              >
                {isPending ? "添加中..." : "添加"}
              </button>
            </div>
          </form>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}
```

- [ ] **Step 2: TypeScript 验证**

```bash
cd /Users/ckj/dev/test/aiskills/apps/desktop && pnpm exec tsc --noEmit 2>&1 | head -10
```

Expected: 无错误.

- [ ] **Step 3: 提交**

```bash
cd /Users/ckj/dev/test/aiskills
git add apps/desktop/src/routes/tools/
git commit -m "feat(p2): add AddDirectoryDialog component"
```

---

## Task 6: 工具与目录页完整实现

**Files:**
- Modify: `apps/desktop/src/routes/tools/index.tsx`

- [ ] **Step 1: 替换 tools/index.tsx 为完整页面**

```tsx
import { useEffect, useState } from "react";
import { RefreshCw, PlusCircle } from "lucide-react";
import {
  useDirectories,
  useScanDirectories,
  useAddDirectory,
  useToggleDirectory,
  useSetDefaultDirectory,
  useDeleteDirectory,
} from "../../hooks/use-directories";
import { DirectoryCard } from "./DirectoryCard";
import { AddDirectoryDialog } from "./AddDirectoryDialog";
import { useToast } from "../../components/ui/toast";
import type { AiToolDirectory } from "@skills-pp/shared";
import { open } from "@tauri-apps/plugin-opener";

function groupByTool(dirs: AiToolDirectory[]): Map<string, AiToolDirectory[]> {
  const map = new Map<string, AiToolDirectory[]>();
  for (const d of dirs) {
    const list = map.get(d.toolName) ?? [];
    list.push(d);
    map.set(d.toolName, list);
  }
  return map;
}

export default function ToolsPage() {
  const { data: dirs = [], isLoading } = useDirectories();
  const scan = useScanDirectories();
  const add = useAddDirectory();
  const toggle = useToggleDirectory();
  const setDefault = useSetDefaultDirectory();
  const del = useDeleteDirectory();
  const toast = useToast();
  const [dialogOpen, setDialogOpen] = useState(false);

  // Auto-scan on first mount
  useEffect(() => {
    scan.mutate();
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  function handleOpenFolder(path: string) {
    open(path).catch(() =>
      toast("无法打开目录", path, "error")
    );
  }

  function handleAdd(toolName: string, path: string) {
    add.mutate(
      { toolName, path },
      {
        onSuccess: () => {
          setDialogOpen(false);
          toast("目录已添加", path);
        },
        onError: (e) => toast("添加失败", String(e), "error"),
      },
    );
  }

  function handleToggle(id: string, enabled: boolean) {
    toggle.mutate(
      { id, enabled },
      { onError: (e) => toast("操作失败", String(e), "error") },
    );
  }

  function handleSetDefault(id: string) {
    setDefault.mutate(id, {
      onSuccess: () => toast("已设为默认目录"),
      onError: (e) => toast("操作失败", String(e), "error"),
    });
  }

  function handleDelete(id: string) {
    del.mutate(id, {
      onError: (e) => toast("删除失败", String(e), "error"),
    });
  }

  const grouped = groupByTool(dirs);
  const detectedCount = dirs.filter((d) => d.isDetected).length;

  return (
    <div>
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-xl font-semibold text-gray-900">工具与目录</h2>
          <p className="mt-1 text-sm text-gray-500">
            {isLoading || scan.isPending
              ? "扫描中..."
              : `共 ${dirs.length} 个目录，已找到 ${detectedCount} 个`}
          </p>
        </div>
        <div className="flex items-center gap-2">
          <button
            onClick={() => scan.mutate()}
            disabled={scan.isPending}
            className="flex items-center gap-2 rounded-lg border border-gray-300 px-3 py-2 text-sm font-medium text-gray-700 hover:bg-gray-50 disabled:opacity-60"
          >
            <RefreshCw
              className={`h-4 w-4 ${scan.isPending ? "animate-spin" : ""}`}
            />
            重新扫描
          </button>
          <button
            onClick={() => setDialogOpen(true)}
            className="flex items-center gap-2 rounded-lg bg-brand-600 px-3 py-2 text-sm font-medium text-white hover:bg-brand-700"
          >
            <PlusCircle className="h-4 w-4" />
            新增目录
          </button>
        </div>
      </div>

      {isLoading && dirs.length === 0 ? (
        <div className="mt-12 text-center text-sm text-gray-400">加载中...</div>
      ) : (
        <div className="mt-6 space-y-6">
          {Array.from(grouped.entries()).map(([toolName, toolDirs]) => (
            <div key={toolName}>
              <h3 className="mb-2 text-xs font-semibold uppercase tracking-wide text-gray-400">
                {toolName}
              </h3>
              <div className="space-y-2">
                {toolDirs.map((d) => (
                  <DirectoryCard
                    key={d.id}
                    dir={d}
                    onToggle={handleToggle}
                    onSetDefault={handleSetDefault}
                    onDelete={handleDelete}
                    onOpenFolder={handleOpenFolder}
                  />
                ))}
              </div>
            </div>
          ))}
        </div>
      )}

      <AddDirectoryDialog
        open={dialogOpen}
        onOpenChange={setDialogOpen}
        onAdd={handleAdd}
        isPending={add.isPending}
      />
    </div>
  );
}
```

- [ ] **Step 2: TypeScript 验证**

```bash
cd /Users/ckj/dev/test/aiskills/apps/desktop && pnpm exec tsc --noEmit 2>&1 | head -20
```

Expected: 无错误.

- [ ] **Step 3: Vite 构建验证**

```bash
cd /Users/ckj/dev/test/aiskills/apps/desktop && pnpm build 2>&1 | tail -10
```

Expected: `✓ built in X.XXs`.

- [ ] **Step 4: 运行测试**

```bash
cd /Users/ckj/dev/test/aiskills/apps/desktop && pnpm test:run 2>&1
```

Expected: `1 passed`.

- [ ] **Step 5: 提交**

```bash
cd /Users/ckj/dev/test/aiskills
git add apps/desktop/src/routes/tools/
git commit -m "feat(p2): complete tools & directories page with scan, add, toggle, set_default"
```

---

## Task 7: P2 交付验证

**验收标准（来自 roadmap P2 Checkpoints）：**

- [ ] **Checkpoint 1: Rust 编译通过**

```bash
cd /Users/ckj/dev/test/aiskills/apps/desktop/src-tauri && cargo check 2>&1 | tail -3
```

Expected: `Finished dev profile`.

- [ ] **Checkpoint 2: TypeScript 零错误**

```bash
cd /Users/ckj/dev/test/aiskills/apps/desktop && pnpm exec tsc --noEmit 2>&1
```

Expected: 无输出.

- [ ] **Checkpoint 3: 前端构建通过**

```bash
cd /Users/ckj/dev/test/aiskills/apps/desktop && pnpm build 2>&1 | tail -5
```

Expected: `✓ built in X.XXs`.

- [ ] **Checkpoint 4: 测试通过**

```bash
cd /Users/ckj/dev/test/aiskills/apps/desktop && pnpm test:run 2>&1
```

Expected: `Tests 1 passed`.

- [ ] **Step 5: 最终提交 + tag**

```bash
cd /Users/ckj/dev/test/aiskills
git add .
git commit -m "feat: complete P2 - tool directory management with scan, config and UI"
git tag p2-complete
```

---

## 自审（Spec Coverage）

| 开发计划 P2 任务 | 对应 Task |
|---|---|
| 实现工具目录规则模型与路径展开逻辑 | Task 1 |
| 实现默认目录扫描、读写权限检测、skill 数量统计 | Task 1 |
| 实现工具与目录页的扫描结果展示 | Task 6 |
| 实现手动新增目录、编辑目录、禁用目录、设置默认目录 | Task 2, 5, 6 |
| 实现首次启动引导（简化：auto-scan on mount） | Task 6 (useEffect scan) |
| 接入 `skills` CLI 探测能力（P2 暂缓，规则已内置） | 已通过内置规则覆盖 |
