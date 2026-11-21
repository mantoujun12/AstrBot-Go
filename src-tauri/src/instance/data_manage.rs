//! Direct read/write access to AstrBot instance data (personas, cron jobs, config files).
//!
//! All operations are offline edits against stopped instances: AstrBot v4 keeps
//! personas and cron jobs in `data/data_v4.db` and caches them in memory, so
//! writes against a live instance would be ignored or clobbered on exit. The
//! table schemas evolve upstream, hence every read/write picks the columns
//! that actually exist instead of hard-coding the full v4 shape.

use std::fs;
use std::io::ErrorKind;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::time::{Duration, UNIX_EPOCH};

use rusqlite::types::{Value as SqlValue, ValueRef};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::error::ErrorKind as AppErrorKind;
use crate::error::{AppError, Result};
use crate::utils::fs_ops::{create_unique_sibling_file, replace_file_atomically};
use crate::utils::lock_check::ensure_target_not_locked;
use crate::utils::paths::get_instance_core_dir;
use crate::utils::validation::validate_instance_id;

const DB_FILE: &str = "data_v4.db";
/// SQLite can stay locked for a moment after a shutdown flush.
const BUSY_TIMEOUT: Duration = Duration::from_secs(5);

/// A persona row from the AstrBot `personas` table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstancePersona {
    pub persona_id: String,
    pub system_prompt: String,
    #[serde(default)]
    pub begin_dialogs: Vec<String>,
    /// `None` means all tools, an empty list disables tools.
    #[serde(default)]
    pub tools: Option<Vec<String>>,
    #[serde(default)]
    pub skills: Option<Vec<String>>,
    #[serde(default)]
    pub custom_error_message: Option<String>,
    #[serde(default)]
    pub folder_id: Option<String>,
    #[serde(default)]
    pub sort_order: i64,
}

/// A cron job row from the AstrBot `cron_jobs` table. The `status`/`last_*`
/// fields are runtime observability and are never written back by the launcher.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceCronJob {
    pub job_id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default = "default_job_type")]
    pub job_type: String,
    #[serde(default)]
    pub cron_expression: Option<String>,
    #[serde(default)]
    pub timezone: Option<String>,
    #[serde(default)]
    pub payload: JsonValue,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub persistent: bool,
    #[serde(default)]
    pub run_once: bool,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub last_run_at: Option<String>,
    #[serde(default)]
    pub next_run_time: Option<String>,
    #[serde(default)]
    pub last_error: Option<String>,
}

fn default_job_type() -> String {
    "basic".to_string()
}

/// A config file entry, `name` is relative to the instance data dir.
#[derive(Debug, Clone, Serialize)]
pub struct InstanceConfigFile {
    pub name: String,
    pub size: u64,
    pub modified_ms: Option<i64>,
}

/// A persona folder row from the AstrBot `persona_folders` table.
#[derive(Debug, Clone, Serialize)]
pub struct InstancePersonaFolder {
    pub folder_id: String,
    pub name: String,
    pub parent_id: Option<String>,
    pub description: Option<String>,
    pub sort_order: i64,
}

/// A knowledge base summary: `knowledge_bases` row joined with the on-disk
/// size of `knowledge_base/<kb_id>`.
#[derive(Debug, Clone, Serialize)]
pub struct InstanceKnowledgeBase {
    pub kb_id: String,
    pub name: String,
    pub description: Option<String>,
    pub emoji: Option<String>,
    pub doc_count: i64,
    pub chunk_count: i64,
    pub size_bytes: u64,
}

/// Cross-instance copy request; cron jobs are always inserted with a fresh
/// `job_id`, personas and config files follow the `overwrite` flag.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CopyInstanceDataRequest {
    pub source_instance_id: String,
    pub target_instance_ids: Vec<String>,
    pub persona_ids: Vec<String>,
    pub cron_job_ids: Vec<String>,
    pub config_files: Vec<String>,
    pub overwrite: bool,
}

const PERSONA_WRITE_COLUMNS: [&str; 8] = [
    "persona_id",
    "system_prompt",
    "begin_dialogs",
    "tools",
    "skills",
    "custom_error_message",
    "folder_id",
    "sort_order",
];

const CRON_WRITE_COLUMNS: [&str; 10] = [
    "job_id",
    "name",
    "description",
    "job_type",
    "cron_expression",
    "timezone",
    "payload",
    "enabled",
    "persistent",
    "run_once",
];

pub(crate) fn get_instance_data_dir(instance_id: &str) -> Result<PathBuf> {
    validate_instance_id(instance_id)?;
    let dir = get_instance_core_dir(instance_id).join("data");
    if !dir.is_dir() {
        return Err(AppError::io(
            "实例数据目录不存在，可以尝试先运行一次实例再管理数据。",
        ));
    }
    Ok(dir)
}

fn db_err(error: rusqlite::Error) -> AppError {
    AppError::io(format!("数据库操作失败：{error}"))
}

fn open_db(data_dir: &Path) -> Result<Connection> {
    let path = data_dir.join(DB_FILE);
    if !path.is_file() {
        return Err(AppError::config(
            "未找到 data_v4.db，实例数据管理仅支持 AstrBot v4 及以上版本。",
        ));
    }
    let conn = Connection::open(&path).map_err(db_err)?;
    conn.busy_timeout(BUSY_TIMEOUT).map_err(db_err)?;
    Ok(conn)
}

/// Reject writes when another process still holds the database file.
fn ensure_db_not_locked(data_dir: &Path) -> Result<()> {
    let db = data_dir.join(DB_FILE);
    ensure_target_not_locked(std::slice::from_ref(&db))
}

fn table_exists(conn: &Connection, table: &str) -> Result<bool> {
    let mut stmt = conn
        .prepare("SELECT name FROM sqlite_master WHERE type = 'table' AND name = ?1")
        .map_err(db_err)?;
    stmt.exists(rusqlite::params![table]).map_err(db_err)
}

fn ensure_table(conn: &Connection, table: &str) -> Result<()> {
    if table_exists(conn, table)? {
        Ok(())
    } else {
        Err(AppError::config(format!(
            "数据库缺少 {table} 表，请确认实例为 AstrBot v4 及以上版本。"
        )))
    }
}

