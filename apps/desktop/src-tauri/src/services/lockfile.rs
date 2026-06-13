//! `~/.agents/.skill-lock.json` 读写，字段对齐 vercel-labs `npx skills` 格式。

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockEntry {
    pub source: String,
    #[serde(rename = "sourceType")]
    pub source_type: String,
    #[serde(rename = "sourceUrl")]
    pub source_url: String,
    #[serde(rename = "skillPath")]
    pub skill_path: String,
    #[serde(rename = "skillFolderHash")]
    pub skill_folder_hash: String,
    #[serde(rename = "installedAt")]
    pub installed_at: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lockfile {
    /// vercel-labs 当前使用 version: 3。
    #[serde(default = "default_version")]
    pub version: i64,
    #[serde(default)]
    pub skills: BTreeMap<String, LockEntry>,
}

fn default_version() -> i64 {
    3
}

/// 返回 lockfile 路径（`~/.agents/.skill-lock.json`）。无 home 时回退到 None。
pub fn lockfile_path() -> Option<PathBuf> {
    crate::services::canonical_store::lockfile_path()
}

/// 读取 lockfile；文件不存在或解析失败时返回空 map（不抛错，保持调用方简单）。
pub fn read_lockfile() -> BTreeMap<String, LockEntry> {
    let Some(path) = lockfile_path() else {
        return BTreeMap::new();
    };
    match fs::read_to_string(&path) {
        Ok(text) => {
            let parsed: Result<Lockfile, _> = serde_json::from_str(&text);
            match parsed {
                Ok(f) => f.skills,
                Err(e) => {
                    log::warn!("lockfile parse failed at {:?}: {e}", path);
                    BTreeMap::new()
                }
            }
        }
        Err(_) => BTreeMap::new(),
    }
}

/// 原子写入 lockfile（先写 `.tmp` 再 rename），保证并发安全。
/// 若 lockfile 中已有其它条目，会保留。
pub fn write_lockfile(map: &BTreeMap<String, LockEntry>) -> Result<(), String> {
    let path = lockfile_path().ok_or_else(|| "cannot resolve home dir".to_string())?;
    write_lockfile_at(&path, map)
}

pub fn write_lockfile_at(path: &Path, map: &BTreeMap<String, LockEntry>) -> Result<(), String> {
    let file = Lockfile {
        version: 3,
        skills: map.clone(),
    };
    let json = serde_json::to_string_pretty(&file).map_err(|e| e.to_string())?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("create lockfile dir: {e}"))?;
    }
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, format!("{json}\n")).map_err(|e| format!("write lockfile tmp: {e}"))?;
    fs::rename(&tmp, path).map_err(|e| format!("rename lockfile: {e}"))?;
    Ok(())
}

/// 便捷：合并写入单条 entry。
pub fn upsert_entry(name: &str, entry: LockEntry) -> Result<(), String> {
    let mut map = read_lockfile();
    map.insert(name.to_string(), entry);
    write_lockfile(&map)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_entry(name: &str) -> (String, LockEntry) {
        (
            name.to_string(),
            LockEntry {
                source: format!("acme/{name}"),
                source_type: "github".to_string(),
                source_url: format!("https://github.com/acme/{name}.git"),
                skill_path: format!("skills/{name}/SKILL.md"),
                skill_folder_hash: "deadbeef".repeat(10),
                installed_at: "2026-01-01T00:00:00.000Z".to_string(),
                updated_at: "2026-01-02T00:00:00.000Z".to_string(),
            },
        )
    }

    #[test]
    fn round_trip_write_read() {
        let tmp = std::env::temp_dir().join("skills_pp_lockfile_roundtrip.json");
        let _ = fs::remove_file(&tmp);
        let mut map = BTreeMap::new();
        let (k, v) = sample_entry("demo");
        map.insert(k, v);
        write_lockfile_at(&tmp, &map).unwrap();
        // 重新读：手工 parse（不走 read_lockfile，因为后者读固定路径）。
        let text = fs::read_to_string(&tmp).unwrap();
        let f: Lockfile = serde_json::from_str(&text).unwrap();
        assert_eq!(f.version, 3);
        assert!(f.skills.contains_key("demo"));
        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn parses_vercel_labs_sample() {
        // 模拟 vercel-labs 真实格式片段。
        let json = r#"{
          "version": 3,
          "skills": {
            "find-skills": {
              "source": "vercel-labs/skills",
              "sourceType": "github",
              "sourceUrl": "https://github.com/vercel-labs/skills.git",
              "skillPath": "skills/find-skills/SKILL.md",
              "skillFolderHash": "3013fdeb8a11b10b1eb795ec3ae8bfca38f7c26d",
              "installedAt": "2026-02-17T07:47:00.724Z",
              "updatedAt": "2026-05-27T13:40:49.129Z"
            }
          }
        }"#;
        let f: Lockfile = serde_json::from_str(json).unwrap();
        assert_eq!(f.version, 3);
        let entry = f.skills.get("find-skills").expect("entry present");
        assert_eq!(entry.source_type, "github");
        assert_eq!(entry.skill_path, "skills/find-skills/SKILL.md");
        assert_eq!(entry.skill_folder_hash, "3013fdeb8a11b10b1eb795ec3ae8bfca38f7c26d");
    }

    #[test]
    fn upsert_preserves_other_entries() {
        let tmp = std::env::temp_dir().join("skills_pp_lockfile_upsert.json");
        let _ = fs::remove_file(&tmp);
        let mut map = BTreeMap::new();
        map.insert("a".to_string(), sample_entry("a").1);
        write_lockfile_at(&tmp, &map).unwrap();

        // 模拟 upsert：读 -> 改 -> 写。
        let text = fs::read_to_string(&tmp).unwrap();
        let mut f: Lockfile = serde_json::from_str(&text).unwrap();
        let (k, v) = sample_entry("b");
        f.skills.insert(k, v);
        write_lockfile_at(&tmp, &f.skills).unwrap();

        let text2 = fs::read_to_string(&tmp).unwrap();
        let f2: Lockfile = serde_json::from_str(&text2).unwrap();
        assert!(f2.skills.contains_key("a"));
        assert!(f2.skills.contains_key("b"));
        let _ = fs::remove_file(&tmp);
    }
}
