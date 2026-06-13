# skills++ P3 来源站聚合与发现页 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans

**Goal:** 实现 5 个来源站适配器（2 个真实数据 + 3 个 stub）、SQLite 缓存层、发现页（搜索/筛选/排序）、skill 详情页和来源站启用/禁用设置。

**Architecture:** Rust 侧用 `reqwest`（async）实现 SourceAdapter trait，各适配器统一输出 `SkillItem`，结果写入 `skill_cache` 表（TTL=30分钟）。前端发现页通过 `list_skills` / `refresh_source` 命令消费缓存，TanStack Query 管理前端状态，React Router 详情页接收 skillId query param。

**Tech Stack:** 已有栈 + `reqwest` (async, TLS) + Tauri async commands

---

## Task 1: Rust HTTP 基础 + SourceAdapter trait

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Create: `src-tauri/src/services/source.rs`
- Modify: `src-tauri/src/services/mod.rs`
- Modify: `src-tauri/src/models/mod.rs`

- [ ] **Step 1: 添加 reqwest 到 Cargo.toml**

```toml
reqwest = { version = "0.12", features = ["json", "rustls-tls"], default-features = false }
tokio = { version = "1", features = ["rt-multi-thread"] }
```

- [ ] **Step 2: 更新 models/mod.rs — 新增 SkillItem 和 SourceRow**