fn table_columns(conn: &Connection, table: &str) -> Result<Vec<String>> {
    let mut stmt = conn
        .prepare(&format!("PRAGMA table_info({table})"))
        .map_err(db_err)?;
    let columns = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(db_err)?
        .collect::<std::result::Result<Vec<String>, _>>()
        .map_err(db_err)?;
    Ok(columns)
}

// === Row decoding ===========================================================

fn cell_text(row: &rusqlite::Row<'_>, column: Option<usize>) -> Option<String> {
    match column.and_then(|idx| row.get_ref(idx).ok()) {
        Some(ValueRef::Text(bytes)) => Some(String::from_utf8_lossy(bytes).into_owned()),
        _ => None,
    }
}

fn cell_i64(row: &rusqlite::Row<'_>, column: Option<usize>, default: i64) -> i64 {
    match column.and_then(|idx| row.get_ref(idx).ok()) {
        Some(ValueRef::Integer(value)) => value,
        _ => default,
    }
}

fn cell_bool(row: &rusqlite::Row<'_>, column: Option<usize>, default: bool) -> bool {
    cell_i64(row, column, i64::from(default)) != 0
}

fn cell_json_string_list(row: &rusqlite::Row<'_>, column: Option<usize>) -> Option<Vec<String>> {
    let text = cell_text(row, column)?;
    let parsed: JsonValue = serde_json::from_str(&text).ok()?;
    match parsed {
        JsonValue::Array(items) => Some(
            items
                .into_iter()
                .filter_map(|item| item.as_str().map(str::to_string))
                .collect(),
        ),
        _ => None,
    }
}

fn cell_json_object(row: &rusqlite::Row<'_>, column: Option<usize>) -> JsonValue {
    match cell_text(row, column).and_then(|text| serde_json::from_str::<JsonValue>(&text).ok()) {
        Some(value @ JsonValue::Object(_)) => value,
        _ => JsonValue::Object(serde_json::Map::new()),
    }
}

// === Personas ===============================================================

pub(crate) fn list_personas(data_dir: &Path) -> Result<Vec<InstancePersona>> {
    let conn = open_db(data_dir)?;
    let mut stmt = conn
        .prepare("SELECT * FROM personas ORDER BY persona_id")
        .map_err(db_err)?;
    let columns: Vec<String> = stmt.column_names().map(|name| name.to_string()).collect();
    let mut rows = stmt.query([]).map_err(db_err)?;
    let mut personas = Vec::new();
    while let Some(row) = rows.next().map_err(db_err)? {
        match persona_from_row(row, &columns) {
            Some(persona) => personas.push(persona),
            None => log::warn!("Skipped a persona row without persona_id"),
        }
    }
    Ok(personas)
}

fn persona_from_row(row: &rusqlite::Row<'_>, columns: &[String]) -> Option<InstancePersona> {
    let position = |name: &str| columns.iter().position(|column| column == name);
    Some(InstancePersona {
        persona_id: cell_text(row, position("persona_id"))?,
        system_prompt: cell_text(row, position("system_prompt")).unwrap_or_default(),
        begin_dialogs: cell_json_string_list(row, position("begin_dialogs")).unwrap_or_default(),
        tools: cell_json_string_list(row, position("tools")),
        skills: cell_json_string_list(row, position("skills")),
        custom_error_message: cell_text(row, position("custom_error_message")),
        folder_id: cell_text(row, position("folder_id")),
        sort_order: cell_i64(row, position("sort_order"), 0),
    })
}

pub(crate) fn save_personas(
    data_dir: &Path,
    upserts: &[InstancePersona],
    deletes: &[String],
) -> Result<()> {
    ensure_db_not_locked(data_dir)?;
    for persona in upserts {
        if persona.persona_id.trim().is_empty() {
            return Err(AppError::config("persona_id 不能为空。"));
        }
    }
    ensure_unique_ids(
        upserts.iter().map(|persona| persona.persona_id.as_str()),
        "persona_id",
    )?;

    let mut conn = open_db(data_dir)?;
    ensure_table(&conn, "personas")?;
    let columns = table_columns(&conn, "personas")?;
    let tx = conn.transaction().map_err(db_err)?;
    delete_rows(&tx, "personas", "persona_id", deletes)?;
    write_persona_rows(&tx, upserts, &columns)?;
    tx.commit().map_err(db_err)
}

fn write_persona_rows(
    conn: &Connection,
    upserts: &[InstancePersona],
    table_columns: &[String],
) -> Result<()> {
    let columns = available_columns(&PERSONA_WRITE_COLUMNS, table_columns);
    ensure_required_columns(&columns, &["persona_id", "system_prompt"])?;
    let sql = upsert_sql("personas", &columns, "persona_id");
    for persona in upserts {
        let values: Vec<SqlValue> = columns
            .iter()
            .map(|column| persona_cell(persona, column))
            .collect::<std::result::Result<_, _>>()?;
        conn.execute(&sql, rusqlite::params_from_iter(values))
            .map_err(db_err)?;
    }
    Ok(())
}

fn persona_cell(persona: &InstancePersona, column: &str) -> Result<SqlValue> {
    Ok(match column {
        "persona_id" => SqlValue::Text(persona.persona_id.clone()),
        "system_prompt" => SqlValue::Text(persona.system_prompt.clone()),
        "begin_dialogs" => json_cell(&persona.begin_dialogs)?,
        "tools" => json_opt_cell(&persona.tools)?,
        "skills" => json_opt_cell(&persona.skills)?,
        "custom_error_message" => opt_text_cell(&persona.custom_error_message),
        "folder_id" => opt_text_cell(&persona.folder_id),
        "sort_order" => SqlValue::Integer(persona.sort_order),
        _ => SqlValue::Null,
    })
}

// === Cron jobs ==============================================================

