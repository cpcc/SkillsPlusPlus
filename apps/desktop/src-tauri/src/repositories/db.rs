use rusqlite::{Connection, Result as SqliteResult};
use std::path::PathBuf;

/// 当前 schema 版本号。新建库直接写 V2 全表；旧库按 user_version 升级。
const CURRENT_USER_VERSION: i64 = 2;

pub fn open(db_path: &PathBuf) -> SqliteResult<Connection> {
    let conn = Connection::open(db_path)?;
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
    Ok(conn)
}

pub fn migrate(conn: &Connection) -> SqliteResult<()> {
    let current: i64 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;

    if current == 0 {
        // 旧版 migrate() 从未设置 user_version，所以 user_version == 0 既可能是
        // 全新库，也可能是历史遗留的 V1 库。用「核心表是否已存在」来区分：
        // - 不存在 → 真·全新库，建 V2 全表。
        // - 已存在 → 历史遗留 V1 库，下面 ensure_v2_columns 会补齐列。
        let has_legacy_tables: i64 = conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='installed_skills'",
            [],
            |row| row.get(0),
        )?;
        if has_legacy_tables == 0 {
            conn.execute_batch(SCHEMA_V2)?;
        }
    }

    // 始终跑幂等的列补齐：保证无论 user_version 处于何种状态
    // （包括历史 bug 导致 user_version 已被写成 2 但列缺失的库）都能自愈。
    // add_column_if_missing 对已存在的列是 no-op。
    ensure_v2_columns(conn)?;

    conn.execute_batch(&format!("PRAGMA user_version = {CURRENT_USER_VERSION}"))?;
    Ok(())
}

/// 幂等地保证 V2 新列存在。对全新库（已由 SCHEMA_V2 建好）和已迁移库都是 no-op，
/// 对状态不一致的库（例如历史 bug 写错 user_version）能自愈。
fn ensure_v2_columns(conn: &Connection) -> SqliteResult<()> {
    add_column_if_missing(conn, "installed_skills", "install_strategy", "TEXT NOT NULL DEFAULT 'git'")?;
    add_column_if_missing(conn, "installed_skills", "content_hash", "TEXT")?;
    add_column_if_missing(conn, "installed_skills", "canonical_path", "TEXT")?;
    add_column_if_missing(conn, "installed_skills", "author", "TEXT")?;
    add_column_if_missing(conn, "installed_skills", "description", "TEXT")?;
    add_column_if_missing(conn, "skill_cache", "install_strategy", "TEXT")?;
    add_column_if_missing(conn, "skill_cache", "archive_url", "TEXT")?;
    add_column_if_missing(conn, "skill_cache", "stars", "INTEGER")?;
    Ok(())
}

/// 安全地为现有表新增列（幂等：列已存在时跳过）。
fn add_column_if_missing(
    conn: &Connection,
    table: &str,
    column: &str,
    definition: &str,
) -> SqliteResult<()> {
    let exists: i64 = conn.query_row(
        &format!(
            "SELECT COUNT(*) FROM pragma_table_info('{table}') WHERE name = ?1"
        ),
        rusqlite::params![column],
        |row| row.get(0),
    )?;
    if exists == 0 {
        conn.execute_batch(&format!(
            "ALTER TABLE {table} ADD COLUMN {column} {definition};"
        ))?;
    }
    Ok(())
}

pub fn seed_sources(conn: &Connection) -> SqliteResult<()> {
    let sources = &[
        ("skills_sh",  "skills.sh",    "https://skills.sh"),
        ("lobehub",    "LobeHub",      "https://lobehub.com/skills"),
        ("skillhub",   "SkillHub.cn",  "https://skillhub.cn"),
        ("clawhub",    "ClawHub.ai",   "https://clawhub.ai/skills"),
        ("skillsmp",   "SkillsMP",     "https://skillsmp.com"),
    ];
    for (id, name, url) in sources {
        conn.execute(
            "INSERT OR IGNORE INTO skill_sources (id, name, base_url, enabled) VALUES (?1, ?2, ?3, 1)",
            rusqlite::params![id, name, url],
        )?;
    }
    Ok(())
}

const SCHEMA_V2: &str = "
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
    cached_at       TEXT NOT NULL DEFAULT (datetime('now')),
    install_strategy TEXT,
    archive_url     TEXT,
    stars           INTEGER
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
    status       TEXT NOT NULL DEFAULT 'ok',
    install_strategy TEXT NOT NULL DEFAULT 'git',
    content_hash TEXT,
    canonical_path TEXT,
    author       TEXT,
    description  TEXT
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