追加到已有内容：
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillItem {
    pub id: String,
    pub name: String,
    pub author: Option<String>,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub source_id: String,
    pub repo_url: Option<String>,
    pub detail_url: String,
    pub updated_at: Option<String>,
    pub compatible_tools: Vec<String>,
    pub stars: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceRow {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub enabled: bool,
}
```

- [ ] **Step 3: 创建 services/source.rs**

```rust
use crate::models::SkillItem;

pub trait SourceAdapter: Send + Sync {
    fn source_id(&self) -> &'static str;
    fn source_name(&self) -> &'static str;
    fn base_url(&self) -> &'static str;
    fn fetch(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<SkillItem>, String>> + Send>>;
}
```

- [ ] **Step 4: 更新 services/mod.rs**

```rust
pub mod directory;
pub mod source;
```

- [ ] **Step 5: cargo check**

```bash
cd /Users/ckj/dev/test/aiskills/apps/desktop/src-tauri && cargo check 2>&1 | tail -5
```

Expected: `Finished dev profile`.

- [ ] **Step 6: 提交**

```bash
cd /Users/ckj/dev/test/aiskills
git add apps/desktop/src-tauri/
git commit -m "feat(p3): add SourceAdapter trait, SkillItem model and reqwest dependency"
```

---

## Task 2: GitHub Search Adapter（skills.sh 生态）

**Files:**
- Create: `src-tauri/src/services/adapters/github.rs`
- Create: `src-tauri/src/services/adapters/mod.rs`

- [ ] **Step 1: 创建 adapters/github.rs**

GitHub Search API 按 topic 搜索 skill repos，支持 `claude-skill`、`copilot-skill`、`codex-skill`、`gemini-skill`。

```rust
use crate::models::SkillItem;
use crate::services::source::SourceAdapter;
use serde::Deserialize;

pub struct GithubAdapter;

#[derive(Deserialize)]
struct GithubSearchResponse {
    items: Vec<GithubRepo>,
}

#[derive(Deserialize)]
struct GithubRepo {
    id: u64,
    full_name: String,
    description: Option<String>,
    html_url: String,
    stargazers_count: i64,
    updated_at: String,
    topics: Vec<String>,
    owner: GithubOwner,
    name: String,
}

#[derive(Deserialize)]
struct GithubOwner {
    login: String,
}

const TOPICS: &[&str] = &[
    "claude-skill",
    "codex-skill",
    "copilot-skill",
    "gemini-skill",
    "opencode-skill",
    "ai-skill",
];

fn infer_tools(topics: &[String]) -> Vec<String> {
    let mut tools = vec![];
    let t = topics.join(" ").to_lowercase();
    if t.contains("claude") { tools.push("Claude".to_string()); }
    if t.contains("codex") { tools.push("Codex".to_string()); }
    if t.contains("copilot") { tools.push("GitHub Copilot".to_string()); }
    if t.contains("gemini") { tools.push("Gemini CLI".to_string()); }
    if t.contains("cursor") { tools.push("Cursor".to_string()); }
    if t.contains("opencode") { tools.push("OpenCode".to_string()); }
    if tools.is_empty() { tools.push("通用".to_string()); }
    tools
}

impl SourceAdapter for GithubAdapter {
    fn source_id(&self) -> &'static str { "skills_sh" }
    fn source_name(&self) -> &'static str { "skills.sh" }
    fn base_url(&self) -> &'static str { "https://skills.sh" }

    fn fetch(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<SkillItem>, String>> + Send>> {
        Box::pin(async {
            let client = reqwest::Client::builder()
                .user_agent("skills-plus-plus/0.1")
                .timeout(std::time::Duration::from_secs(15))
                .build()
                .map_err(|e| e.to_string())?;

            let mut all_items: Vec<SkillItem> = vec![];
            let mut seen_ids = std::collections::HashSet::new();

            for topic in TOPICS {
                let url = format!(
                    "https://api.github.com/search/repositories?q=topic:{topic}&sort=stars&order=desc&per_page=30"
                );
                let resp: GithubSearchResponse = client
                    .get(&url)
                    .header("Accept", "application/vnd.github+json")
                    .header("X-GitHub-Api-Version", "2022-11-28")
                    .send()
                    .await
                    .map_err(|e| e.to_string())?
                    .json()
                    .await
                    .map_err(|e| e.to_string())?;

                for repo in resp.items {
                    let id = format!("skills_sh_{}", repo.id);
                    if seen_ids.contains(&id) { continue; }
                    seen_ids.insert(id.clone());

                    let compatible_tools = infer_tools(&repo.topics);
                    let mut tags: Vec<String> = repo.topics
                        .iter()
                        .filter(|t| !t.ends_with("-skill"))
                        .cloned()
                        .collect();
                    if tags.is_empty() { tags.push("skill".to_string()); }

                    all_items.push(SkillItem {
                        id,
                        name: repo.name,
                        author: Some(repo.owner.login),
                        description: repo.description,
                        tags,
                        source_id: "skills_sh".to_string(),
                        repo_url: Some(repo.html_url.clone()),
                        detail_url: repo.html_url,
                        updated_at: Some(repo.updated_at),
                        compatible_tools,
                        stars: Some(repo.stargazers_count),
                    });
                }

                // Short delay to respect GitHub rate limits
                tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            }

            Ok(all_items)
        })
    }
}
```

- [ ] **Step 2: 创建 adapters/mod.rs**

```rust
pub mod github;
pub mod lobehub;
pub mod stub;

pub use github::GithubAdapter;
pub use lobehub::LobehubAdapter;
pub use stub::StubAdapter;
```

- [ ] **Step 3: cargo check**

```bash
cd /Users/ckj/dev/test/aiskills/apps/desktop/src-tauri && cargo check 2>&1 | tail -5
```

Expected: `Finished dev profile`.

- [ ] **Step 4: 提交**

```bash
cd /Users/ckj/dev/test/aiskills
git add apps/desktop/src-tauri/
git commit -m "feat(p3): add GitHub Search adapter for skills.sh ecosystem"
```

---

## Task 3: LobeHub + Stub 适配器

**Files:**
- Create: `src-tauri/src/services/adapters/lobehub.rs`
- Create: `src-tauri/src/services/adapters/stub.rs`

- [ ] **Step 1: 创建 adapters/lobehub.rs**

```rust
use crate::models::SkillItem;
use crate::services::source::SourceAdapter;
use serde::Deserialize;

pub struct LobehubAdapter;

#[derive(Deserialize)]
struct LobehubIndex {
    plugins: Vec<LobehubPlugin>,
}

#[derive(Deserialize)]
struct LobehubPlugin {
    identifier: String,
    author: Option<String>,
    homepage: Option<String>,
    #[serde(rename = "createdAt")]
    created_at: Option<String>,
    meta: LobehubMeta,
}

#[derive(Deserialize)]
struct LobehubMeta {
    title: String,
    description: Option<String>,
    tags: Option<Vec<String>>,
}

impl SourceAdapter for LobehubAdapter {
    fn source_id(&self) -> &'static str { "lobehub" }
    fn source_name(&self) -> &'static str { "LobeHub" }
    fn base_url(&self) -> &'static str { "https://lobehub.com/skills" }

    fn fetch(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<SkillItem>, String>> + Send>> {
        Box::pin(async {
            let client = reqwest::Client::builder()
                .user_agent("skills-plus-plus/0.1")
                .timeout(std::time::Duration::from_secs(15))
                .build()
                .map_err(|e| e.to_string())?;

            let index: LobehubIndex = client
                .get("https://chat-plugins.lobehub.com/index.json")
                .send()
                .await
                .map_err(|e| e.to_string())?
                .json()
                .await
                .map_err(|e| e.to_string())?;

            let items = index
                .plugins
                .into_iter()
                .map(|p| {
                    let detail_url = p.homepage.clone()
                        .unwrap_or_else(|| format!("https://lobehub.com/plugins/{}", p.identifier));
                    SkillItem {
                        id: format!("lobehub_{}", p.identifier),
                        name: p.meta.title,
                        author: p.author,
                        description: p.meta.description,
                        tags: p.meta.tags.unwrap_or_default(),
                        source_id: "lobehub".to_string(),
                        repo_url: p.homepage.clone(),
                        detail_url,
                        updated_at: p.created_at,
                        compatible_tools: vec!["通用".to_string()],
                        stars: None,
                    }
                })
                .collect();

            Ok(items)
        })
    }
}
```

- [ ] **Step 2: 创建 adapters/stub.rs（skillhub.cn / clawhub.ai / skillsmp.com 用）**

```rust
use crate::models::SkillItem;
use crate::services::source::SourceAdapter;

/// Stub adapter for sources without a public API.
pub struct StubAdapter {
    pub id: &'static str,
    pub name: &'static str,
    pub url: &'static str,
}

impl SourceAdapter for StubAdapter {
    fn source_id(&self) -> &'static str { self.id }
    fn source_name(&self) -> &'static str { self.name }
    fn base_url(&self) -> &'static str { self.url }

    fn fetch(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<SkillItem>, String>> + Send>> {
        let name = self.name;
        Box::pin(async move {
            // Stub: return empty list with informational message.
            // Will be replaced with a real adapter in a future phase.
            log::info!("{} adapter is a stub — returning empty list", name);
            Ok(vec![])
        })
    }
}
```

- [ ] **Step 3: cargo check**

```bash
cd /Users/ckj/dev/test/aiskills/apps/desktop/src-tauri && cargo check 2>&1 | tail -5
```

Expected: `Finished dev profile`.

- [ ] **Step 4: 提交**

```bash
cd /Users/ckj/dev/test/aiskills
git add apps/desktop/src-tauri/
git commit -m "feat(p3): add LobeHub adapter and stub adapters"
```

---

## Task 4: 来源聚合服务 + 缓存 + IPC 命令

**Files:**
- Create: `src-tauri/src/services/source_registry.rs`
- Create: `src-tauri/src/commands/source.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/repositories/db.rs`（seed sources）

- [ ] **Step 1: 更新 repositories/db.rs — 植入 5 个来源站初始数据**

在 `migrate` 函数末尾调用：`seed_sources(conn)?;`

新增函数：
```rust
pub fn seed_sources(conn: &Connection) -> SqliteResult<()> {
    let sources = &[
        ("skills_sh",  "skills.sh",   "https://skills.sh"),
        ("lobehub",    "LobeHub",     "https://lobehub.com/skills"),
        ("skillhub",   "SkillHub.cn", "https://skillhub.cn"),
        ("clawhub",    "ClawHub.ai",  "https://clawhub.ai/skills"),
        ("skillsmp",   "SkillsMP",    "https://skillsmp.com"),
    ];
    for (id, name, url) in sources {
        conn.execute(
            "INSERT OR IGNORE INTO skill_sources (id, name, base_url, enabled) VALUES (?1, ?2, ?3, 1)",
            rusqlite::params![id, name, url],
        )?;
    }
    Ok(())
}
```

- [ ] **Step 2: 创建 services/source_registry.rs**

```rust
use crate::models::SkillItem;
use crate::services::adapters::{GithubAdapter, LobehubAdapter, StubAdapter};
use crate::services::source::SourceAdapter;

pub struct SourceRegistry {
    adapters: Vec<Box<dyn SourceAdapter>>,
}

impl SourceRegistry {
    pub fn new() -> Self {
        SourceRegistry {
            adapters: vec![
                Box::new(GithubAdapter),
                Box::new(LobehubAdapter),
                Box::new(StubAdapter { id: "skillhub", name: "SkillHub.cn", url: "https://skillhub.cn" }),
                Box::new(StubAdapter { id: "clawhub", name: "ClawHub.ai", url: "https://clawhub.ai/skills" }),
                Box::new(StubAdapter { id: "skillsmp", name: "SkillsMP", url: "https://skillsmp.com" }),
            ],
        }
    }

    pub fn get_adapter(&self, source_id: &str) -> Option<&dyn SourceAdapter> {
        self.adapters.iter().find(|a| a.source_id() == source_id).map(|a| a.as_ref())
    }

    pub fn all_adapters(&self) -> &[Box<dyn SourceAdapter>] {
        &self.adapters
    }
}

impl Default for SourceRegistry {
    fn default() -> Self { Self::new() }
}
```

- [ ] **Step 3: 创建 commands/source.rs**

```rust
use crate::commands::app::DbState;
use crate::models::{SkillItem, SourceRow};
use crate::services::source_registry::SourceRegistry;
use rusqlite::params;
use tauri::State;

const CACHE_TTL_MINUTES: i64 = 30;

// ─── Source management ────────────────────────────────────────────────────────

#[tauri::command]
pub fn list_sources(db: State<DbState>) -> Result<Vec<SourceRow>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id, name, base_url, enabled FROM skill_sources ORDER BY name")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok(SourceRow {
                id: row.get(0)?,
                name: row.get(1)?,
                base_url: row.get(2)?,
                enabled: row.get::<_, i64>(3)? != 0,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn toggle_source(db: State<DbState>, id: String, enabled: bool) -> Result<(), String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE skill_sources SET enabled = ?1 WHERE id = ?2",
        params![enabled as i64, id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

// ─── Skill cache ─────────────────────────────────────────────────────────────

fn store_skills(conn: &rusqlite::Connection, source_id: &str, items: &[SkillItem]) -> Result<(), String> {
    conn.execute("DELETE FROM skill_cache WHERE source_id = ?1", params![source_id])
        .map_err(|e| e.to_string())?;
    for item in items {
        let tags = serde_json::to_string(&item.tags).unwrap_or_default();
        let tools = serde_json::to_string(&item.compatible_tools).unwrap_or_default();
        conn.execute(
            "INSERT OR REPLACE INTO skill_cache \
             (id, source_id, name, author, description, tags, repo_url, detail_url, updated_at, compatible_tools) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                item.id, source_id, item.name, item.author, item.description,
                tags, item.repo_url, item.detail_url, item.updated_at, tools,
            ],
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn load_skills_from_cache(conn: &rusqlite::Connection, source_ids: &[String]) -> Result<Vec<SkillItem>, String> {
    let placeholders: String = source_ids
        .iter()
        .enumerate()
        .map(|(i, _)| format!("?{}", i + 1))
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        "SELECT id, source_id, name, author, description, tags, repo_url, detail_url, updated_at, compatible_tools \
         FROM skill_cache WHERE source_id IN ({placeholders}) ORDER BY name"
    );
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let params: Vec<&dyn rusqlite::ToSql> = source_ids.iter().map(|s| s as &dyn rusqlite::ToSql).collect();
    let rows = stmt
        .query_map(params.as_slice(), |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, Option<String>>(6)?,
                row.get::<_, String>(7)?,
                row.get::<_, Option<String>>(8)?,
                row.get::<_, String>(9)?,
            ))
        })
        .map_err(|e| e.to_string())?;

    let mut items = vec![];
    for row in rows {
        let (id, source_id, name, author, description, tags_s, repo_url, detail_url, updated_at, tools_s) =
            row.map_err(|e| e.to_string())?;
        let tags: Vec<String> = serde_json::from_str(&tags_s).unwrap_or_default();
        let compatible_tools: Vec<String> = serde_json::from_str(&tools_s).unwrap_or_default();
        items.push(SkillItem {
            id, name, author, description, tags, source_id, repo_url, detail_url, updated_at,
            compatible_tools, stars: None,
        });
    }
    Ok(items)
}

fn cache_is_fresh(conn: &rusqlite::Connection, source_id: &str) -> bool {
    conn.query_row(
        "SELECT COUNT(*) FROM skill_cache \
         WHERE source_id = ?1 \
         AND datetime(cached_at, '+' || ?2 || ' minutes') > datetime('now')",
        params![source_id, CACHE_TTL_MINUTES],
        |row| row.get::<_, i64>(0),
    )
    .unwrap_or(0) > 0
}

// ─── Main commands ────────────────────────────────────────────────────────────

/// Return cached skills for all enabled sources. Does not trigger network fetch.
#[tauri::command]
pub fn list_skills(db: State<DbState>) -> Result<Vec<SkillItem>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id FROM skill_sources WHERE enabled = 1")
        .map_err(|e| e.to_string())?;
    let source_ids: Vec<String> = stmt
        .query_map([], |row| row.get(0))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    load_skills_from_cache(&conn, &source_ids)
}

/// Fetch fresh skills from a single source and update cache.
#[tauri::command]
pub async fn refresh_source(
    db: State<'_, DbState>,
    source_id: String,
) -> Result<Vec<SkillItem>, String> {
    let registry = SourceRegistry::new();
    let adapter = registry
        .get_adapter(&source_id)
        .ok_or_else(|| format!("Unknown source: {source_id}"))?;

    let items = adapter.fetch().await?;

    {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        store_skills(&conn, &source_id, &items)?;
    }

    Ok(items)
}

/// Refresh all enabled sources that have stale/empty cache.
#[tauri::command]
pub async fn refresh_all_sources(db: State<'_, DbState>) -> Result<Vec<SkillItem>, String> {
    let source_ids: Vec<String> = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare("SELECT id FROM skill_sources WHERE enabled = 1")
            .map_err(|e| e.to_string())?;
        stmt.query_map([], |row| row.get(0))
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?
    };

    let registry = SourceRegistry::new();

    for sid in &source_ids {
        let is_fresh = {
            let conn = db.0.lock().map_err(|e| e.to_string())?;
            cache_is_fresh(&conn, sid)
        };
        if is_fresh { continue; }

        if let Some(adapter) = registry.get_adapter(sid) {
            match adapter.fetch().await {
                Ok(items) => {
                    let conn = db.0.lock().map_err(|e| e.to_string())?;
                    let _ = store_skills(&conn, sid, &items);
                }
                Err(e) => log::warn!("Failed to fetch {sid}: {e}"),
            }
        }
    }

    let conn = db.0.lock().map_err(|e| e.to_string())?;
    load_skills_from_cache(&conn, &source_ids)
}

/// Get a single skill by ID.
#[tauri::command]
pub fn get_skill(db: State<DbState>, id: String) -> Result<Option<SkillItem>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let result = conn.query_row(
        "SELECT id, source_id, name, author, description, tags, repo_url, detail_url, updated_at, compatible_tools \
         FROM skill_cache WHERE id = ?1",
        params![id],
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, Option<String>>(6)?,
                row.get::<_, String>(7)?,
                row.get::<_, Option<String>>(8)?,
                row.get::<_, String>(9)?,
            ))
        },
    );
    match result {
        Ok((id, source_id, name, author, description, tags_s, repo_url, detail_url, updated_at, tools_s)) => {
            let tags: Vec<String> = serde_json::from_str(&tags_s).unwrap_or_default();
            let compatible_tools: Vec<String> = serde_json::from_str(&tools_s).unwrap_or_default();
            Ok(Some(SkillItem { id, name, author, description, tags, source_id, repo_url, detail_url, updated_at, compatible_tools, stars: None }))
        }
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}
```

- [ ] **Step 4: 更新 commands/mod.rs**

```rust
pub mod app;
pub mod directory;
pub mod source;
pub use app::*;
pub use directory::*;
pub use source::*;
```

- [ ] **Step 5: 更新 lib.rs — 注册新命令 + 调用 seed_sources**

在 setup 的 `db::migrate(&conn)` 后加：
```rust
repositories::db::seed_sources(&conn).expect("Failed to seed sources");
```

在 `invoke_handler` 中追加：
```rust
commands::source::list_sources,
commands::source::toggle_source,
commands::source::list_skills,
commands::source::refresh_source,
commands::source::refresh_all_sources,
commands::source::get_skill,
```

- [ ] **Step 6: cargo check**

```bash
cd /Users/ckj/dev/test/aiskills/apps/desktop/src-tauri && cargo check 2>&1 | tail -5
```

Expected: `Finished dev profile`.

- [ ] **Step 7: 提交**

```bash
cd /Users/ckj/dev/test/aiskills
git add apps/desktop/src-tauri/
git commit -m "feat(p3): add source registry, cache layer and skill IPC commands"
```

---

## Task 5: 前端 IPC + Skill Hooks

**Files:**
- Modify: `apps/desktop/src/lib/ipc.ts`
- Create: `apps/desktop/src/hooks/use-skills.ts`
- Create: `apps/desktop/src/hooks/use-sources.ts`

- [ ] **Step 1: 更新 ipc.ts**

```typescript
// 追加以下方法：
listSources: (): Promise<SkillSource[]> => invoke("list_sources"),
toggleSource: (id: string, enabled: boolean): Promise<void> =>
  invoke("toggle_source", { id, enabled }),