pub(crate) fn list_cron_jobs(data_dir: &Path) -> Result<Vec<InstanceCronJob>> {
    let conn = open_db(data_dir)?;
    let mut stmt = conn
        .prepare("SELECT * FROM cron_jobs ORDER BY id")
        .map_err(db_err)?;
    let columns: Vec<String> = stmt.column_names().map(|name| name.to_string()).collect();
    let mut rows = stmt.query([]).map_err(db_err)?;
    let mut jobs = Vec::new();
    while let Some(row) = rows.next().map_err(db_err)? {
        match cron_from_row(row, &columns) {
            Some(job) => jobs.push(job),
            None => log::warn!("Skipped a cron job row without job_id"),
        }
    }
    Ok(jobs)
}

fn cron_from_row(row: &rusqlite::Row<'_>, columns: &[String]) -> Option<InstanceCronJob> {
    let position = |name: &str| columns.iter().position(|column| column == name);
    Some(InstanceCronJob {
        job_id: cell_text(row, position("job_id"))?,
        name: cell_text(row, position("name")).unwrap_or_default(),
        description: cell_text(row, position("description")),
        job_type: cell_text(row, position("job_type")).unwrap_or_else(default_job_type),
        cron_expression: cell_text(row, position("cron_expression")),
        timezone: cell_text(row, position("timezone")),
        payload: cell_json_object(row, position("payload")),
        enabled: cell_bool(row, position("enabled"), true),
        persistent: cell_bool(row, position("persistent"), true),
        run_once: cell_bool(row, position("run_once"), false),
        status: cell_text(row, position("status")),
        last_run_at: cell_text(row, position("last_run_at")),
        next_run_time: cell_text(row, position("next_run_time")),
        last_error: cell_text(row, position("last_error")),
    })
}

pub(crate) fn save_cron_jobs(
    data_dir: &Path,
    upserts: &[InstanceCronJob],
    deletes: &[String],
) -> Result<()> {
    ensure_db_not_locked(data_dir)?;
    validate_cron_jobs(upserts)?;

    let mut conn = open_db(data_dir)?;
    ensure_table(&conn, "cron_jobs")?;
    let columns = table_columns(&conn, "cron_jobs")?;
    let tx = conn.transaction().map_err(db_err)?;
    delete_rows(&tx, "cron_jobs", "job_id", deletes)?;
    write_cron_rows(&tx, upserts, &columns)?;
    tx.commit().map_err(db_err)
}

fn validate_cron_jobs(jobs: &[InstanceCronJob]) -> Result<()> {
    for job in jobs {
        if job.job_id.trim().is_empty() {
            return Err(AppError::config("job_id 不能为空。"));
        }
        if job.name.trim().is_empty() {
            return Err(AppError::config("任务名不能为空。"));
        }
        if !matches!(job.job_type.as_str(), "basic" | "active_agent") {
            return Err(AppError::config(format!(
                "未知的任务类型：{}",
                job.job_type
            )));
        }
        if !matches!(job.payload, JsonValue::Object(_)) {
            return Err(AppError::config("payload 必须是 JSON 对象。"));
        }
    }
    ensure_unique_ids(jobs.iter().map(|job| job.job_id.as_str()), "job_id")
}

fn write_cron_rows(
    conn: &Connection,
    upserts: &[InstanceCronJob],
    table_columns: &[String],
) -> Result<()> {
    let columns = available_columns(&CRON_WRITE_COLUMNS, table_columns);
    ensure_required_columns(&columns, &["job_id", "name"])?;
    let sql = upsert_sql("cron_jobs", &columns, "job_id");
    for job in upserts {
        let values: Vec<SqlValue> = columns
            .iter()
            .map(|column| cron_cell(job, column))
            .collect::<std::result::Result<_, _>>()?;
        conn.execute(&sql, rusqlite::params_from_iter(values))
            .map_err(db_err)?;
    }
    Ok(())
}

fn cron_cell(job: &InstanceCronJob, column: &str) -> Result<SqlValue> {
    Ok(match column {
        "job_id" => SqlValue::Text(job.job_id.clone()),
        "name" => SqlValue::Text(job.name.clone()),
        "description" => opt_text_cell(&job.description),
        "job_type" => SqlValue::Text(job.job_type.clone()),
        "cron_expression" => opt_text_cell(&job.cron_expression),
        "timezone" => opt_text_cell(&job.timezone),
        "payload" => json_cell(&job.payload)?,
        "enabled" => SqlValue::Integer(i64::from(job.enabled)),
        "persistent" => SqlValue::Integer(i64::from(job.persistent)),
        "run_once" => SqlValue::Integer(i64::from(job.run_once)),
        _ => SqlValue::Null,
    })
}

// === Shared SQL helpers =====================================================

fn available_columns<'a>(wanted: &[&'a str], table_columns: &[String]) -> Vec<&'a str> {
    wanted
        .iter()
        .copied()
        .filter(|column| table_columns.iter().any(|existing| existing == column))
        .collect()
}

fn ensure_required_columns(columns: &[&str], required: &[&str]) -> Result<()> {
    for name in required {
        if !columns.contains(name) {
            return Err(AppError::config(format!(
                "数据库表缺少列 {name}，实例数据结构与当前版本不兼容。"
            )));
        }
    }
    Ok(())
}

/// Build an upsert statement over the available columns, keyed by `conflict_key`.
fn upsert_sql(table: &str, columns: &[&str], conflict_key: &str) -> String {
    let placeholders: Vec<String> = (1..=columns.len()).map(|idx| format!("?{idx}")).collect();
    let updates: Vec<String> = columns
        .iter()
        .filter(|column| **column != conflict_key)
        .map(|column| format!("{column}=excluded.{column}"))
        .collect();
    format!(
        "INSERT INTO {table} ({columns}) VALUES ({values}) ON CONFLICT({key}) DO UPDATE SET {updates}",
        columns = columns.join(", "),
        values = placeholders.join(", "),
        key = conflict_key,
        updates = updates.join(", ")
    )
}

fn delete_rows(conn: &Connection, table: &str, key: &str, ids: &[String]) -> Result<()> {
    for id in ids {
        if id.trim().is_empty() {
            return Err(AppError::config(format!("要删除的 {key} 不能为空。")));
        }
        conn.execute(
            &format!("DELETE FROM {table} WHERE {key} = ?1"),
            rusqlite::params![id],
        )
        .map_err(db_err)?;
    }
    Ok(())
}