#[cfg(test)]
mod tests {
    use super::*;

    /// 模拟历史遗留 V1 库（user_version 从未设置、表已存在但缺 V2 列），
    /// 验证 migrate() 能自愈补齐 install_strategy / stars 等列。
    #[test]
    fn migrate_self_heals_legacy_v1_db() {
        let conn = Connection::open_in_memory().unwrap();
        // 1) 手工建一个「旧 V1」schema：没有 user_version，没有 V2 列。
        conn.execute_batch(
            "CREATE TABLE skill_sources (id TEXT PRIMARY KEY, name TEXT, base_url TEXT, enabled INTEGER, created_at TEXT);
             CREATE TABLE skill_cache (id TEXT PRIMARY KEY, source_id TEXT, name TEXT, author TEXT, description TEXT, tags TEXT, repo_url TEXT, detail_url TEXT, updated_at TEXT, compatible_tools TEXT, cached_at TEXT);
             CREATE TABLE ai_tool_directories (id TEXT PRIMARY KEY, tool_name TEXT, path TEXT, is_default INTEGER, is_detected INTEGER, writable INTEGER, enabled INTEGER, skill_count INTEGER, created_at TEXT);
             CREATE TABLE installed_skills (id TEXT PRIMARY KEY, skill_id TEXT, name TEXT, tool_name TEXT, directory_id TEXT, source_id TEXT, repo_url TEXT, installed_at TEXT, status TEXT);
             CREATE TABLE install_tasks (id TEXT PRIMARY KEY, skill_id TEXT, skill_name TEXT, tool_name TEXT, directory_id TEXT, action TEXT, status TEXT, started_at TEXT, finished_at TEXT, error_message TEXT, created_at TEXT);
             CREATE TABLE app_settings (key TEXT PRIMARY KEY, value TEXT);",
        )
        .unwrap();
        // user_version 应为 0。
        let v: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap();
        assert_eq!(v, 0);

        // 2) 跑 migrate()。
        migrate(&conn).unwrap();

        // 3) 验证 V2 列已加上。
        let has_strategy: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('installed_skills') WHERE name='install_strategy'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(has_strategy, 1);
        let has_stars: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('skill_cache') WHERE name='stars'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(has_stars, 1);

        // 4) user_version 应升到 2。
        let v: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap();
        assert_eq!(v, CURRENT_USER_VERSION);

        // 5) 迁移后再跑一次 migrate() 应仍能正常完成（幂等）。
        migrate(&conn).unwrap();
    }

    /// 模拟「历史 bug 写错 user_version」的库：user_version 已是 2 但列缺失。
    /// ensure_v2_columns 必须能把它修好。
    #[test]
    fn migrate_self_heals_inconsistent_version() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE skill_sources (id TEXT PRIMARY KEY, name TEXT, base_url TEXT, enabled INTEGER, created_at TEXT);
             CREATE TABLE skill_cache (id TEXT PRIMARY KEY, source_id TEXT, name TEXT, author TEXT, description TEXT, tags TEXT, repo_url TEXT, detail_url TEXT, updated_at TEXT, compatible_tools TEXT, cached_at TEXT);
             CREATE TABLE ai_tool_directories (id TEXT PRIMARY KEY, tool_name TEXT, path TEXT, is_default INTEGER, is_detected INTEGER, writable INTEGER, enabled INTEGER, skill_count INTEGER, created_at TEXT);
             CREATE TABLE installed_skills (id TEXT PRIMARY KEY, skill_id TEXT, name TEXT, tool_name TEXT, directory_id TEXT, source_id TEXT, repo_url TEXT, installed_at TEXT, status TEXT);
             CREATE TABLE install_tasks (id TEXT PRIMARY KEY, skill_id TEXT, skill_name TEXT, tool_name TEXT, directory_id TEXT, action TEXT, status TEXT, started_at TEXT, finished_at TEXT, error_message TEXT, created_at TEXT);
             CREATE TABLE app_settings (key TEXT PRIMARY KEY, value TEXT);
             PRAGMA user_version = 2;",
        )
        .unwrap();

        migrate(&conn).unwrap();

        let has_strategy: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('installed_skills') WHERE name='install_strategy'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(has_strategy, 1);
    }

    /// 真·全新库：migrate() 应直接建出 V2 全表。
    #[test]
    fn migrate_fresh_db_builds_v2() {
        let conn = Connection::open_in_memory().unwrap();
        migrate(&conn).unwrap();
        let has_table: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='installed_skills'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(has_table, 1);
        let v: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap();
        assert_eq!(v, CURRENT_USER_VERSION);
    }
}