listSkills: (): Promise<SkillItem[]> => invoke("list_skills"),
refreshSource: (sourceId: string): Promise<SkillItem[]> =>
  invoke("refresh_source", { sourceId }),
refreshAllSources: (): Promise<SkillItem[]> => invoke("refresh_all_sources"),
getSkill: (id: string): Promise<SkillItem | null> =>
  invoke("get_skill", { id }),
```

- [ ] **Step 2: 创建 hooks/use-skills.ts**

```typescript
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { ipc } from "../lib/ipc";

export const SKILLS_KEY = ["skills"] as const;

export function useSkills() {
  return useQuery({
    queryKey: SKILLS_KEY,
    queryFn: () => ipc.listSkills(),
  });
}

export function useRefreshAllSources() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: () => ipc.refreshAllSources(),
    onSuccess: (data) => qc.setQueryData(SKILLS_KEY, data),
  });
}

export function useRefreshSource() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (sourceId: string) => ipc.refreshSource(sourceId),
    onSuccess: () => qc.invalidateQueries({ queryKey: SKILLS_KEY }),
  });
}

export function useSkill(id: string) {
  return useQuery({
    queryKey: ["skill", id],
    queryFn: () => ipc.getSkill(id),
    enabled: !!id,
  });
}
```

- [ ] **Step 3: 创建 hooks/use-sources.ts**

```typescript
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { ipc } from "../lib/ipc";