fn ensure_unique_ids<'a>(ids: impl Iterator<Item = &'a str>, label: &str) -> Result<()> {
    let mut seen = std::collections::HashSet::new();
    for id in ids {
        if !seen.insert(id) {
            return Err(AppError::config(format!("存在重复的 {label}：{id}")));
        }
    }
    Ok(())
}

fn json_cell<T: Serialize>(value: &T) -> Result<SqlValue> {
    serde_json::to_string(value)
        .map(SqlValue::Text)
        .map_err(|error| AppError::config(format!("JSON 序列化失败：{error}")))
}

fn json_opt_cell<T: Serialize>(value: &Option<T>) -> Result<SqlValue> {
    match value {
        None => Ok(SqlValue::Null),
        Some(value) => json_cell(value),
    }
}

fn opt_text_cell(value: &Option<String>) -> SqlValue {
    match value {
        None => SqlValue::Null,
        Some(text) => SqlValue::Text(text.clone()),
    }
}

// === Persona folders ========================================================

/// Older AstrBot builds have no `persona_folders` table; an empty list keeps
/// the folder selector usable without failing the whole dialog.
pub(crate) fn list_persona_folders(data_dir: &Path) -> Result<Vec<InstancePersonaFolder>> {
    let conn = open_db(data_dir)?;
    if !table_exists(&conn, "persona_folders")? {
        return Ok(Vec::new());
    }
    let mut stmt = conn
        .prepare("SELECT * FROM persona_folders ORDER BY sort_order, name")
        .map_err(db_err)?;
    let columns: Vec<String> = stmt.column_names().map(|name| name.to_string()).collect();
    let mut rows = stmt.query([]).map_err(db_err)?;
    let mut folders = Vec::new();
    while let Some(row) = rows.next().map_err(db_err)? {
        let position = |name: &str| columns.iter().position(|column| column == name);
        let Some(folder_id) = cell_text(row, position("folder_id")) else {
            log::warn!("Skipped a persona folder row without folder_id");
            continue;
        };
        folders.push(InstancePersonaFolder {
            folder_id,
            name: cell_text(row, position("name")).unwrap_or_default(),
            parent_id: cell_text(row, position("parent_id")),
            description: cell_text(row, position("description")),
            sort_order: cell_i64(row, position("sort_order"), 0),
        });
    }
    Ok(folders)
}

// === Knowledge bases ========================================================

fn knowledge_base_root(data_dir: &Path) -> PathBuf {
    data_dir.join("knowledge_base")
}

fn dir_size(dir: &Path) -> u64 {
    walkdir::WalkDir::new(dir)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| entry.metadata().ok())
        .filter(|metadata| metadata.is_file())
        .map(|metadata| metadata.len())
        .sum()
}

pub(crate) fn list_knowledge_bases(data_dir: &Path) -> Result<Vec<InstanceKnowledgeBase>> {
    let kb_root = knowledge_base_root(data_dir);
    let db = kb_root.join("kb.db");
    if !db.is_file() {
        return Ok(Vec::new());
    }
    let conn = Connection::open(&db).map_err(db_err)?;
    conn.busy_timeout(BUSY_TIMEOUT).map_err(db_err)?;
    ensure_table(&conn, "knowledge_bases")?;
    let mut stmt = conn
        .prepare(
            "SELECT kb_id, kb_name, description, emoji, doc_count, chunk_count
             FROM knowledge_bases
             ORDER BY kb_name",
        )
        .map_err(db_err)?;
    let mut bases = stmt
        .query_map([], |row| {
            Ok(InstanceKnowledgeBase {
                kb_id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                emoji: row.get(3)?,
                doc_count: row.get(4)?,
                chunk_count: row.get(5)?,
                size_bytes: 0,
            })
        })
        .map_err(db_err)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(db_err)?;
    drop(stmt);
    for base in &mut bases {
        base.size_bytes = dir_size(&kb_root.join(&base.kb_id));
    }
    Ok(bases)
}

/// Delete a knowledge base completely: index rows first (a dangling row is
/// worse than a leftover directory), then the directory itself.
pub(crate) fn delete_knowledge_base(data_dir: &Path, kb_id: &str) -> Result<()> {
    let valid_id = !kb_id.trim().is_empty()
        && !kb_id.contains(['/', '\\', ':', '\0'])
        && kb_id != "."
        && kb_id != "..";
    if !valid_id {
        return Err(AppError::config(format!("知识库 id 不合法：{kb_id}")));
    }
    let kb_root = knowledge_base_root(data_dir);
    let db = kb_root.join("kb.db");
    if db.is_file() {
        ensure_target_not_locked(std::slice::from_ref(&db))?;
        let conn = Connection::open(&db).map_err(db_err)?;
        conn.busy_timeout(BUSY_TIMEOUT).map_err(db_err)?;
        let mut conn = conn;
        let tx = conn.transaction().map_err(db_err)?;
        for table in ["kb_documents", "kb_media", "knowledge_bases"] {
            if table_exists(&tx, table)? {
                tx.execute(
                    &format!("DELETE FROM {table} WHERE kb_id = ?1"),
                    rusqlite::params![kb_id],
                )
                .map_err(db_err)?;
            }
        }
        tx.commit().map_err(db_err)?;
    }
    let dir = kb_root.join(kb_id);
    if dir.is_dir() {
        fs::remove_dir_all(&dir)
            .map_err(|error| AppError::io(format!("删除知识库目录失败：{error}")))?;
    }
    Ok(())
}

// === Config files ===========================================================

pub(crate) fn list_config_files(data_dir: &Path) -> Result<Vec<InstanceConfigFile>> {
    let mut files = Vec::new();
    for known in ["cmd_config.json", "mcp_server.json"] {
        let path = data_dir.join(known);
        if path.is_file() {
            files.push(config_file_entry(known, &path));
        }
    }

    let config_dir = data_dir.join("config");
    let entries = match fs::read_dir(&config_dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(files),
        Err(error) => return Err(AppError::io(format!("读取配置目录失败：{error}"))),
    };
    for entry in entries {
        let entry = entry.map_err(|error| AppError::io(format!("读取配置目录失败：{error}")))?;
        let file_name = entry.file_name().to_string_lossy().into_owned();
        if entry.path().is_file() && is_config_file_name(&file_name) {
            let name = format!("config/{file_name}");
            files.push(config_file_entry(&name, &entry.path()));
        }
    }
    files.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(files)
}

