//! `app_settings` 表的通用 KV 读写。
//!
//! 表结构（在 `SCHEMA_V2` 中已建好）：
//! ```sql
//! CREATE TABLE app_settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
//! ```
//!
//! 所有应用级配置都走这里：镜像源列表、是否启用镜像、未来扩展的其它开关。
//! 复杂结构用 JSON 字符串存（`get_json` / `set_json`）。

use rusqlite::{params, Connection, Result as SqliteResult};
use serde::de::DeserializeOwned;
use serde::Serialize;

/// 读一个字符串配置。返回 `Ok(None)` 表示 key 不存在。
pub fn get_str(conn: &Connection, key: &str) -> SqliteResult<Option<String>> {
    let mut stmt = conn.prepare("SELECT value FROM app_settings WHERE key = ?1")?;
    let result = stmt.query_row(params![key], |row| row.get::<_, String>(0));
    match result {
        Ok(v) => Ok(Some(v)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e),
    }
}

/// 写一个字符串配置（UPSERT）。
pub fn set_str(conn: &Connection, key: &str, value: &str) -> SqliteResult<()> {
    conn.execute(
        "INSERT INTO app_settings (key, value) VALUES (?1, ?2) \
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

/// 删除一个配置（如果存在）。
pub fn delete(conn: &Connection, key: &str) -> SqliteResult<()> {
    conn.execute("DELETE FROM app_settings WHERE key = ?1", params![key])?;
    Ok(())
}

/// 读一个 JSON 配置并反序列化。返回 `Ok(None)` 表示 key 不存在。
/// 解析失败也返回 `Ok(None)` 并打 warn（避免脏数据让调用方崩溃）。
pub fn get_json<T: DeserializeOwned>(conn: &Connection, key: &str) -> SqliteResult<Option<T>> {
    match get_str(conn, key)? {
        None => Ok(None),
        Some(s) => match serde_json::from_str::<T>(&s) {
            Ok(v) => Ok(Some(v)),
            Err(e) => {
                log::warn!("settings: key {key} json parse failed: {e}");
                Ok(None)
            }
        },
    }
}

/// 把一个值序列化为 JSON 写入。
pub fn set_json<T: Serialize>(conn: &Connection, key: &str, value: &T) -> SqliteResult<()> {
    let s = serde_json::to_string(value)
        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
    set_str(conn, key, &s)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open_test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE app_settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);",
        )
        .unwrap();
        conn
    }

    #[test]
    fn get_str_returns_none_for_missing_key() {
        let conn = open_test_db();
        assert_eq!(get_str(&conn, "nope").unwrap(), None);
    }

    #[test]
    fn set_str_then_get_str_roundtrips() {
        let conn = open_test_db();
        set_str(&conn, "k1", "v1").unwrap();
        assert_eq!(get_str(&conn, "k1").unwrap().as_deref(), Some("v1"));
    }

    #[test]
    fn set_str_upserts_existing_key() {
        let conn = open_test_db();
        set_str(&conn, "k1", "v1").unwrap();
        set_str(&conn, "k1", "v2").unwrap();
        assert_eq!(get_str(&conn, "k1").unwrap().as_deref(), Some("v2"));
    }

    #[test]
    fn delete_removes_key() {
        let conn = open_test_db();
        set_str(&conn, "k1", "v1").unwrap();
        delete(&conn, "k1").unwrap();
        assert_eq!(get_str(&conn, "k1").unwrap(), None);
    }

    #[test]
    fn delete_missing_key_is_noop() {
        let conn = open_test_db();
        delete(&conn, "never").unwrap();
    }

    #[test]
    fn set_json_then_get_json_roundtrips() {
        let conn = open_test_db();
        let value = vec!["a".to_string(), "b".to_string(), "".to_string()];
        set_json(&conn, "list", &value).unwrap();
        let got: Option<Vec<String>> = get_json(&conn, "list").unwrap();
        assert_eq!(got.as_deref(), Some(&value[..]));
    }

    #[test]
    fn get_json_returns_none_for_missing_key() {
        let conn = open_test_db();
        let got: Option<Vec<String>> = get_json(&conn, "missing").unwrap();
        assert_eq!(got, None);
    }

    #[test]
    fn get_json_returns_none_on_parse_error() {
        let conn = open_test_db();
        set_str(&conn, "broken", "{not json").unwrap();
        let got: Option<Vec<String>> = get_json(&conn, "broken").unwrap();
        assert_eq!(got, None);
    }

    #[test]
    fn set_json_overwrites_existing_value() {
        let conn = open_test_db();
        set_json(&conn, "v", &vec![1u32, 2]).unwrap();
        set_json(&conn, "v", &vec![3u32, 4, 5]).unwrap();
        let got: Option<Vec<u32>> = get_json(&conn, "v").unwrap();
        assert_eq!(got.as_deref(), Some(&[3u32, 4, 5][..]));
    }
}