const SOURCES_KEY = ["sources"] as const;

export function useSources() {
  return useQuery({
    queryKey: SOURCES_KEY,
    queryFn: () => ipc.listSources(),
  });
}

export function useToggleSource() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ id, enabled }: { id: string; enabled: boolean }) =>
      ipc.toggleSource(id, enabled),
    onSuccess: () => qc.invalidateQueries({ queryKey: SOURCES_KEY }),
  });
}
```

- [ ] **Step 4: TypeScript 验证**

```bash
cd /Users/ckj/dev/test/aiskills/apps/desktop && pnpm exec tsc --noEmit 2>&1 | head -10
```

Expected: 无错误.

- [ ] **Step 5: 提交**

```bash
cd /Users/ckj/dev/test/aiskills
git add apps/desktop/src/
git commit -m "feat(p3): add skill and source IPC wrappers and React Query hooks"
```

---

## Task 6: 发现页（搜索/筛选/排序）

**Files:**
- Modify: `apps/desktop/src/routes/discover/index.tsx`
- Create: `apps/desktop/src/routes/discover/SkillCard.tsx`
- Create: `apps/desktop/src/routes/discover/FilterBar.tsx`

- [ ] **Step 1: 创建 SkillCard.tsx**

```tsx
import type { SkillItem } from "@skills-pp/shared";
import { ExternalLink, Star } from "lucide-react";
import { useNavigate } from "react-router-dom";