fn is_config_file_name(file_name: &str) -> bool {
    matches!(
        file_name.rsplit_once('.'),
        Some((_, "json" | "yaml" | "yml"))
    )
}

fn config_file_entry(name: &str, path: &Path) -> InstanceConfigFile {
    let metadata = fs::metadata(path).ok();
    InstanceConfigFile {
        name: name.to_string(),
        size: metadata.as_ref().map_or(0, |meta| meta.len()),
        modified_ms: metadata
            .as_ref()
            .and_then(|meta| meta.modified().ok())
            .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
            .map(|duration| duration.as_millis() as i64),
    }
}

pub(crate) fn read_config_file(data_dir: &Path, name: &str) -> Result<String> {
    validate_config_file_name(name)?;
    let path = data_dir.join(name);
    if !path.is_file() {
        // `missing_file` is a structured marker the frontend uses to tell a
        // not-yet-created file (AstrBot creates some on first run) apart from
        // a genuine read failure.
        return Err(AppError::new(
            AppErrorKind::Config,
            std::collections::HashMap::from([
                ("detail".to_string(), format!("配置文件不存在：{name}")),
                ("missing_file".to_string(), name.to_string()),
            ]),
        ));
    }
    fs::read_to_string(&path).map_err(|error| AppError::io(format!("读取配置文件失败：{error}")))
}

pub(crate) fn write_config_file(data_dir: &Path, name: &str, content: &str) -> Result<()> {
    validate_config_file_name(name)?;
    validate_config_content(name, content)?;
    let dest = data_dir.join(name);
    if dest.is_file() {
        ensure_target_not_locked(std::slice::from_ref(&dest))?;
    }
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| AppError::io(format!("创建配置目录失败：{error}")))?;
    }

    let (temp, file) = create_unique_sibling_file(&dest, "part")?;
    let write_result = {
        let mut writer = std::io::BufWriter::new(&file);
        writer
            .write_all(content.as_bytes())
            .and_then(|_| writer.flush())
    };
    drop(file);
    write_result.map_err(|error| {
        let _ = fs::remove_file(&temp);
        AppError::io(format!("写入配置文件失败：{error}"))
    })?;
    replace_file_atomically(&temp, &dest)
}

/// Only the launcher-known config paths are addressable: the two known
/// top-level config files and single files inside `data/config/`. Arbitrary
/// paths are rejected so a crafted name can never escape the data dir.
fn validate_config_file_name(name: &str) -> Result<()> {
    let valid = name == "cmd_config.json"
        || name == "mcp_server.json"
        || (name.starts_with("config/")
            && !name["config/".len()..].contains(['/', '\\', ':', '\0'])
            && !name.contains("..")
            && matches!(name.rsplit_once('.'), Some((_, "json" | "yaml" | "yml"))));
    if valid {
        Ok(())
    } else {
        Err(AppError::config(format!("配置文件名不合法：{name}")))
    }
}

fn validate_config_content(name: &str, content: &str) -> Result<()> {
    let extension = name.rsplit_once('.').map(|(_, extension)| extension);
    match extension {
        Some("json") => serde_json::from_str::<JsonValue>(content)
            .map_err(|error| AppError::config(format!("JSON 解析失败：{error}")))
            .map(|_| ()),
        Some("yaml" | "yml") => serde_yaml_ng::from_str::<serde_yaml_ng::Value>(content)
            .map_err(|error| AppError::config(format!("YAML 解析失败：{error}")))
            .map(|_| ()),
        _ => Ok(()),
    }
}

// === Cross-instance copy ====================================================

pub(crate) fn copy_instance_data(request: &CopyInstanceDataRequest) -> Result<()> {
    if request
        .target_instance_ids
        .iter()
        .any(|target| target == &request.source_instance_id)
    {
        return Err(AppError::config("目标实例不能包含源实例。"));
    }

    let source_dir = get_instance_data_dir(&request.source_instance_id)?;
    let personas = selected_personas(&source_dir, &request.persona_ids)?;
    let cron_jobs = selected_cron_jobs(&source_dir, &request.cron_job_ids)?;
    let config_contents = read_selected_config_files(&source_dir, &request.config_files)?;

    for target_id in &request.target_instance_ids {
        let target_dir = get_instance_data_dir(target_id)?;
        copy_to_target(
            &target_dir,
            &personas,
            &cron_jobs,
            &config_contents,
            request.overwrite,
        )?;
    }
    Ok(())
}

fn selected_personas(data_dir: &Path, persona_ids: &[String]) -> Result<Vec<InstancePersona>> {
    if persona_ids.is_empty() {
        return Ok(Vec::new());
    }
    let all = list_personas(data_dir)?;
    let mut selected = Vec::new();
    for persona_id in persona_ids {
        let persona = all
            .iter()
            .find(|persona| &persona.persona_id == persona_id)
            .ok_or_else(|| AppError::config(format!("源实例不存在人格：{persona_id}")))?;
        selected.push(persona.clone());
    }
    Ok(selected)
}

fn selected_cron_jobs(data_dir: &Path, job_ids: &[String]) -> Result<Vec<InstanceCronJob>> {
    if job_ids.is_empty() {
        return Ok(Vec::new());
    }
    let all = list_cron_jobs(data_dir)?;
    let mut selected = Vec::new();
    for job_id in job_ids {
        let job = all
            .iter()
            .find(|job| &job.job_id == job_id)
            .ok_or_else(|| AppError::config(format!("源实例不存在定时任务：{job_id}")))?;
        selected.push(job.clone());
    }
    Ok(selected)
}

fn read_selected_config_files(data_dir: &Path, names: &[String]) -> Result<Vec<(String, String)>> {
    let mut contents = Vec::new();
    for name in names {
        contents.push((name.clone(), read_config_file(data_dir, name)?));
    }
    Ok(contents)
}

