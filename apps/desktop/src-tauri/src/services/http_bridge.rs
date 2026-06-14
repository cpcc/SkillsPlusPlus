//! Lightweight HTTP bridge exposing existing Tauri commands as
//! `POST /invoke/:cmd` so the same backend logic can be reached from a
//! normal browser tab during development (no mocking).
//!
//! The bridge only binds to 127.0.0.1 and is therefore never exposed
//! to the network.

use crate::commands::{
    app as app_cmd, directory as dir_cmd, install as install_cmd, source as src_cmd,
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::post,
    Json, Router,
};
use rusqlite::Connection;
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use tower_http::cors::{Any, CorsLayer};

#[derive(Clone)]
struct BridgeState {
    db: Arc<Mutex<Connection>>,
    version: String,
    db_path: String,
}

// ─── Per-command argument structs (snake_case field names; we convert
//      the camelCase JSON body before deserializing). ────────────────────────

#[derive(Deserialize)]
struct AddDirectoryArgs {
    tool_name: String,
    path: String,
}

#[derive(Deserialize)]
struct ToggleArgs {
    id: String,
    enabled: bool,
}

#[derive(Deserialize)]
struct IdArgs {
    id: String,
}

#[derive(Deserialize)]
struct RefreshSourceArgs {
    source_id: String,
}

#[derive(Deserialize)]
struct PreviewInstallArgs {
    skill_name: String,
    repo_url: String,
    directory_id: String,
    strategy: Option<crate::models::InstallStrategy>,
}

#[derive(Deserialize)]
struct InstallSkillArgs {
    skill_id: Option<String>,
    skill_name: String,
    repo_url: String,
    directory_id: String,
    overwrite: Option<bool>,
    strategy: Option<crate::models::InstallStrategy>,
    archive_url: Option<String>,
}

#[derive(Deserialize)]
struct ReinstallSkillArgs {
    skill_id: Option<String>,
    skill_name: String,
    repo_url: String,
    directory_id: String,
    strategy: Option<crate::models::InstallStrategy>,
    archive_url: Option<String>,
}

#[derive(Deserialize)]
struct UninstallSkillArgs {
    skill_name: String,
    directory_id: String,
}

#[derive(Deserialize)]
struct CheckSkillUpdateArgs {
    skill_id: String,
}

#[derive(Deserialize)]
struct SearchOnlineArgs {
    query: String,
    limit: Option<u32>,
}

/// Convert all top-level keys of a JSON object from camelCase to snake_case
/// so the typed arg structs above deserialize cleanly from the JS body.
fn camel_to_snake_keys(v: Value) -> Value {
    match v {
        Value::Object(map) => {
            let mut out = serde_json::Map::new();
            for (k, val) in map.into_iter() {
                out.insert(camel_to_snake(&k), camel_to_snake_keys(val));
            }
            Value::Object(out)
        }
        Value::Array(arr) => Value::Array(arr.into_iter().map(camel_to_snake_keys).collect()),
        other => other,
    }
}

fn camel_to_snake(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 4);
    for (i, ch) in s.chars().enumerate() {
        if ch.is_ascii_uppercase() {
            if i > 0 {
                out.push('_');
            }
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push(ch);
        }
    }
    out
}

/// Lock the DB and run a sync inner function, returning JSON.
fn with_conn<T, F>(st: &BridgeState, f: F) -> Result<Value, String>
where
    T: serde::Serialize,
    F: FnOnce(&Connection) -> Result<T, String>,
{
    let conn = st.db.lock().map_err(|e| e.to_string())?;
    let result = f(&conn)?;
    Ok(serde_json::to_value(result).unwrap_or(Value::Null))
}

fn parse_args<T: serde::de::DeserializeOwned>(v: &Value) -> Result<T, String> {
    serde_json::from_value::<T>(v.clone()).map_err(|e| format!("invalid args: {e}"))
}

fn to_json<T: serde::Serialize>(v: T) -> Value {
    serde_json::to_value(v).unwrap_or(Value::Null)
}