interface Props {
  skill: SkillItem;
}

export function SkillCard({ skill }: Props) {
  const navigate = useNavigate();

  return (
    <button
      className="w-full rounded-lg border border-gray-200 bg-white p-4 text-left transition-shadow hover:shadow-md"
      onClick={() => navigate(`/skill/${encodeURIComponent(skill.id)}`)}
    >
      <div className="flex items-start justify-between gap-3">
        <div className="min-w-0 flex-1">
          <div className="flex items-center gap-2">
            <span className="truncate text-sm font-semibold text-gray-900">
              {skill.name}
            </span>
            {skill.stars != null && (
              <span className="flex items-center gap-1 text-xs text-yellow-600">
                <Star className="h-3 w-3" />
                {skill.stars}
              </span>
            )}
          </div>
          {skill.author && (
            <p className="mt-0.5 text-xs text-gray-400">by {skill.author}</p>
          )}
          {skill.description && (
            <p className="mt-1 line-clamp-2 text-xs text-gray-500">
              {skill.description}
            </p>
          )}
          <div className="mt-2 flex flex-wrap gap-1">
            {skill.tags.slice(0, 4).map((tag) => (
              <span
                key={tag}
                className="rounded-full bg-gray-100 px-2 py-0.5 text-xs text-gray-500"
              >
                {tag}
              </span>
            ))}
            {skill.compatibleTools?.slice(0, 2).map((tool) => (
              <span
                key={tool}
                className="rounded-full bg-brand-50 px-2 py-0.5 text-xs text-brand-700"
              >
                {tool}
              </span>
            ))}
          </div>
        </div>
        <ExternalLink className="mt-0.5 h-3.5 w-3.5 shrink-0 text-gray-300" />
      </div>
    </button>
  );
}
```

- [ ] **Step 2: 创建 FilterBar.tsx**

```tsx
import type { SkillSource } from "@skills-pp/shared";