fn copy_to_target(
    target_dir: &Path,
    personas: &[InstancePersona],
    cron_jobs: &[InstanceCronJob],
    config_contents: &[(String, String)],
    overwrite: bool,
) -> Result<()> {
    if !personas.is_empty() || !cron_jobs.is_empty() {
        ensure_db_not_locked(target_dir)?;
        let mut conn = open_db(target_dir)?;
        ensure_table(&conn, "personas")?;
        ensure_table(&conn, "cron_jobs")?;
        let persona_columns = table_columns(&conn, "personas")?;
        let cron_columns = table_columns(&conn, "cron_jobs")?;
        let tx = conn.transaction().map_err(db_err)?;
        copy_personas(&tx, personas, &persona_columns, overwrite)?;
        copy_cron_jobs(&tx, cron_jobs, &cron_columns)?;
        tx.commit().map_err(db_err)?;
    }
    for (name, content) in config_contents {
        if !overwrite && target_dir.join(name).is_file() {
            continue;
        }
        write_config_file(target_dir, name, content)?;
    }
    Ok(())
}

fn copy_personas(
    conn: &Connection,
    personas: &[InstancePersona],
    columns: &[String],
    overwrite: bool,
) -> Result<()> {
    for persona in personas {
        if !overwrite && row_exists(conn, "personas", "persona_id", &persona.persona_id)? {
            continue;
        }
        write_persona_rows(conn, std::slice::from_ref(persona), columns)?;
    }
    Ok(())
}

/// Cron job copies always get a fresh `job_id`: a copied job must not alias
/// the source row, and copying onto a target twice would otherwise collide.
fn copy_cron_jobs(conn: &Connection, jobs: &[InstanceCronJob], columns: &[String]) -> Result<()> {
    let mut copies = Vec::new();
    for job in jobs {
        let mut copy = job.clone();
        copy.job_id = uuid::Uuid::new_v4().to_string();
        copies.push(copy);
    }
    write_cron_rows(conn, &copies, columns)
}

