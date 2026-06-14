# 开发日志：skills_cli 调试、Victauri 桥接修复、状态检查修复

> 日期：2026-06-14
> 分支：`worktree-installskills` → 合并到 `main`
> 提交：
> - `f3d0d44` fix: skills_cli search SKILL.md in subdirs + withGlobalTauri for Victauri bridge
> - `7b18487` fix: skills_cli status check uses canonical path fallback

---

## 背景

之前实现了 4 种安装策略，其中 `skills_cli` 策略对齐 vercel-labs `npx skills`。但在实际使用中发现了 3 个问题，逐个排查修复。

---

## 问题 1：Victauri MCP eval 工具全部超时

### 症状

`eval_js`、`find_elements`、`invoke_command`、`navigate`、`input`、`logs` 等依赖 webview bridge 的工具全部报 "eval timed out after 30s"。只有 `screenshot`、`window` 等使用 native API 的工具正常工作。插件报告 `bridge_version: 0.3.0, registered_commands: 0`。

### 排查过程

1. 先用 curl + Python 脚本验证 MCP 协议套接字正常（HTTP 200，JSON-RPC 响应正确）
2. 逐步排查：确认端口、auth token、`Mcp-Session-Id` header、Accept header
3. 阅读 `victauri-plugin 0.8.0` 源码：`probe_bridge()` 注入 JS 调用 `window.__TAURI_INTERNALS__.invoke()`，2s 超时 → 说明 JS 注入成功但 `__TAURI_INTERNALS__` 不可用
4. 阅读 `js_bridge.rs`：`init_script()` 通过 `js_init_script` 注入，创建 `window.__VICTAURI__`
5. Web 搜索 "Tauri webview eval timeout macOS" → 发现 **WKWebView isolated content world** 问题

### 根因

macOS WKWebView 将注入的脚本运行在隔离的 JS 内容世界（isolated content world）中。在这个世界里 `window.__TAURI__` 和 `window.__TAURI_INTERNALS__` 都不存在，所以 bridge JS 调用 IPC 回调永远无法触发，导致 30s 超时。

### 修复

在 `tauri.conf.json` 的 `app` 段添加 `"withGlobalTauri": true`：

```json
"app": {
  "withGlobalTauri": true,
  "windows": [...]
}
```

这个选项将 Tauri API 暴露到所有内容世界，使注入脚本能正常调用 IPC。

### 附：Victauri MCP 传输要点

- 传输方式：普通 JSON-RPC over HTTP（POST `/mcp`），**非 SSE**
- **不需要** `Mcp-Session-Id` header（sessionless）
- `Accept` header 必须包含 `application/json, text/event-stream`
- Auth：`Authorization: Bearer dev-secret-unchanging`

---

## 问题 2：skills_cli 安装报 "installed directory has no SKILL.md"

### 症状

通过 `invoke_command` 安装 `last30days-skill`，install 任务状态为 `failed`，错误信息 `"installed directory has no SKILL.md"`。

### 排查过程

1. 查看 GitHub 仓库结构：`mvanhorn/last30days-skill` 的 SKILL.md 在 `skills/last30days/SKILL.md`，不在根目录
2. 阅读 `install_skills_cli()` 源码：发现它直接用 `git_clone(repo_url, &canonical)` 将仓库克隆到 `~/.agents/skills/last30days-skill/`，然后 `has_skill_md(&canonical)` 只检查根目录
3. 搜索 `find_skill_folder_with_name` → 不存在！之前 PR 中描述的这个修复实际并未实现

### 根因

`install_skills_cli` 原实现：
1. `git_clone` 整个仓库到 canonical 目录
2. 检查根目录是否有 SKILL.md
3. 没有 → 删除目录，返回失败

但许多仓库（尤其是 vercel-labs skills registry 的 repo）的 SKILL.md 在子目录中（如 `skills/<name>/SKILL.md`）。这与 `npx skills` 的行为不一致 — `npx skills` 会在仓库中搜索 SKILL.md。

### 修复

重写 `install_skills_cli()` 的数据获取流程：