interface Props {
  sources: SkillSource[];
  selectedSource: string;
  selectedTool: string;
  onSourceChange: (v: string) => void;
  onToolChange: (v: string) => void;
  allTools: string[];
}

export function FilterBar({
  sources, selectedSource, selectedTool,
  onSourceChange, onToolChange, allTools,
}: Props) {
  return (
    <div className="flex flex-wrap gap-3">
      <select
        className="rounded-lg border border-gray-300 px-3 py-1.5 text-sm focus:outline-none focus:ring-2 focus:ring-brand-500"
        value={selectedSource}
        onChange={(e) => onSourceChange(e.target.value)}
      >
        <option value="">全部来源</option>
        {sources.filter((s) => s.enabled).map((s) => (
          <option key={s.id} value={s.id}>{s.name}</option>
        ))}
      </select>

      <select
        className="rounded-lg border border-gray-300 px-3 py-1.5 text-sm focus:outline-none focus:ring-2 focus:ring-brand-500"
        value={selectedTool}
        onChange={(e) => onToolChange(e.target.value)}
      >
        <option value="">全部工具</option>
        {allTools.map((t) => (
          <option key={t} value={t}>{t}</option>
        ))}
      </select>
    </div>
  );
}
```

- [ ] **Step 3: 更新 discover/index.tsx 为完整页面**

```tsx
import { useEffect, useMemo, useState } from "react";
import { RefreshCw, Search } from "lucide-react";
import { useSkills, useRefreshAllSources } from "../../hooks/use-skills";
import { useSources } from "../../hooks/use-sources";
import { SkillCard } from "./SkillCard";
import { FilterBar } from "./FilterBar";
import { useToast } from "../../components/ui/toast";
import type { SkillItem } from "@skills-pp/shared";