async fn invoke_handler(
    State(st): State<BridgeState>,
    Path(cmd): Path<String>,
    body: Option<String>,
) -> impl IntoResponse {
    let raw = body.unwrap_or_else(|| "{}".to_string());
    let parsed: Value = match serde_json::from_str(&raw) {
        Ok(v) => v,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": format!("invalid JSON: {e}") })),
            );
        }
    };
    let args = camel_to_snake_keys(parsed);
    let empty = Value::Object(serde_json::Map::new());
    let args = if args.is_null() { empty } else { args };

    let result: Result<Value, String> = match cmd.as_str() {
        // ── App ────────────────────────────────────────────────────────
        "get_app_info" => with_conn(&st, |c| {
            app_cmd::get_app_info_inner(c, st.version.clone(), st.db_path.clone())
        }),

        // ── Directories ────────────────────────────────────────────────
        "scan_directories" => with_conn(&st, dir_cmd::scan_directories_inner),
        "list_directories" => with_conn(&st, dir_cmd::list_directories_inner),
        "add_directory" => match parse_args::<AddDirectoryArgs>(&args) {
            Ok(a) => with_conn(&st, |c| {
                dir_cmd::add_directory_inner(c, a.tool_name.clone(), a.path.clone())
            }),
            Err(e) => Err(e),
        },
        "toggle_directory" => match parse_args::<ToggleArgs>(&args) {
            Ok(a) => with_conn(&st, |c| {
                dir_cmd::toggle_directory_inner(c, a.id.clone(), a.enabled)
            }),
            Err(e) => Err(e),
        },
        "set_default_directory" => match parse_args::<IdArgs>(&args) {
            Ok(a) => with_conn(&st, |c| {
                dir_cmd::set_default_directory_inner(c, a.id.clone())
            }),
            Err(e) => Err(e),
        },
        "delete_directory" => match parse_args::<IdArgs>(&args) {
            Ok(a) => with_conn(&st, |c| dir_cmd::delete_directory_inner(c, a.id.clone())),
            Err(e) => Err(e),
        },

        // ── Sources ────────────────────────────────────────────────────
        "list_sources" => with_conn(&st, src_cmd::list_sources_inner),
        "toggle_source" => match parse_args::<ToggleArgs>(&args) {
            Ok(a) => with_conn(&st, |c| {
                src_cmd::toggle_source_inner(c, a.id.clone(), a.enabled)
            }),
            Err(e) => Err(e),
        },
        "list_skills" => with_conn(&st, src_cmd::list_skills_inner),
        "refresh_source" => match parse_args::<RefreshSourceArgs>(&args) {
            Ok(a) => {
                let db = Arc::clone(&st.db);
                src_cmd::refresh_source_inner(db, a.source_id.clone())
                    .await
                    .map(to_json)
            }
            Err(e) => Err(e),
        },
        "refresh_all_sources" => {
            let db = Arc::clone(&st.db);
            src_cmd::refresh_all_sources_inner(db).await.map(to_json)
        }
        "get_skill" => match parse_args::<IdArgs>(&args) {
            Ok(a) => with_conn(&st, |c| src_cmd::get_skill_inner(c, a.id.clone())),
            Err(e) => Err(e),
        },
        "search_online" => match parse_args::<SearchOnlineArgs>(&args) {
            Ok(a) => src_cmd::search_online_inner(a.query.clone(), a.limit).await.map(to_json),
            Err(e) => Err(e),
        },

        // ── Install ────────────────────────────────────────────────────
        "preview_install" => match parse_args::<PreviewInstallArgs>(&args) {
            Ok(a) => with_conn(&st, |c| {
                install_cmd::preview_install_inner(
                    c,
                    a.skill_name.clone(),
                    a.repo_url.clone(),
                    a.directory_id.clone(),
                    a.strategy,
                )
            }),
            Err(e) => Err(e),
        },
        "install_skill" => match parse_args::<InstallSkillArgs>(&args) {
            Ok(a) => {
                let db = Arc::clone(&st.db);
                install_cmd::install_skill_inner(
                    db,
                    a.skill_id.clone(),
                    a.skill_name.clone(),
                    a.repo_url.clone(),
                    a.directory_id.clone(),
                    a.overwrite.unwrap_or(false),
                    a.strategy,
                    a.archive_url,
                )
                .await
                .map(to_json)
            }
            Err(e) => Err(e),
        },
        "reinstall_skill" => match parse_args::<ReinstallSkillArgs>(&args) {
            Ok(a) => {
                let db = Arc::clone(&st.db);
                install_cmd::reinstall_skill_inner(
                    db,
                    a.skill_id.clone(),
                    a.skill_name.clone(),
                    a.repo_url.clone(),
                    a.directory_id.clone(),
                    a.strategy,
                    a.archive_url,
                )
                .await
                .map(to_json)
            }
            Err(e) => Err(e),
        },
        "uninstall_skill" => match parse_args::<UninstallSkillArgs>(&args) {
            Ok(a) => with_conn(&st, |c| {
                install_cmd::uninstall_skill_inner(
                    c,
                    a.skill_name.clone(),
                    a.directory_id.clone(),
                )
            }),
            Err(e) => Err(e),
        },
        "list_installed_skills" => {
            with_conn(&st, install_cmd::list_installed_skills_inner)
        }
        "refresh_installed_skills" => {
            with_conn(&st, install_cmd::refresh_installed_skills_inner)
        }
        "check_skill_update" => match parse_args::<CheckSkillUpdateArgs>(&args) {
            Ok(a) => with_conn(&st, |c| {
                install_cmd::check_skill_update_inner(c, a.skill_id.clone())
            }),
            Err(e) => Err(e),
        },
        "list_install_tasks" => {
            with_conn(&st, install_cmd::list_install_tasks_inner)
        }
        "check_git_available" => Ok(json!(install_cmd::check_git_available_inner())),
        "read_lockfile" => install_cmd::read_lockfile().map(to_json),
        "list_canonical_skills" => install_cmd::list_canonical_skills().map(to_json),

        other => Err(format!("unknown command: {other}")),
    };

    match result {
        Ok(v) => (StatusCode::OK, Json(v)),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e }))),
    }
}