fn row_exists(conn: &Connection, table: &str, key: &str, value: &str) -> Result<bool> {
    let mut stmt = conn
        .prepare(&format!("SELECT 1 FROM {table} WHERE {key} = ?1"))
        .map_err(db_err)?;
    stmt.exists(rusqlite::params![value]).map_err(db_err)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(tag: &str) -> Result<Self> {
            let dir = std::env::temp_dir()
                .join(format!("astrbot-go-data-{tag}-{}", uuid::Uuid::new_v4()));
            fs::create_dir_all(&dir).map_err(|error| AppError::io(error.to_string()))?;
            Ok(Self(dir))
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn open_test_db(path: &Path) -> Result<Connection> {
        let conn = Connection::open(path.join(DB_FILE)).map_err(db_err)?;
        conn.execute_batch(
            "CREATE TABLE personas (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                persona_id TEXT NOT NULL UNIQUE,
                system_prompt TEXT NOT NULL,
                begin_dialogs TEXT,
                tools TEXT,
                skills TEXT,
                custom_error_message TEXT,
                folder_id TEXT,
                sort_order INTEGER NOT NULL DEFAULT 0
            );
            CREATE TABLE cron_jobs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                job_id TEXT NOT NULL UNIQUE,
                name TEXT NOT NULL,
                description TEXT,
                job_type TEXT NOT NULL,
                cron_expression TEXT,
                timezone TEXT,
                payload TEXT,
                enabled INTEGER NOT NULL DEFAULT 1,
                persistent INTEGER NOT NULL DEFAULT 1,
                run_once INTEGER NOT NULL DEFAULT 0,
                status TEXT,
                last_run_at TEXT,
                next_run_time TEXT,
                last_error TEXT
            );",
        )
        .map_err(db_err)?;
        Ok(conn)
    }

    fn persona(persona_id: &str, prompt: &str) -> InstancePersona {
        InstancePersona {
            persona_id: persona_id.to_string(),
            system_prompt: prompt.to_string(),
            begin_dialogs: vec!["你好".to_string()],
            tools: None,
            skills: Some(Vec::new()),
            custom_error_message: None,
            folder_id: None,
            sort_order: 0,
        }
    }

    fn cron_job(job_id: &str, name: &str) -> InstanceCronJob {
        InstanceCronJob {
            job_id: job_id.to_string(),
            name: name.to_string(),
            description: None,
            job_type: "basic".to_string(),
            cron_expression: Some("0 9 * * *".to_string()),
            timezone: None,
            payload: serde_json::json!({"prompt": "hi"}),
            enabled: true,
            persistent: true,
            run_once: false,
            status: None,
            last_run_at: None,
            next_run_time: None,
            last_error: None,
        }
    }

    #[test]
    fn save_personas_is_idempotent_and_supports_delete() -> Result<()> {
        let dir = TempDir::new("personas")?;
        open_test_db(dir.path())?;

        save_personas(
            dir.path(),
            &[persona("p1", "one"), persona("p2", "two")],
            &[],
        )?;

        // Re-running the same save leaves a single row per persona. The lock
        // check must not see a dangling handle, so assertions open their own
        // short-lived connection instead of holding one across saves.
        save_personas(
            dir.path(),
            &[persona("p1", "one-edited"), persona("p2", "two")],
            &[],
        )?;
        {
            let conn = Connection::open(dir.path().join(DB_FILE)).map_err(db_err)?;
            let mut stmt = conn
                .prepare("SELECT persona_id, system_prompt FROM personas ORDER BY persona_id")
                .map_err(db_err)?;
            let rows = stmt
                .query_map([], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })
                .map_err(db_err)?
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(db_err)?;
            // One row per persona: the repeat save updated in place instead of
            // duplicating rows.
            assert_eq!(
                rows,
                vec![
                    ("p1".to_string(), "one-edited".to_string()),
                    ("p2".to_string(), "two".to_string())
                ]
            );
        }
        assert_eq!(list_personas(dir.path())?.len(), 2);

        // A delete followed by an upsert of the same id wins for the upsert.
        save_personas(dir.path(), &[persona("p1", "three")], &["p1".to_string()])?;
        let personas = list_personas(dir.path())?;
        assert_eq!(personas.len(), 2);
        let edited = personas
            .iter()
            .find(|persona| persona.persona_id == "p1")
            .ok_or_else(|| AppError::config("p1 缺失"))?;
        assert_eq!(edited.system_prompt, "three");

        // A delete without a matching upsert removes the row.
        save_personas(dir.path(), &[], &["p1".to_string()])?;
        assert_eq!(list_personas(dir.path())?.len(), 1);
        Ok(())
    }

    #[test]
    fn personas_survive_an_older_table_schema() -> Result<()> {
        let dir = TempDir::new("old-schema")?;
        let conn = Connection::open(dir.path().join(DB_FILE)).map_err(db_err)?;
        conn.execute_batch(
            "CREATE TABLE personas (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                persona_id TEXT NOT NULL UNIQUE,
                system_prompt TEXT NOT NULL,
                begin_dialogs TEXT,
                tools TEXT
            );",
        )
        .map_err(db_err)?;
        conn.execute(
            "INSERT INTO personas (persona_id, system_prompt, begin_dialogs, tools) VALUES ('old', 'keep', '[\"hi\"]', NULL)",
            [],
        )
        .map_err(db_err)?;
        drop(conn);

        let personas = list_personas(dir.path())?;
        assert_eq!(personas.len(), 1);
        assert_eq!(personas[0].persona_id, "old");
        assert_eq!(personas[0].begin_dialogs, vec!["hi".to_string()]);

        // The missing columns are simply not written; the rest round-trips.
        save_personas(dir.path(), &[persona("new", "added")], &[])?;
        let personas = list_personas(dir.path())?;
        assert_eq!(personas.len(), 2);
        assert!(personas.iter().any(|persona| persona.persona_id == "new"));
        Ok(())
    }

    #[test]
    fn save_cron_jobs_round_trips_and_preserves_runtime_columns() -> Result<()> {
        let dir = TempDir::new("cron")?;
        open_test_db(dir.path())?;
        let mut job = cron_job("job-1", "daily");
        job.status = Some("scheduled".to_string());

        save_cron_jobs(dir.path(), &[job], &[])?;
        let jobs = list_cron_jobs(dir.path())?;
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].name, "daily");
        // The status in the request payload is ignored: the launcher never
        // writes runtime columns.
        assert_eq!(jobs[0].status, None);

        // The launcher upsert does not touch runtime-only columns even when
        // they are absent from the request payload.
        {
            let conn = Connection::open(dir.path().join(DB_FILE)).map_err(db_err)?;
            conn.execute(
                "UPDATE cron_jobs SET status = 'running', last_error = 'boom' WHERE job_id = 'job-1'",
                [],
            )
            .map_err(db_err)?;
        }
        let mut edited = cron_job("job-1", "renamed");
        edited.status = None;
        save_cron_jobs(dir.path(), &[edited], &[])?;
        let jobs = list_cron_jobs(dir.path())?;
        assert_eq!(jobs[0].name, "renamed");
        assert_eq!(jobs[0].status, Some("running".to_string()));
        assert_eq!(jobs[0].last_error, Some("boom".to_string()));
        Ok(())
    }

    #[test]
    fn save_rejects_invalid_cron_payloads_and_duplicate_ids() -> Result<()> {
        let dir = TempDir::new("cron-invalid")?;
        open_test_db(dir.path())?;

        let mut bad_payload = cron_job("job-1", "bad");
        bad_payload.payload = serde_json::json!([1, 2]);
        assert!(save_cron_jobs(dir.path(), &[bad_payload], &[]).is_err());

        let mut bad_type = cron_job("job-2", "typed");
        bad_type.job_type = "unknown".to_string();
        assert!(save_cron_jobs(dir.path(), &[bad_type], &[]).is_err());

        assert!(list_cron_jobs(dir.path())?.is_empty());
        Ok(())
    }

    #[test]
    fn duplicate_upsert_ids_are_rejected() -> Result<()> {
        let dir = TempDir::new("dup-ids")?;
        open_test_db(dir.path())?;
        let error = save_personas(dir.path(), &[persona("p1", "a"), persona("p1", "b")], &[]);
        assert!(error.is_err());
        Ok(())
    }

    #[test]
    fn config_file_names_are_restricted() {
        for name in [
            "cmd_config.json",
            "mcp_server.json",
            "config/abconf_extra.yaml",
            "config/abconf_extra.yml",
            "config/astrbot_plugin_demo_config.json",
        ] {
            assert!(validate_config_file_name(name).is_ok(), "{name}");
        }
        for name in [
            "data_v4.db",
            "plugins.json",
            "../cmd_config.json",
            "config/../evil.json",
            "config/sub/x.json",
            "config/abconf_x.exe",
            "config\\abconf_x.yaml",
        ] {
            assert!(validate_config_file_name(name).is_err(), "{name}");
        }
    }

    #[test]
    fn config_writes_are_validated_and_atomic() -> Result<()> {
        let dir = TempDir::new("config")?;
        fs::create_dir_all(dir.path().join("config"))
            .map_err(|error| AppError::io(error.to_string()))?;
        let dest = dir.path().join("config/abconf_a.yaml");
        fs::write(&dest, "key: before\n").map_err(|error| AppError::io(error.to_string()))?;

        // Invalid YAML is rejected and keeps the previous content.
        let error = write_config_file(dir.path(), "config/abconf_a.yaml", "key: [broken");
        assert!(error.is_err());
        assert_eq!(
            fs::read_to_string(&dest).map_err(|error| AppError::io(error.to_string()))?,
            "key: before\n"
        );

        write_config_file(dir.path(), "config/abconf_a.yaml", "key: after\n")?;
        assert_eq!(
            fs::read_to_string(&dest).map_err(|error| AppError::io(error.to_string()))?,
            "key: after\n"
        );
        let files = list_config_files(dir.path())?;
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].name, "config/abconf_a.yaml");
        assert!(files[0].size > 0);
        Ok(())
    }

    #[test]
    fn copy_regenerates_job_ids_and_respects_overwrite() -> Result<()> {
        let source = TempDir::new("copy-source")?;
        let target = TempDir::new("copy-target")?;
        open_test_db(source.path())?;
        open_test_db(target.path())?;

        save_personas(source.path(), &[persona("p1", "one")], &[])?;
        save_cron_jobs(source.path(), &[cron_job("job-1", "daily")], &[])?;

        let request = CopyInstanceDataRequest {
            source_instance_id: "src".to_string(),
            target_instance_ids: vec!["tgt".to_string()],
            persona_ids: vec!["p1".to_string()],
            cron_job_ids: vec!["job-1".to_string()],
            config_files: Vec::new(),
            overwrite: false,
        };

        // The instance-id guard is bypassed by calling the copy helpers on the
        // resolved directories directly.
        let source_dir = source.path();
        let target_dir = target.path();
        let personas = selected_personas(source_dir, &request.persona_ids)?;
        let jobs = selected_cron_jobs(source_dir, &request.cron_job_ids)?;
        copy_to_target(target_dir, &personas, &jobs, &[], false)?;

        // Without overwrite, a second copy skips the existing persona but the
        // cron job is always copied again with a fresh job_id.
        copy_to_target(target_dir, &personas, &jobs, &[], false)?;
        let target_personas = list_personas(target_dir)?;
        assert_eq!(target_personas.len(), 1);
        let target_jobs = list_cron_jobs(target_dir)?;
        assert_eq!(target_jobs.len(), 2);
        assert_ne!(target_jobs[0].job_id, target_jobs[1].job_id);
        assert_ne!(target_jobs[0].job_id, "job-1");
        Ok(())
    }

    fn create_folder_schema(dir: &TempDir) -> Result<()> {
        let conn = Connection::open(dir.path().join(DB_FILE)).map_err(db_err)?;
        conn.execute_batch(
            "CREATE TABLE persona_folders (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                folder_id TEXT NOT NULL UNIQUE,
                name TEXT NOT NULL,
                parent_id TEXT,
                description TEXT,
                sort_order INTEGER NOT NULL DEFAULT 0
            );",
        )
        .map_err(db_err)?;
        Ok(())
    }

    #[test]
    fn persona_folders_round_trip_and_tolerate_missing_table() -> Result<()> {
        let dir = TempDir::new("folders")?;
        open_test_db(dir.path())?;
        // No persona_folders table yet: an empty list instead of an error.
        assert!(list_persona_folders(dir.path())?.is_empty());

        create_folder_schema(&dir)?;
        {
            let conn = Connection::open(dir.path().join(DB_FILE)).map_err(db_err)?;
            conn.execute(
                "INSERT INTO persona_folders (folder_id, name, parent_id, sort_order) VALUES ('f1', '助手', NULL, 0)",
                [],
            )
            .map_err(db_err)?;
        }

        let folders = list_persona_folders(dir.path())?;
        assert_eq!(folders.len(), 1);
        assert_eq!(folders[0].folder_id, "f1");
        assert_eq!(folders[0].name, "助手");
        Ok(())
    }

    fn create_kb_fixture(dir: &Path) -> Result<()> {
        let kb_root = dir.join("knowledge_base");
        let kb_dir = kb_root.join("kb-1");
        fs::create_dir_all(kb_dir.join("files"))
            .map_err(|error| AppError::io(error.to_string()))?;
        fs::write(kb_dir.join("files/doc.txt"), b"hello knowledge")
            .map_err(|error| AppError::io(error.to_string()))?;
        let conn = Connection::open(kb_root.join("kb.db")).map_err(db_err)?;
        conn.execute_batch(
            "CREATE TABLE knowledge_bases (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                kb_id VARCHAR(36) NOT NULL,
                kb_name VARCHAR(100) NOT NULL,
                description TEXT,
                emoji VARCHAR(10),
                doc_count INTEGER NOT NULL,
                chunk_count INTEGER NOT NULL
            );
            CREATE TABLE kb_documents (id INTEGER PRIMARY KEY, kb_id VARCHAR(36), doc_name TEXT);
            CREATE TABLE kb_media (id INTEGER PRIMARY KEY, kb_id VARCHAR(36));",
        )
        .map_err(db_err)?;
        conn.execute(
            "INSERT INTO knowledge_bases (kb_id, kb_name, description, emoji, doc_count, chunk_count)
             VALUES ('kb-1', '帮助文档', '测试', '📘', 1, 3)",
            [],
        )
        .map_err(db_err)?;
        conn.execute(
            "INSERT INTO kb_documents (kb_id, doc_name) VALUES ('kb-1', 'doc')",
            [],
        )
        .map_err(db_err)?;
        Ok(())
    }

    #[test]
    fn knowledge_bases_list_reports_disk_size_and_delete_cleans_everything() -> Result<()> {
        let dir = TempDir::new("kb")?;
        create_kb_fixture(dir.path())?;

        // No knowledge_base dir at all: an empty list instead of an error.
        let empty_dir = TempDir::new("kb-empty")?;
        assert!(list_knowledge_bases(empty_dir.path())?.is_empty());

        let bases = list_knowledge_bases(dir.path())?;
        assert_eq!(bases.len(), 1);
        assert_eq!(bases[0].name, "帮助文档");
        assert_eq!(bases[0].emoji, Some("📘".to_string()));
        assert_eq!(bases[0].doc_count, 1);
        assert_eq!(bases[0].size_bytes, "hello knowledge".len() as u64);

        delete_knowledge_base(dir.path(), "kb-1")?;
        assert!(!dir.path().join("knowledge_base/kb-1").exists());
        let bases = list_knowledge_bases(dir.path())?;
        assert!(bases.is_empty());

        // The row deletion covered every index table.
        let conn = Connection::open(dir.path().join("knowledge_base/kb.db")).map_err(db_err)?;
        let documents = conn
            .query_row("SELECT COUNT(*) FROM kb_documents", [], |row| {
                row.get::<_, i64>(0)
            })
            .map_err(db_err)?;
        assert_eq!(documents, 0);
        Ok(())
    }

    #[test]
    fn knowledge_base_ids_are_restricted() -> Result<()> {
        let dir = TempDir::new("kb-guard")?;
        create_kb_fixture(dir.path())?;
        for bad_id in ["../escape", "sub/dir", "", ".", "..", "a\\b"] {
            assert!(
                delete_knowledge_base(dir.path(), bad_id).is_err(),
                "{bad_id}"
            );
        }
        // The fixture is untouched by the rejected deletes.
        assert!(dir.path().join("knowledge_base/kb-1").exists());
        Ok(())
    }
}