function useUniqueTools(skills: SkillItem[]) {
  return useMemo(() => {
    const set = new Set<string>();
    for (const s of skills) s.compatibleTools?.forEach((t) => set.add(t));
    return Array.from(set).sort();
  }, [skills]);
}

export default function DiscoverPage() {
  const { data: skills = [], isLoading } = useSkills();
  const { data: sources = [] } = useSources();
  const refresh = useRefreshAllSources();
  const toast = useToast();

  const [query, setQuery] = useState("");
  const [selectedSource, setSelectedSource] = useState("");
  const [selectedTool, setSelectedTool] = useState("");

  const allTools = useUniqueTools(skills);

  // Auto-refresh on first mount if cache is empty
  useEffect(() => {
    if (!isLoading && skills.length === 0) {
      refresh.mutate(undefined, {
        onError: (e) => toast("刷新失败", String(e), "error"),
      });
    }
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [isLoading]);

  const filtered = useMemo(() => {
    const q = query.toLowerCase();
    return skills.filter((s) => {
      if (selectedSource && s.sourceId !== selectedSource) return false;
      if (selectedTool && !s.compatibleTools?.includes(selectedTool)) return false;
      if (q) {
        return (
          s.name.toLowerCase().includes(q) ||
          s.description?.toLowerCase().includes(q) ||
          s.author?.toLowerCase().includes(q) ||
          s.tags.some((t) => t.toLowerCase().includes(q))
        );
      }
      return true;
    });
  }, [skills, query, selectedSource, selectedTool]);

  return (
    <div>
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-xl font-semibold text-gray-900">发现</h2>
          <p className="mt-1 text-sm text-gray-500">
            {isLoading || refresh.isPending
              ? "加载中..."
              : `${filtered.length} / ${skills.length} 个 skill`}
          </p>
        </div>
        <button
          onClick={() =>
            refresh.mutate(undefined, {
              onError: (e) => toast("刷新失败", String(e), "error"),
            })
          }
          disabled={refresh.isPending}
          className="flex items-center gap-2 rounded-lg border border-gray-300 px-3 py-2 text-sm font-medium text-gray-700 hover:bg-gray-50 disabled:opacity-60"
        >
          <RefreshCw className={`h-4 w-4 ${refresh.isPending ? "animate-spin" : ""}`} />
          刷新来源
        </button>
      </div>

      {/* Search */}
      <div className="relative mt-4">
        <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-gray-400" />
        <input
          type="text"
          className="w-full rounded-lg border border-gray-300 py-2 pl-9 pr-4 text-sm focus:outline-none focus:ring-2 focus:ring-brand-500"
          placeholder="搜索 skill 名称、描述或标签..."
          value={query}
          onChange={(e) => setQuery(e.target.value)}
        />
      </div>

      {/* Filters */}
      <div className="mt-3">
        <FilterBar
          sources={sources}
          selectedSource={selectedSource}
          selectedTool={selectedTool}
          onSourceChange={setSelectedSource}
          onToolChange={setSelectedTool}
          allTools={allTools}
        />
      </div>

      {/* Results */}
      <div className="mt-5 grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
        {filtered.map((s) => (
          <SkillCard key={s.id} skill={s} />
        ))}
      </div>

      {!isLoading && !refresh.isPending && filtered.length === 0 && (
        <div className="mt-16 text-center text-sm text-gray-400">
          {skills.length === 0 ? "点击「刷新来源」加载 skill 数据" : "没有匹配的 skill"}
        </div>
      )}
    </div>
  );
}
```

- [ ] **Step 4: TypeScript 验证**

```bash
cd /Users/ckj/dev/test/aiskills/apps/desktop && pnpm exec tsc --noEmit 2>&1 | head -15
```

Expected: 无错误.

- [ ] **Step 5: 提交**

```bash
cd /Users/ckj/dev/test/aiskills
git add apps/desktop/src/
git commit -m "feat(p3): complete discover page with search, filter and skill cards"
```

---

## Task 7: Skill 详情页

**Files:**
- Create: `apps/desktop/src/routes/skill/index.tsx`
- Modify: `apps/desktop/src/routes/index.tsx`

- [ ] **Step 1: 创建 routes/skill/index.tsx**

```tsx
import { useNavigate, useParams } from "react-router-dom";
import { ArrowLeft, ExternalLink, Github } from "lucide-react";
import { useSkill } from "../../hooks/use-skills";
import { openUrl } from "@tauri-apps/plugin-opener";

export default function SkillDetailPage() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const { data: skill, isLoading } = useSkill(id ? decodeURIComponent(id) : "");

  if (isLoading) {
    return (
      <div className="text-center text-sm text-gray-400 mt-20">加载中...</div>
    );
  }

  if (!skill) {
    return (
      <div className="mt-20 text-center">
        <p className="text-sm text-gray-400">Skill 不存在或已从缓存中移除</p>
        <button
          onClick={() => navigate(-1)}
          className="mt-4 text-sm text-brand-600 hover:underline"
        >
          返回
        </button>
      </div>
    );
  }

  return (
    <div className="mx-auto max-w-2xl">
      <button
        onClick={() => navigate(-1)}
        className="flex items-center gap-2 text-sm text-gray-500 hover:text-gray-700"
      >
        <ArrowLeft className="h-4 w-4" />
        返回
      </button>

      <div className="mt-6">
        <h2 className="text-2xl font-bold text-gray-900">{skill.name}</h2>
        {skill.author && (
          <p className="mt-1 text-sm text-gray-400">by {skill.author}</p>
        )}

        {skill.description && (
          <p className="mt-4 text-sm leading-relaxed text-gray-600">
            {skill.description}
          </p>
        )}

        <div className="mt-6 space-y-4 rounded-lg border border-gray-200 bg-white p-4">
          <DetailRow label="来源">
            <span className="text-sm text-gray-600">{skill.sourceId}</span>
          </DetailRow>

          {skill.updatedAt && (
            <DetailRow label="更新时间">
              <span className="text-sm text-gray-600">
                {new Date(skill.updatedAt).toLocaleDateString("zh-CN")}
              </span>
            </DetailRow>
          )}

          {skill.compatibleTools && skill.compatibleTools.length > 0 && (
            <DetailRow label="兼容工具">
              <div className="flex flex-wrap gap-1">
                {skill.compatibleTools.map((t) => (
                  <span
                    key={t}
                    className="rounded-full bg-brand-50 px-2 py-0.5 text-xs text-brand-700"
                  >
                    {t}
                  </span>
                ))}
              </div>
            </DetailRow>
          )}

          {skill.tags.length > 0 && (
            <DetailRow label="标签">
              <div className="flex flex-wrap gap-1">
                {skill.tags.map((tag) => (
                  <span
                    key={tag}
                    className="rounded-full bg-gray-100 px-2 py-0.5 text-xs text-gray-500"
                  >
                    {tag}
                  </span>
                ))}
              </div>
            </DetailRow>
          )}
        </div>

        <div className="mt-6 flex gap-3">
          {skill.repoUrl && (
            <button
              onClick={() => openUrl(skill.repoUrl!)}
              className="flex items-center gap-2 rounded-lg border border-gray-300 px-4 py-2 text-sm font-medium text-gray-700 hover:bg-gray-50"
            >
              <Github className="h-4 w-4" />
              查看仓库
            </button>
          )}
          <button
            onClick={() => openUrl(skill.detailUrl)}
            className="flex items-center gap-2 rounded-lg bg-brand-600 px-4 py-2 text-sm font-medium text-white hover:bg-brand-700"
          >
            <ExternalLink className="h-4 w-4" />
            打开详情
          </button>
        </div>
      </div>
    </div>
  );
}