/// Start the HTTP bridge on the given address (e.g. "127.0.0.1:3030").
pub async fn start(
    addr: &str,
    db: Arc<Mutex<Connection>>,
    version: String,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let listener = tokio::net::TcpListener::bind(addr).await?;
    let actual = listener.local_addr()?;
    log::info!("HTTP bridge listening on http://{actual}");
    serve(listener, db, version).await
}

/// Build the bridge router (shared between `start` and tests).
fn build_router(db: Arc<Mutex<Connection>>, version: String) -> Router {
    let db_path = match db.lock() {
        Ok(c) => c.path().map(|p| p.to_string()).unwrap_or_default(),
        Err(_) => String::new(),
    };
    let state = BridgeState { db, version, db_path };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/invoke/:cmd", post(invoke_handler))
        .layer(cors)
        .with_state(state)
}

/// Serve the bridge on an already-bound TCP listener.
async fn serve(
    listener: tokio::net::TcpListener,
    db: Arc<Mutex<Connection>>,
    version: String,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let app = build_router(db, version);
    axum::serve(listener, app).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repositories::db;

    async fn spawn_bridge() -> (std::net::SocketAddr, tokio::task::JoinHandle<()>) {
        let conn = Connection::open_in_memory().unwrap();
        db::migrate(&conn).unwrap();
        db::seed_sources(&conn).unwrap();
        let db_arc = Arc::new(Mutex::new(conn));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let handle = tokio::spawn(async move {
            let _ = serve(listener, db_arc, "0.0.0-test".to_string()).await;
        });
        (addr, handle)
    }

    async fn post_json(addr: &str, cmd: &str, body: &str) -> (StatusCode, String) {
        let client = reqwest::Client::new();
        let r = client
            .post(format!("http://{addr}/invoke/{cmd}"))
            .header("content-type", "application/json")
            .body(body.to_string())
            .send()
            .await
            .unwrap();
        (r.status(), r.text().await.unwrap())
    }

    #[tokio::test]
    async fn list_sources_returns_seeded_rows() {
        let (addr, _h) = spawn_bridge().await;
        let (status, body) = post_json(&addr.to_string(), "list_sources", "{}").await;
        assert_eq!(status, StatusCode::OK);
        assert!(body.contains("skills.sh"));
        assert!(body.contains("LobeHub"));
    }

    #[tokio::test]
    async fn list_skills_returns_array() {
        let (addr, _h) = spawn_bridge().await;
        let (status, body) = post_json(&addr.to_string(), "list_skills", "{}").await;
        assert_eq!(status, StatusCode::OK);
        // No cache → empty array, but still valid JSON.
        assert_eq!(body.trim(), "[]");
    }

    #[tokio::test]
    async fn toggle_source_persists() {
        let (addr, _h) = spawn_bridge().await;
        let (status, _body) = post_json(
            &addr.to_string(),
            "toggle_source",
            r#"{"id":"skills_sh","enabled":false}"#,
        )
        .await;
        assert_eq!(status, StatusCode::OK);

        // Verify it persisted via list_sources.
        let (status, body) = post_json(&addr.to_string(), "list_sources", "{}").await;
        assert_eq!(status, StatusCode::OK);
        let v: serde_json::Value = serde_json::from_str(&body).unwrap();
        let skills_sh = v
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["id"] == "skills_sh")
            .unwrap();
        assert_eq!(skills_sh["enabled"], false);
    }

    #[tokio::test]
    async fn toggle_source_rejects_missing_args() {
        let (addr, _h) = spawn_bridge().await;
        let (status, _body) = post_json(
            &addr.to_string(),
            "toggle_source",
            r#"{"sourceId":"skills_sh"}"#,
        )
        .await;
        // {id, enabled} required → 500 with invalid-args error.
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn unknown_command_returns_500() {
        let (addr, _h) = spawn_bridge().await;
        let (status, body) = post_json(&addr.to_string(), "does_not_exist", "{}").await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert!(body.contains("unknown command"));
    }
}