```rust
// 新增函数
fn find_skill_folder_with_name(base_dir: &Path, skill_name: &str) -> Option<PathBuf>
  // BFS 搜索所有包含 SKILL.md 的目录
  // 按名称相似度（common_prefix_len）排序
  // 返回最佳匹配目录

fn copy_dir_contents(src: &Path, dst: &Path) -> Result<(), String>
  // 递归复制目录内容（类似 cp -r src/* dst/）

// 修改后的 install_skills_cli 流程
// 1. clone/download 到临时目录
// 2. find_skill_folder_with_name() 搜索 SKILL.md
// 3. copy_dir_contents() 复制到 canonical
// 4. 规范化目录名（按 SKILL.md frontmatter）
// 5. 创建 symlink + 写 lockfile
```

同时修复了 lockfile key 和 symlink 名字：原来用原始输入名（`last30days-skill`），现在从 SKILL.md frontmatter 的 `name:` 提取（`last30days`），确保与 `npx skills` 完全互通。

### 验证

```bash
npx skills list -g | grep last30days
# last30days  ~/.agents/skills/last30days
```

---

## 问题 3：skills_cli 已安装 skill 状态显示 "缺失"

### 症状

手动安装 `Agentic-SEO-Skill` 后，在「已安装」页面状态显示「缺失」，但实际文件系统完全正常：
- Canonical 目录 `~/.agents/skills/seo/` 存在，含 SKILL.md
- Symlink `~/.claude/skills/seo → ~/.agents/skills/seo` 存在

### 排查过程

1. `list_installed_skills` 的返回中 `Agentic-SEO-Skill` 的 `status: "missing"`
2. 同时 `last30days-skill`（上一步刚装好）也是 `status: "missing"`
3. 检查文件系统 — 两个 skill 的 canonical 目录和 symlink 都正常
4. 阅读 `compute_skill_status()` 源码

### 根因

`compute_skill_status()` 的检查逻辑：

```rust
let target = target_path(directory_path, skill_name);
// = /Users/ckj/.claude/skills/Agentic-SEO-Skill
// 实际 symlink 是 /Users/ckj/.claude/skills/seo
// → 路径不存在 → "missing"
```

对于 `skills_cli` 策略，symlink 的名字来自 SKILL.md frontmatter 的 `name:` 字段（`seo`），但 DB 的 `name` 字段存的是仓库原始名（`Agentic-SEO-Skill`）。两个名字不一致时，路径检查永远找不到 symlink。

### 修复

为 `compute_skill_status()` 增加 `canonical_path` 参数作为回退：

```rust
fn compute_skill_status(
    skill_name: &str,
    directory_path: &str,
    canonical_path: Option<&str>,  // 新增
) -> &'static str

// 当常规路径不存在时，回退检查 canonical_path
// canonical_path 存在且非空 → "ok"
// canonical_path 也不存在 → "missing"
```

---

## Git Worktree 踩坑

本次开发中一个值得记住的教训：**worktree 的文件路径和主仓库的文件路径是不同的**。

- 主仓库工作目录：`/Users/ckj/dev/test/aiskills/`
- Worktree 工作目录：`/Users/ckj/dev/test/aiskills/.claude/worktrees/installskills/`

用 Edit 工具修改文件时，如果指定的路径是主仓库路径（如 `/Users/ckj/dev/test/aiskills/apps/desktop/src-tauri/tauri.conf.json`），修改会落到主仓库的工作树，**不会反映到 worktree 中**。

在合并时才发现两边不一致：
- 主仓库有未提交的修改（因为 Edit 写到了主仓库）
- Worktree 的 working tree 是干净的（因为从未被修改）

**教训**：在 worktree 会话中，所有文件操作都应使用 worktree 路径（可通过 `git rev-parse --show-toplevel` 确认）。

---

## 总结

| 问题 | 根因 | 修复 | 文件 |
|------|------|------|------|
| Victauri eval 超时 | macOS WKWebView isolated content world | `tauri.conf.json` 加 `"withGlobalTauri": true` | `tauri.conf.json` |
| SKILL.md 找不到 | 只检查仓库根目录 | clone 到 temp → BFS 搜索 → 复制到 canonical | `services/install.rs` |
| 状态显示"缺失" | 路径检查用 DB name 而非实际 symlink 名 | `compute_skill_status` 增加 canonical_path 回退 | `services/install.rs` |

三项修复均已合并到 `main`，全部 60 项单元测试通过。