function DetailRow({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="flex items-start gap-4">
      <span className="w-20 shrink-0 text-xs font-medium text-gray-400">{label}</span>
      <div className="flex-1">{children}</div>
    </div>
  );
}
```

- [ ] **Step 2: 添加路由到 routes/index.tsx**

在 `<Route element={<AppShell />}>` 内追加:
```tsx
import SkillDetailPage from "./skill/index";
// ...
<Route path="/skill/:id" element={<SkillDetailPage />} />
```

- [ ] **Step 3: TypeScript 验证 + 构建**

```bash
cd /Users/ckj/dev/test/aiskills/apps/desktop
pnpm exec tsc --noEmit 2>&1 | head -10
pnpm build 2>&1 | tail -5
```

Expected: 无错误，`✓ built in X.XXs`.

- [ ] **Step 4: 提交**

```bash
cd /Users/ckj/dev/test/aiskills
git add apps/desktop/src/
git commit -m "feat(p3): add skill detail page"
```

---

## Task 8: P3 交付验证

- [ ] **Rust 编译**

```bash
cd /Users/ckj/dev/test/aiskills/apps/desktop/src-tauri && cargo check 2>&1 | tail -3
```

- [ ] **TypeScript 零错误**

```bash
cd /Users/ckj/dev/test/aiskills/apps/desktop && pnpm exec tsc --noEmit 2>&1 && echo OK
```

- [ ] **前端构建**

```bash
cd /Users/ckj/dev/test/aiskills/apps/desktop && pnpm build 2>&1 | tail -3
```

- [ ] **测试通过**

```bash
cd /Users/ckj/dev/test/aiskills/apps/desktop && pnpm test:run 2>&1 | tail -5
```

- [ ] **最终提交**

```bash
cd /Users/ckj/dev/test/aiskills
git add .
git commit -m "feat: complete P3 - source aggregation, discover page and skill detail"
git tag p3-complete
```
