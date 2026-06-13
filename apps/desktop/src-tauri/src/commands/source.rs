use crate::commands::app::DbState;
use crate::models::{SkillItem, SourceRow};
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

pub fn load_skills(conn: &Connection, source_ids: &[String]) -> Result<Vec<SkillItem>, String> {
    if source_ids.is_empty() {
        return Ok(vec![]);
    }
    let ph: String = (1..=source_ids.len()).map(|i| format!("?{i}")).collect::<Vec<_>>().join(", ");
    let sql = format!(
        "SELECT id, source_id, name, author, description, tags, repo_url, detail_url, updated_at, compatible_tools \
         FROM skill_cache WHERE source_id IN ({ph}) ORDER BY name"
    );
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let params_refs: Vec<&dyn rusqlite::ToSql> = source_ids.iter().map(|s| s as &dyn rusqlite::ToSql).collect();
    let rows = stmt
        .query_map(params_refs.as_slice(), |row| {
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

    let mut out = vec![];
    for row in rows {
        let (id, source_id, name, author, description, tags_s, repo_url, detail_url, updated_at, tools_s) =
            row.map_err(|e| e.to_string())?;
        out.push(SkillItem {
            id, name, author, description, source_id, repo_url, detail_url, updated_at,
            tags: serde_json::from_str(&tags_s).unwrap_or_default(),
            compatible_tools: serde_json::from_str(&tools_s).unwrap_or_default(),
            stars: None,
        });
    }
    Ok(out)
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
) -> Result<Vec<SkillItem>, String> {
    let registry = SourceRegistry::new();
    let adapter = registry
        .get_adapter(&source_id)
        .ok_or_else(|| format!("Unknown source: {source_id}"))?;

    let items = adapter.fetch().await?;

    {
        let conn = db.lock().map_err(|e| e.to_string())?;
        store_skills(&conn, &source_id, &items)?;
    }

    Ok(items)
}

#[tauri::command]
pub async fn refresh_source(
    db: State<'_, DbState>,
    source_id: String,
) -> Result<Vec<SkillItem>, String> {
    refresh_source_inner(std::sync::Arc::clone(&db.0), source_id).await
}

/// Refresh all enabled sources with stale cache.
pub async fn refresh_all_sources_inner(
    db: Arc<Mutex<Connection>>,
) -> Result<Vec<SkillItem>, String> {
    let source_ids: Vec<String> = {
        let conn = db.lock().map_err(|e| e.to_string())?;
        enabled_source_ids(&conn)?
    };

    let registry = SourceRegistry::new();

    for sid in &source_ids {
        let is_fresh = {
            let conn = db.lock().map_err(|e| e.to_string())?;
            cache_is_fresh(&conn, sid)
        };
        if is_fresh { continue; }

        if let Some(adapter) = registry.get_adapter(sid) {
            match adapter.fetch().await {
                Ok(items) => {
                    let conn = db.lock().map_err(|e| e.to_string())?;
                    if let Err(e) = store_skills(&conn, sid, &items) {
                        log::warn!("Failed to cache {sid}: {e}");
                    }
                }
                Err(e) => log::warn!("Failed to fetch {sid}: {e}"),
            }
        }
    }

    let conn = db.lock().map_err(|e| e.to_string())?;
    load_skills(&conn, &source_ids)
}

#[tauri::command]
pub async fn refresh_all_sources(db: State<'_, DbState>) -> Result<Vec<SkillItem>, String> {
    refresh_all_sources_inner(std::sync::Arc::clone(&db.0)).await
}

/// Get a single skill by ID.
pub fn get_skill_inner(conn: &Connection, id: String) -> Result<Option<SkillItem>, String> {
    let result = conn.query_row(
        "SELECT id, source_id, name, author, description, tags, repo_url, detail_url, updated_at, compatible_tools \
         FROM skill_cache WHERE id = ?1",
        params![id],
        |row| Ok((
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
        )),
    );
    match result {
        Ok((id, source_id, name, author, description, tags_s, repo_url, detail_url, updated_at, tools_s)) =>
            Ok(Some(SkillItem {
                id, name, author, description, source_id, repo_url, detail_url, updated_at,
                tags: serde_json::from_str(&tags_s).unwrap_or_default(),
                compatible_tools: serde_json::from_str(&tools_s).unwrap_or_default(),
                stars: None,
            })),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub fn get_skill(db: State<DbState>, id: String) -> Result<Option<SkillItem>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    get_skill_inner(&conn, id)
}
