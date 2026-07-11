use crate::commands::app::DbState;
use crate::models::{RefreshSourcesResult, RefreshWarning, SkillItem, SourceRow};
use crate::services::skill_md::{parse_frontmatter_and_strip, FrontmatterMeta};
use crate::services::source_registry::SourceRegistry;
use rusqlite::{params, Connection};
use std::sync::{Arc, Mutex};
use tauri::State;

const CACHE_TTL_MINUTES: i64 = 30;

// ─── Source management ─────────────────────────────────────────────────────

pub fn list_sources_inner(conn: &Connection) -> Result<Vec<SourceRow>, String> {
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
pub fn list_sources(db: State<DbState>) -> Result<Vec<SourceRow>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    list_sources_inner(&conn)
}

pub fn toggle_source_inner(conn: &Connection, id: String, enabled: bool) -> Result<(), String> {
    conn.execute(
        "UPDATE skill_sources SET enabled = ?1 WHERE id = ?2",
        params![enabled as i64, id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn toggle_source(db: State<DbState>, id: String, enabled: bool) -> Result<(), String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    toggle_source_inner(&conn, id, enabled)
}

// ─── Cache helpers ─────────────────────────────────────────────────────────

pub fn store_skills(conn: &Connection, source_id: &str, items: &[SkillItem]) -> Result<(), String> {
    conn.execute("DELETE FROM skill_cache WHERE source_id = ?1", params![source_id])
        .map_err(|e| e.to_string())?;
    for item in items {
        let tags = serde_json::to_string(&item.tags).unwrap_or_default();
        let tools = serde_json::to_string(&item.compatible_tools).unwrap_or_default();
        let strategy = item.install_strategy.map(|s| s.as_str().to_string());
        conn.execute(
            "INSERT OR REPLACE INTO skill_cache \
             (id, source_id, name, author, description, tags, repo_url, detail_url, updated_at, compatible_tools, install_strategy, archive_url, stars, category) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                item.id, source_id, item.name, item.author, item.description,
                tags, item.repo_url, item.detail_url, item.updated_at, tools,
                strategy, item.archive_url, item.stars, item.category,
            ],
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn load_skills(conn: &Connection, source_ids: &[String]) -> Result<Vec<SkillItem>, String> {
    if source_ids.is_empty() {
        return Ok(vec![]);
    }
    let ph: String = (1..=source_ids.len()).map(|i| format!("?{i}")).collect::<Vec<_>>().join(", ");
    let sql = format!(
        "SELECT id, source_id, name, author, description, tags, repo_url, detail_url, updated_at, compatible_tools, install_strategy, archive_url, stars, category \
         FROM skill_cache WHERE source_id IN ({ph}) \
         ORDER BY CASE source_id WHEN 'registry' THEN 0 WHEN 'skills_sh' THEN 1 ELSE 2 END, \
                  stars DESC NULLS LAST, \
                  name COLLATE NOCASE"
    );
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let params_refs: Vec<&dyn rusqlite::ToSql> = source_ids.iter().map(|s| s as &dyn rusqlite::ToSql).collect();
    let rows = stmt
        .query_map(params_refs.as_slice(), |row| map_skill_cache_row(row))
        .map_err(|e| e.to_string())?;

    let mut out = vec![];
    for row in rows {
        out.push(row.map_err(|e| e.to_string())?);
    }
    Ok(out)
}

/// 从 skill_cache 的一行映射出 SkillItem。
pub fn map_skill_cache_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<SkillItem> {
    use crate::models::InstallStrategy;
    let tags_s: String = row.get(5)?;
    let tools_s: String = row.get(9)?;
    let strategy_s: Option<String> = row.get(10)?;
    let install_strategy = strategy_s
        .as_deref()
        .map(|s| InstallStrategy::parse(s));
    Ok(SkillItem {
        id: row.get(0)?,
        name: row.get(2)?,
        author: row.get(3)?,
        description: row.get(4)?,
        source_id: row.get(1)?,
        repo_url: row.get(6)?,
        detail_url: row.get(7)?,
        updated_at: row.get(8)?,
        tags: serde_json::from_str(&tags_s).unwrap_or_default(),
        compatible_tools: serde_json::from_str(&tools_s).unwrap_or_default(),
        stars: row.get(12)?,
        install_strategy,
        archive_url: row.get(11)?,
        category: row.get(13)?,
    })
}

pub fn cache_is_fresh(conn: &Connection, source_id: &str) -> bool {
    conn.query_row(
        "SELECT COUNT(*) FROM skill_cache \
         WHERE source_id = ?1 \
         AND datetime(cached_at, '+' || ?2 || ' minutes') > datetime('now')",
        params![source_id, CACHE_TTL_MINUTES],
        |row| row.get::<_, i64>(0),
    )
    .unwrap_or(0) > 0
}

pub fn enabled_source_ids(conn: &Connection) -> Result<Vec<String>, String> {
    let mut stmt = conn
        .prepare("SELECT id FROM skill_sources WHERE enabled = 1")
        .map_err(|e| e.to_string())?;
    let ids: Vec<String> = stmt
        .query_map([], |row| row.get(0))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(ids)
}

// ─── Main commands ─────────────────────────────────────────────────────────

/// Return cached skills for all enabled sources (no network).
pub fn list_skills_inner(conn: &Connection) -> Result<Vec<SkillItem>, String> {
    let ids = enabled_source_ids(conn)?;
    load_skills(conn, &ids)
}

#[tauri::command]
pub fn list_skills(db: State<DbState>) -> Result<Vec<SkillItem>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    list_skills_inner(&conn)
}

/// Refresh one source and return its updated skills.
pub async fn refresh_source_inner(
    db: Arc<Mutex<Connection>>,
    source_id: String,
) -> Result<RefreshSourcesResult, String> {
    let registry = SourceRegistry::new();
    let adapter = registry
        .get_adapter(&source_id)
        .ok_or_else(|| format!("Unknown source: {source_id}"))?;

    let (items, warnings) = adapter.fetch_with_warnings().await?;

    {
        let conn = db.lock().map_err(|e| e.to_string())?;
        store_skills(&conn, &source_id, &items)?;
    }

    Ok(RefreshSourcesResult {
        skills: items,
        warnings: warnings
            .into_iter()
            .map(|message| RefreshWarning {
                source_id: source_id.clone(),
                message,
            })
            .collect(),
    })
}

#[tauri::command]
pub async fn refresh_source(
    db: State<'_, DbState>,
    source_id: String,
) -> Result<RefreshSourcesResult, String> {
    refresh_source_inner(std::sync::Arc::clone(&db.0), source_id).await
}

/// Refresh all enabled sources with stale cache.
pub async fn refresh_all_sources_inner(
    db: Arc<Mutex<Connection>>,
) -> Result<RefreshSourcesResult, String> {
    let source_ids: Vec<String> = {
        let conn = db.lock().map_err(|e| e.to_string())?;
        enabled_source_ids(&conn)?
    };

    let registry = SourceRegistry::new();
    let mut warnings: Vec<RefreshWarning> = vec![];

    for sid in &source_ids {
        let is_fresh = {
            let conn = db.lock().map_err(|e| e.to_string())?;
            cache_is_fresh(&conn, sid)
        };
        if is_fresh { continue; }

        if let Some(adapter) = registry.get_adapter(sid) {
            match adapter.fetch_with_warnings().await {
                Ok((items, adapter_warnings)) => {
                    let conn = db.lock().map_err(|e| e.to_string())?;
                    if let Err(e) = store_skills(&conn, sid, &items) {
                        log::warn!("Failed to cache {sid}: {e}");
                    }
                    warnings.extend(adapter_warnings.into_iter().map(|message| RefreshWarning {
                        source_id: sid.clone(),
                        message,
                    }));
                }
                Err(e) => log::warn!("Failed to fetch {sid}: {e}"),
            }
        }
    }

    let conn = db.lock().map_err(|e| e.to_string())?;
    Ok(RefreshSourcesResult {
        skills: load_skills(&conn, &source_ids)?,
        warnings,
    })
}

#[tauri::command]
pub async fn refresh_all_sources(db: State<'_, DbState>) -> Result<RefreshSourcesResult, String> {
    refresh_all_sources_inner(std::sync::Arc::clone(&db.0)).await
}

/// Get a single skill by ID.
pub fn get_skill_inner(conn: &Connection, id: String) -> Result<Option<SkillItem>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, source_id, name, author, description, tags, repo_url, detail_url, updated_at, compatible_tools, install_strategy, archive_url, stars, category \
             FROM skill_cache WHERE id = ?1",
        )
        .map_err(|e| e.to_string())?;
    let result = stmt.query_row(params![id], |row| map_skill_cache_row(row));
    match result {
        Ok(item) => Ok(Some(item)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub fn get_skill(db: State<DbState>, id: String) -> Result<Option<SkillItem>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    get_skill_inner(&conn, id)
}

// ─── SKILL.md content ──────────────────────────────────────────────────────

/// Extract (owner, repo) from a GitHub URL like https://github.com/owner/repo
fn extract_github_owner_repo(url: &str) -> Option<(String, String)> {
    let path = url
        .strip_prefix("https://github.com/")
        .or_else(|| url.strip_prefix("http://github.com/"))?;
    let path = path.strip_suffix('/').unwrap_or(path);
    let path = path.strip_suffix(".git").unwrap_or(path);
    let mut parts = path.splitn(3, '/');
    let owner = parts.next()?.to_string();
    let repo = parts.next()?.to_string();
    if owner.is_empty() || repo.is_empty() {
        return None;
    }
    Some((owner, repo))
}

/// Fetch SKILL.md from GitHub raw, caching to skill_cache.skill_md.
pub async fn fetch_skill_md_inner(
    db: Arc<Mutex<Connection>>,
    id: String,
) -> Result<Option<String>, String> {
    // 1. Check cache
    {
        let conn = db.lock().map_err(|e| e.to_string())?;
        let cached: Option<String> = conn
            .query_row(
                "SELECT skill_md FROM skill_cache WHERE id = ?1 AND skill_md IS NOT NULL",
                params![id],
                |row| row.get(0),
            )
            .ok();
        if let Some(content) = cached {
            return Ok(Some(content));
        }
    }

    // 2. Get repo_url
    let repo_url: Option<String> = {
        let conn = db.lock().map_err(|e| e.to_string())?;
        conn.query_row(
            "SELECT repo_url FROM skill_cache WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )
        .ok()
        .flatten()
    };

    let repo_url = match repo_url {
        Some(url) => url,
        None => return Ok(None),
    };

    // 3. Extract owner/repo — only GitHub repos are supported
    let (owner, repo) = match extract_github_owner_repo(&repo_url) {
        Some(pair) => pair,
        None => return Ok(None),
    };

    // 4. Fetch SKILL.md and README.md concurrently, prefer SKILL.md
    let client = reqwest::Client::builder()
        .user_agent("skills-plus-plus/0.1")
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;

    async fn try_fetch_file(
        client: &reqwest::Client,
        owner: &str,
        repo: &str,
        filename: &str,
    ) -> Option<(Option<FrontmatterMeta>, String)> {
        for branch in ["main", "master"] {
            let url = format!(
                "https://raw.githubusercontent.com/{owner}/{repo}/refs/heads/{branch}/{filename}"
            );
            match client.get(&url).send().await {
                Ok(r) if r.status().is_success() => {
                    let text = r.text().await.ok()?;
                    let (meta, body) = parse_frontmatter_and_strip(&text);
                    return Some((meta, body));
                }
                _ => continue,
            }
        }
        None
    }

    let (sk, rm) = tokio::join!(
        try_fetch_file(&client, &owner, &repo, "SKILL.md"),
        try_fetch_file(&client, &owner, &repo, "README.md"),
    );
    let content = sk.or(rm);

    match content {
        Some((meta, body)) => {
            // 5. Cache — persist both SKILL.md body and frontmatter metadata.
            let tags_json = meta
                .as_ref()
                .map(|m| serde_json::to_string(&m.tags).unwrap_or_default())
                .unwrap_or_default();
            let conn = db.lock().map_err(|e| e.to_string())?;
            conn.execute(
                "UPDATE skill_cache SET \
                 author = CASE WHEN ?1 != '' THEN ?1 ELSE author END, \
                 description = CASE WHEN ?2 != '' THEN ?2 ELSE description END, \
                 tags = CASE WHEN ?3 != '' THEN ?3 ELSE tags END, \
                 updated_at = CASE WHEN ?4 != '' THEN ?4 ELSE updated_at END, \
                 skill_md = ?5 \
                 WHERE id = ?6",
                params![
                    meta.as_ref().and_then(|m| m.author.as_deref()).unwrap_or(""),
                    meta.as_ref().and_then(|m| m.description.as_deref()).unwrap_or(""),
                    tags_json,
                    meta.as_ref().and_then(|m| m.updated_at.as_deref()).unwrap_or(""),
                    body,
                    id,
                ],
            )
            .map_err(|e| e.to_string())?;
            Ok(Some(body))
        }
        None => Ok(None),
    }
}

#[tauri::command]
pub async fn fetch_skill_md(
    db: State<'_, DbState>,
    id: String,
) -> Result<Option<String>, String> {
    fetch_skill_md_inner(std::sync::Arc::clone(&db.0), id).await
}

// ─── Online fallback search (skills.sh /api/search) ────────────────────────

/// 调 skills.sh 在线搜索，本地缓存为空时的兜底。
/// 搜索结果同时持久化到 skill_cache。
pub async fn search_online_inner(
    db: Arc<Mutex<Connection>>,
    query: String,
    limit: Option<u32>,
) -> Result<Vec<SkillItem>, String> {
    let items = crate::services::adapters::skills_sh_search::search(&query, limit).await?;

    // 逐条 UPSERT，避免与 store_skills 的 DELETE-then-reinsert 冲突。
    {
        let conn = db.lock().map_err(|e| e.to_string())?;
        persist_online_results(&conn, &items)?;
    }

    Ok(items)
}

/// 将在线搜索结果逐条 INSERT OR REPLACE 到 skill_cache。
fn persist_online_results(conn: &Connection, items: &[SkillItem]) -> Result<(), String> {
    for item in items {
        let tags = serde_json::to_string(&item.tags).unwrap_or_default();
        let tools = serde_json::to_string(&item.compatible_tools).unwrap_or_default();
        let strategy = item.install_strategy.map(|s| s.as_str().to_string());
        conn.execute(
            "INSERT OR REPLACE INTO skill_cache \
             (id, source_id, name, author, description, tags, repo_url, detail_url, updated_at, compatible_tools, install_strategy, archive_url, stars, category) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                item.id, item.source_id, item.name, item.author, item.description,
                tags, item.repo_url, item.detail_url, item.updated_at, tools,
                strategy, item.archive_url, item.stars, item.category,
            ],
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub async fn search_online(
    db: State<'_, DbState>,
    query: String,
    limit: Option<u32>,
) -> Result<Vec<SkillItem>, String> {
    search_online_inner(std::sync::Arc::clone(&db.0), query, limit).await
}