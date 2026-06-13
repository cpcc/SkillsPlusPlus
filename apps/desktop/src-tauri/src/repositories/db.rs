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
