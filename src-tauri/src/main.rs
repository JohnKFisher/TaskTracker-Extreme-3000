// Prevents additional console window on Windows in release — DO NOT REMOVE
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod version_manifest;

use keyring::Entry;
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_opener::OpenerExt;

const PRODUCT_NAME: &str = "TaskTracker Extreme 3000";
const GITHUB_URL: &str = "https://github.com/JohnKFisher/TaskTracker-Extreme-3000";
const APP_LICENSE: &str = "MIT";
const PRIMARY_PLATFORM: &str = "Windows";
const COPYRIGHT_TEXT: &str = "Copyright John Kenneth Fisher";

const TASKS_FILE: &str = "tasks.json";
const NOTES_FILE: &str = "notes.json";
const TICKET_SETTINGS_FILE: &str = "config.json";
const HIDDEN_TICKETS_FILE: &str = "hidden-tickets.json";
const WINDOW_STATE_FILE: &str = "window-state.json";
const LOCAL_SETTINGS_FILE: &str = "local-settings.json";
const SHARED_DATA_FILES: [&str; 4] = [
    TASKS_FILE,
    NOTES_FILE,
    TICKET_SETTINGS_FILE,
    HIDDEN_TICKETS_FILE,
];

const API_KEY_SERVICE: &str = "com.tasktracker.extreme3000";
const API_KEY_ACCOUNT: &str = "desk365-api-key";

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct AppError {
    code: String,
    message: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CommandResponse<T: Serialize> {
    success: bool,
    data: Option<T>,
    error: Option<AppError>,
}

impl<T: Serialize> CommandResponse<T> {
    fn ok(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    fn err(code: &str, message: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(AppError {
                code: code.to_string(),
                message: message.into(),
            }),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct LocalSettings {
    #[serde(default)]
    sync_folder: Option<String>,
    #[serde(default)]
    device_id: Option<String>,
    #[serde(default)]
    startup_legacy_import_done: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct TaskItem {
    id: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    notes: String,
    #[serde(default)]
    column: String,
    #[serde(default)]
    order: i64,
    #[serde(default)]
    created_at: Option<String>,
    #[serde(default)]
    updated_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct TaskTombstone {
    id: String,
    updated_at: String,
    #[serde(default)]
    updated_by: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct TaskDocument {
    #[serde(default = "default_schema_version")]
    schema_version: u32,
    #[serde(default)]
    revision: u64,
    #[serde(default)]
    updated_at: Option<String>,
    #[serde(default)]
    updated_by: Option<String>,
    #[serde(default)]
    tasks: Vec<TaskItem>,
    #[serde(default)]
    tombstones: Vec<TaskTombstone>,
}

impl Default for TaskDocument {
    fn default() -> Self {
        Self {
            schema_version: default_schema_version(),
            revision: 0,
            updated_at: None,
            updated_by: None,
            tasks: Vec::new(),
            tombstones: Vec::new(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct NotesDocument {
    #[serde(default = "default_schema_version")]
    schema_version: u32,
    #[serde(default)]
    revision: u64,
    #[serde(default)]
    updated_at: Option<String>,
    #[serde(default)]
    updated_by: Option<String>,
    #[serde(default)]
    content: String,
}

impl Default for NotesDocument {
    fn default() -> Self {
        Self {
            schema_version: default_schema_version(),
            revision: 0,
            updated_at: None,
            updated_by: None,
            content: String::new(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct HiddenTicketState {
    ticket_number: String,
    hidden: bool,
    updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct HiddenTicketsDocument {
    #[serde(default = "hidden_ticket_schema_version")]
    schema_version: u32,
    #[serde(default)]
    revision: u64,
    #[serde(default)]
    updated_at: Option<String>,
    #[serde(default)]
    updated_by: Option<String>,
    #[serde(default)]
    tickets: Vec<String>,
    #[serde(default)]
    states: Vec<HiddenTicketState>,
}

impl Default for HiddenTicketsDocument {
    fn default() -> Self {
        Self {
            schema_version: hidden_ticket_schema_version(),
            revision: 0,
            updated_at: None,
            updated_by: None,
            tickets: Vec::new(),
            states: Vec::new(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct TicketSettingsDocument {
    #[serde(default = "ticket_settings_schema_version")]
    schema_version: u32,
    #[serde(default)]
    revision: u64,
    #[serde(default)]
    updated_at: Option<String>,
    #[serde(default)]
    updated_by: Option<String>,
    #[serde(default)]
    desk365_domain: Option<String>,
}

impl Default for TicketSettingsDocument {
    fn default() -> Self {
        Self {
            schema_version: ticket_settings_schema_version(),
            revision: 0,
            updated_at: None,
            updated_by: None,
            desk365_domain: None,
        }
    }
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct TicketSettingsState {
    schema_version: u32,
    revision: u64,
    desk365_domain: Option<String>,
    has_api_key: bool,
    auth_error: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct StorageStatus {
    mode: String,
    configured_path: Option<String>,
    active_path: Option<String>,
    shared_data_available: bool,
    message: Option<String>,
    notice: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct AppMetadata {
    product_name: String,
    marketing_version: String,
    build_number: u64,
    license: String,
    github_url: String,
    primary_platform: String,
    copyright: String,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct TaskSaveResult {
    document: TaskDocument,
    merged: bool,
    conflict_ids: Vec<String>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct NotesSaveResult {
    document: NotesDocument,
    conflict: bool,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct HiddenTicketsSaveResult {
    document: HiddenTicketsDocument,
    merged: bool,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct SharedDataChangeEvent {
    files: Vec<String>,
}

#[derive(Debug)]
struct SharedDataWatcher {
    #[allow(dead_code)]
    watcher: RecommendedWatcher,
    #[allow(dead_code)]
    watched_dir: PathBuf,
}

#[derive(Debug)]
struct AppState {
    local_settings: Mutex<LocalSettings>,
    ticket_auth_error: Mutex<Option<String>>,
    shared_data_watcher: Mutex<Option<SharedDataWatcher>>,
}

fn default_schema_version() -> u32 {
    2
}

fn hidden_ticket_schema_version() -> u32 {
    2
}

fn ticket_settings_schema_version() -> u32 {
    3
}

trait CredentialStore {
    fn get_api_key(&self) -> Result<Option<String>, AppError>;
    fn set_api_key(&self, api_key: &str) -> Result<(), AppError>;
    fn clear_api_key(&self) -> Result<(), AppError>;
}

struct KeyringCredentialStore;

impl KeyringCredentialStore {
    fn entry(&self) -> Result<Entry, AppError> {
        Entry::new(API_KEY_SERVICE, API_KEY_ACCOUNT).map_err(|err| AppError {
            code: "credential_store_unavailable".to_string(),
            message: format!("Could not access the OS credential store: {err}"),
        })
    }
}

impl CredentialStore for KeyringCredentialStore {
    fn get_api_key(&self) -> Result<Option<String>, AppError> {
        match self.entry()?.get_password() {
            Ok(value) => Ok(Some(value)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(err) => Err(AppError {
                code: "credential_store_unavailable".to_string(),
                message: format!("Could not read the saved Desk365 API key: {err}"),
            }),
        }
    }

    fn set_api_key(&self, api_key: &str) -> Result<(), AppError> {
        self.entry()?.set_password(api_key).map_err(|err| AppError {
            code: "credential_store_unavailable".to_string(),
            message: format!("Could not save the Desk365 API key securely: {err}"),
        })
    }

    fn clear_api_key(&self) -> Result<(), AppError> {
        match self.entry()?.delete_credential() {
            Ok(_) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(err) => Err(AppError {
                code: "credential_store_unavailable".to_string(),
                message: format!("Could not clear the saved Desk365 API key: {err}"),
            }),
        }
    }
}

fn normalize_sync_folder(value: Option<String>) -> Option<String> {
    value.and_then(|entry| {
        let trimmed = entry.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(normalize_path_value(trimmed))
        }
    })
}

fn normalize_path_value(value: &str) -> String {
    let trimmed = value.trim();
    if let Some(stripped) = trimmed.strip_prefix("file:///") {
        #[cfg(target_os = "windows")]
        {
            return stripped.replace('/', "\\");
        }
        #[cfg(not(target_os = "windows"))]
        {
            return format!("/{}", stripped);
        }
    }
    if let Some(stripped) = trimmed.strip_prefix("file://") {
        #[cfg(not(target_os = "windows"))]
        {
            return format!("/{}", stripped.trim_start_matches('/'));
        }
    }
    trimmed.to_string()
}

fn generate_device_id() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    format!("device-{}-{millis}", std::process::id())
}

fn normalize_local_settings(mut settings: LocalSettings) -> LocalSettings {
    settings.sync_folder = normalize_sync_folder(settings.sync_folder);
    if settings
        .device_id
        .as_deref()
        .map(str::trim)
        .unwrap_or("")
        .is_empty()
    {
        settings.device_id = Some(generate_device_id());
    }
    settings
}

fn merge_missing_sync_folder(current: &LocalSettings, imported: &LocalSettings) -> LocalSettings {
    let mut merged = current.clone();
    if merged.sync_folder.is_none() {
        merged.sync_folder = normalize_sync_folder(imported.sync_folder.clone());
    }
    normalize_local_settings(merged)
}

fn default_task_timestamp(task: &TaskItem, fallback: &str) -> String {
    task.updated_at
        .clone()
        .or_else(|| task.created_at.clone())
        .unwrap_or_else(|| fallback.to_string())
}

fn current_iso_timestamp() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format_iso_timestamp(seconds)
}

fn compare_timestamps(a: Option<&str>, b: Option<&str>) -> Ordering {
    match (a, b) {
        (Some(left), Some(right)) => left.cmp(right),
        (Some(_), None) => Ordering::Greater,
        (None, Some(_)) => Ordering::Less,
        (None, None) => Ordering::Equal,
    }
}

fn next_revision(latest: u64, incoming: u64) -> u64 {
    latest.max(incoming).saturating_add(1)
}

fn local_app_data_dir(app: &AppHandle) -> Result<PathBuf, AppError> {
    let dir = app.path().app_data_dir().map_err(|err| AppError {
        code: "app_data_unavailable".to_string(),
        message: format!("Could not resolve the app data directory: {err}"),
    })?;

    fs::create_dir_all(&dir).map_err(|err| AppError {
        code: "app_data_unavailable".to_string(),
        message: format!(
            "Could not create the app data directory at {}: {err}",
            dir.display()
        ),
    })?;

    Ok(dir)
}

fn local_settings_path(app: &AppHandle) -> Result<PathBuf, AppError> {
    Ok(local_app_data_dir(app)?.join(LOCAL_SETTINGS_FILE))
}

fn window_state_path(app: &AppHandle) -> Result<PathBuf, AppError> {
    Ok(local_app_data_dir(app)?.join(WINDOW_STATE_FILE))
}

fn compute_storage_status_from_local_dir(
    local_dir: Result<PathBuf, AppError>,
    settings: &LocalSettings,
) -> StorageStatus {
    match settings.sync_folder.as_deref() {
        Some(folder) if !folder.trim().is_empty() => {
            let path = PathBuf::from(folder);
            if path.is_dir() {
                StorageStatus {
                    mode: "sync".to_string(),
                    configured_path: Some(folder.to_string()),
                    active_path: Some(path.to_string_lossy().to_string()),
                    shared_data_available: true,
                    message: None,
                    notice: None,
                }
            } else {
                StorageStatus {
                    mode: "syncUnavailable".to_string(),
                    configured_path: Some(folder.to_string()),
                    active_path: None,
                    shared_data_available: false,
                    message: Some(format!(
                        "Sync folder unavailable: {}. Tasks, notes, Desk365 settings, and hidden ticket state are unavailable until the folder is reachable. Local settings and window position still work.",
                        path.display()
                    )),
                    notice: None,
                }
            }
        }
        _ => match local_dir {
            Ok(dir) => StorageStatus {
                mode: "local".to_string(),
                configured_path: None,
                active_path: Some(dir.to_string_lossy().to_string()),
                shared_data_available: true,
                message: None,
                notice: None,
            },
            Err(err) => StorageStatus {
                mode: "localUnavailable".to_string(),
                configured_path: None,
                active_path: None,
                shared_data_available: false,
                message: Some(err.message),
                notice: None,
            },
        },
    }
}

fn compute_storage_status(settings: &LocalSettings, app: &AppHandle) -> StorageStatus {
    compute_storage_status_from_local_dir(local_app_data_dir(app), settings)
}

fn shared_data_dir(settings: &LocalSettings, app: &AppHandle) -> Result<PathBuf, AppError> {
    let status = compute_storage_status(settings, app);
    if status.shared_data_available {
        if let Some(active_path) = status.active_path {
            return Ok(PathBuf::from(active_path));
        }
    }

    Err(AppError {
        code: "sync_unavailable".to_string(),
        message: status.message.unwrap_or_else(|| {
            "Shared data is currently unavailable. Local settings still work.".to_string()
        }),
    })
}

fn shared_data_path(
    filename: &str,
    settings: &LocalSettings,
    app: &AppHandle,
) -> Result<PathBuf, AppError> {
    Ok(shared_data_dir(settings, app)?.join(filename))
}

fn read_text_file(path: &Path) -> Result<Option<String>, AppError> {
    match fs::read_to_string(path) {
        Ok(content) => Ok(Some(content)),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(AppError {
            code: "read_failed".to_string(),
            message: format!("Could not read {}: {err}", path.display()),
        }),
    }
}

fn temp_file_path(path: &Path) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let file_name = path
        .file_name()
        .and_then(|entry| entry.to_str())
        .unwrap_or("shared-data.json");
    path.with_file_name(format!(
        ".{file_name}.tmp-{}-{stamp}",
        std::process::id()
    ))
}

fn write_json_file<T: Serialize>(path: &Path, data: &T) -> Result<(), AppError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| AppError {
            code: "write_failed".to_string(),
            message: format!("Could not create {}: {err}", parent.display()),
        })?;
    }

    let content = serde_json::to_string_pretty(data).map_err(|err| AppError {
        code: "write_failed".to_string(),
        message: format!("Could not serialize {}: {err}", path.display()),
    })?;

    let temp_path = temp_file_path(path);
    fs::write(&temp_path, content).map_err(|err| AppError {
        code: "write_failed".to_string(),
        message: format!("Could not write {}: {err}", temp_path.display()),
    })?;

    #[cfg(target_os = "windows")]
    {
        if path.exists() {
            let backup_path = path.with_extension(format!(
                "bak-{}",
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis()
            ));
            fs::rename(path, &backup_path).map_err(|err| AppError {
                code: "write_failed".to_string(),
                message: format!("Could not prepare {} for replacement: {err}", path.display()),
            })?;

            if let Err(err) = fs::rename(&temp_path, path) {
                let _ = fs::rename(&backup_path, path);
                let _ = fs::remove_file(&temp_path);
                return Err(AppError {
                    code: "write_failed".to_string(),
                    message: format!("Could not replace {}: {err}", path.display()),
                });
            }

            let _ = fs::remove_file(&backup_path);
            return Ok(());
        }
    }

    fs::rename(&temp_path, path).map_err(|err| {
        let _ = fs::remove_file(&temp_path);
        AppError {
            code: "write_failed".to_string(),
            message: format!("Could not replace {}: {err}", path.display()),
        }
    })
}

fn read_or_default<T>(path: &Path) -> Result<T, AppError>
where
    T: for<'de> Deserialize<'de> + Default,
{
    match read_text_file(path)? {
        Some(content) => serde_json::from_str(&content).map_err(|err| AppError {
            code: "invalid_data".to_string(),
            message: format!("Could not parse {}: {err}", path.display()),
        }),
        None => Ok(T::default()),
    }
}

fn string_field(value: &Value, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| value.get(*key))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn parse_task_item_value(value: &Value, index: usize) -> Option<TaskItem> {
    let title = string_field(value, &["title"]).unwrap_or_else(|| "Untitled task".to_string());
    let id = string_field(value, &["id"]).unwrap_or_else(|| format!("legacy-task-{index}"));
    let column = string_field(value, &["column"]).unwrap_or_else(|| "todo".to_string());
    let notes = string_field(value, &["notes"]).unwrap_or_default();
    let order = value
        .get("order")
        .and_then(Value::as_i64)
        .unwrap_or(index as i64);
    let created_at = string_field(value, &["createdAt", "created_at"]);
    let updated_at = string_field(value, &["updatedAt", "updated_at"]);

    Some(TaskItem {
        id,
        title,
        notes,
        column,
        order,
        created_at,
        updated_at,
    })
}

fn parse_task_tombstone_value(value: &Value) -> Option<TaskTombstone> {
    Some(TaskTombstone {
        id: string_field(value, &["id"])?,
        updated_at: string_field(value, &["updatedAt", "updated_at"])?,
        updated_by: string_field(value, &["updatedBy", "updated_by"]),
    })
}

fn parse_task_document_content(path: &Path, content: &str) -> Result<TaskDocument, AppError> {
    if let Ok(document) = serde_json::from_str::<TaskDocument>(content) {
        return Ok(normalize_task_document(document));
    }

    let value: Value = serde_json::from_str(content).map_err(|err| AppError {
        code: "invalid_data".to_string(),
        message: format!("Could not parse {}: {err}", path.display()),
    })?;

    let mut document = TaskDocument::default();
    match &value {
        Value::Array(entries) => {
            document.tasks = entries
                .iter()
                .enumerate()
                .filter_map(|(index, task)| parse_task_item_value(task, index))
                .collect();
        }
        Value::Object(_) => {
            document.schema_version = value
                .get("schemaVersion")
                .and_then(Value::as_u64)
                .map(|version| version as u32)
                .unwrap_or(default_schema_version());
            document.revision = value.get("revision").and_then(Value::as_u64).unwrap_or(0);
            document.updated_at = string_field(&value, &["updatedAt", "updated_at"]);
            document.updated_by = string_field(&value, &["updatedBy", "updated_by"]);
            document.tasks = value
                .get("tasks")
                .and_then(Value::as_array)
                .map(|entries| {
                    entries
                        .iter()
                        .enumerate()
                        .filter_map(|(index, task)| parse_task_item_value(task, index))
                        .collect()
                })
                .unwrap_or_default();
            document.tombstones = value
                .get("tombstones")
                .and_then(Value::as_array)
                .map(|entries| {
                    entries
                        .iter()
                        .filter_map(parse_task_tombstone_value)
                        .collect()
                })
                .unwrap_or_default();
        }
        _ => {}
    }

    Ok(normalize_task_document(document))
}

fn normalize_task_document(mut document: TaskDocument) -> TaskDocument {
    document.schema_version = default_schema_version();
    let fallback = document
        .updated_at
        .clone()
        .unwrap_or_else(current_iso_timestamp);

    for task in &mut document.tasks {
        if task.created_at.as_deref().unwrap_or("").is_empty() {
            task.created_at = Some(default_task_timestamp(task, &fallback));
        }
        if task.updated_at.as_deref().unwrap_or("").is_empty() {
            task.updated_at = Some(default_task_timestamp(task, &fallback));
        }
    }

    for tombstone in &mut document.tombstones {
        if tombstone.updated_at.trim().is_empty() {
            tombstone.updated_at = fallback.clone();
        }
    }

    normalize_task_orders(&mut document.tasks);
    document
}

fn normalize_hidden_tickets_document(mut document: HiddenTicketsDocument) -> HiddenTicketsDocument {
    document.schema_version = hidden_ticket_schema_version();
    let fallback = document
        .updated_at
        .clone()
        .unwrap_or_else(current_iso_timestamp);

    if document.states.is_empty() && !document.tickets.is_empty() {
        document.states = document
            .tickets
            .iter()
            .map(|ticket_number| HiddenTicketState {
                ticket_number: ticket_number.clone(),
                hidden: true,
                updated_at: fallback.clone(),
            })
            .collect();
    }

    let mut states_by_ticket: BTreeMap<String, HiddenTicketState> = BTreeMap::new();
    for state in document.states {
        let updated_at = if state.updated_at.trim().is_empty() {
            fallback.clone()
        } else {
            state.updated_at
        };
        let candidate = HiddenTicketState {
            ticket_number: state.ticket_number.clone(),
            hidden: state.hidden,
            updated_at,
        };
        match states_by_ticket.get(&candidate.ticket_number) {
            Some(existing) if existing.updated_at > candidate.updated_at => {}
            _ => {
                states_by_ticket.insert(candidate.ticket_number.clone(), candidate);
            }
        }
    }

    document.states = states_by_ticket.into_values().collect();
    document.tickets = document
        .states
        .iter()
        .filter(|state| state.hidden)
        .map(|state| state.ticket_number.clone())
        .collect();
    document
}

fn normalize_ticket_settings_value(value: &Value) -> TicketSettingsDocument {
    TicketSettingsDocument {
        schema_version: ticket_settings_schema_version(),
        revision: value.get("revision").and_then(Value::as_u64).unwrap_or(0),
        updated_at: value
            .get("updatedAt")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        updated_by: value
            .get("updatedBy")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        desk365_domain: value
            .get("desk365Domain")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
    }
}

fn migrate_legacy_secret_value<S: CredentialStore>(
    value: &Value,
    store: &S,
) -> Result<Option<Value>, AppError> {
    let Some(api_key) = value.get("apiKey").and_then(Value::as_str) else {
        return Ok(None);
    };

    store.set_api_key(api_key)?;
    let mut normalized =
        serde_json::to_value(normalize_ticket_settings_value(value)).map_err(|err| AppError {
            code: "invalid_data".to_string(),
            message: format!("Could not normalize legacy Desk365 settings: {err}"),
        })?;

    if let Some(domain) = value.get("desk365Domain").and_then(Value::as_str) {
        normalized["desk365Domain"] = json!(domain);
    }

    Ok(Some(normalized))
}

fn migrate_legacy_ticket_secret_if_needed<S: CredentialStore>(
    settings: &LocalSettings,
    app: &AppHandle,
    store: &S,
) -> Result<bool, AppError> {
    let path = shared_data_path(TICKET_SETTINGS_FILE, settings, app)?;
    let Some(content) = read_text_file(&path)? else {
        return Ok(false);
    };

    let value: Value = serde_json::from_str(&content).map_err(|err| AppError {
        code: "invalid_data".to_string(),
        message: format!("Could not parse {}: {err}", path.display()),
    })?;

    if let Some(normalized) = migrate_legacy_secret_value(&value, store)? {
        write_json_file(&path, &normalized)?;
        return Ok(true);
    }

    Ok(false)
}

fn read_ticket_settings_document(
    settings: &LocalSettings,
    app: &AppHandle,
) -> Result<TicketSettingsDocument, AppError> {
    let path = shared_data_path(TICKET_SETTINGS_FILE, settings, app)?;
    match read_text_file(&path)? {
        Some(content) => {
            let value: Value = serde_json::from_str(&content).map_err(|err| AppError {
                code: "invalid_data".to_string(),
                message: format!("Could not parse {}: {err}", path.display()),
            })?;
            Ok(normalize_ticket_settings_value(&value))
        }
        None => Ok(TicketSettingsDocument::default()),
    }
}

fn save_ticket_settings_document(
    settings: &LocalSettings,
    app: &AppHandle,
    document: &TicketSettingsDocument,
) -> Result<(), AppError> {
    let path = shared_data_path(TICKET_SETTINGS_FILE, settings, app)?;
    write_json_file(&path, document)
}

fn read_tasks_document(
    settings: &LocalSettings,
    app: &AppHandle,
) -> Result<TaskDocument, AppError> {
    let path = shared_data_path(TASKS_FILE, settings, app)?;
    match read_text_file(&path)? {
        Some(content) => parse_task_document_content(&path, &content),
        None => Ok(TaskDocument::default()),
    }
}

fn save_tasks_document(
    settings: &LocalSettings,
    app: &AppHandle,
    document: &TaskDocument,
) -> Result<(), AppError> {
    let path = shared_data_path(TASKS_FILE, settings, app)?;
    write_json_file(&path, document)
}

fn read_notes_document(
    settings: &LocalSettings,
    app: &AppHandle,
) -> Result<NotesDocument, AppError> {
    let path = shared_data_path(NOTES_FILE, settings, app)?;
    let mut document: NotesDocument = read_or_default(&path)?;
    document.schema_version = default_schema_version();
    Ok(document)
}

fn save_notes_document(
    settings: &LocalSettings,
    app: &AppHandle,
    document: &NotesDocument,
) -> Result<(), AppError> {
    let path = shared_data_path(NOTES_FILE, settings, app)?;
    write_json_file(&path, document)
}

fn read_hidden_tickets_document(
    settings: &LocalSettings,
    app: &AppHandle,
) -> Result<HiddenTicketsDocument, AppError> {
    let path = shared_data_path(HIDDEN_TICKETS_FILE, settings, app)?;
    Ok(normalize_hidden_tickets_document(read_or_default(&path)?))
}

fn save_hidden_tickets_document(
    settings: &LocalSettings,
    app: &AppHandle,
    document: &HiddenTicketsDocument,
) -> Result<(), AppError> {
    let path = shared_data_path(HIDDEN_TICKETS_FILE, settings, app)?;
    write_json_file(&path, document)
}

fn read_local_settings(app: &AppHandle) -> Result<LocalSettings, AppError> {
    let path = local_settings_path(app)?;
    Ok(normalize_local_settings(read_or_default(&path)?))
}

fn read_local_settings_from_path(path: &Path) -> Result<LocalSettings, AppError> {
    Ok(normalize_local_settings(read_or_default(path)?))
}

fn save_local_settings(settings: &LocalSettings, app: &AppHandle) -> Result<(), AppError> {
    let path = local_settings_path(app)?;
    write_json_file(&path, &normalize_local_settings(settings.clone()))
}

fn is_valid_hostname(value: &str) -> bool {
    if value.is_empty()
        || value.len() > 253
        || value.contains("://")
        || value.contains('/')
        || value.contains('?')
        || value.contains('#')
        || value.contains(':')
        || value.chars().any(char::is_whitespace)
    {
        return false;
    }

    value.split('.').all(|label| {
        !label.is_empty()
            && label.len() <= 63
            && !label.starts_with('-')
            && !label.ends_with('-')
            && label
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '-')
    })
}

fn ticket_state(
    settings: &LocalSettings,
    app: &AppHandle,
    auth_error: Option<String>,
    store: &impl CredentialStore,
) -> Result<TicketSettingsState, AppError> {
    let document = read_ticket_settings_document(settings, app)?;
    let (has_api_key, store_error) = match store.get_api_key() {
        Ok(value) => (value.is_some(), None),
        Err(err) => (false, Some(err.message)),
    };

    Ok(TicketSettingsState {
        schema_version: document.schema_version,
        revision: document.revision,
        desk365_domain: document.desk365_domain,
        has_api_key,
        auth_error: auth_error.or(store_error),
    })
}

fn version_build_number() -> u64 {
    env!("TASKTRACKER_BUILD_NUMBER").parse().unwrap_or(0)
}

fn shared_file_modified_time(path: &Path) -> u64 {
    fs::metadata(path)
        .ok()
        .and_then(|metadata| metadata.modified().ok())
        .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn directory_shared_data_score(path: &Path) -> (usize, u64) {
    let mut count = 0usize;
    let mut newest = 0u64;
    for filename in SHARED_DATA_FILES {
        let candidate = path.join(filename);
        if candidate.is_file() {
            count += 1;
            newest = newest.max(shared_file_modified_time(&candidate));
        }
    }
    (count, newest)
}

fn directory_contains_populated_shared_data(path: &Path) -> bool {
    SHARED_DATA_FILES.iter().any(|filename| {
        let candidate = path.join(filename);
        candidate.is_file() && fs::metadata(candidate).map(|metadata| metadata.len() > 0).unwrap_or(false)
    })
}

fn copy_shared_data_from_source(source: &Path, destination: &Path) -> Result<Vec<String>, AppError> {
    let mut copied_files = Vec::new();
    for filename in SHARED_DATA_FILES {
        let source_path = source.join(filename);
        if !source_path.is_file() {
            continue;
        }

        let destination_path = destination.join(filename);
        let should_copy = if !destination_path.exists() {
            true
        } else if !fs::metadata(&destination_path)
            .map(|metadata| metadata.len() > 0)
            .unwrap_or(false)
        {
            true
        } else {
            shared_file_modified_time(&source_path) > shared_file_modified_time(&destination_path)
        };

        if should_copy {
            fs::copy(&source_path, &destination_path).map_err(|err| AppError {
                code: "write_failed".to_string(),
                message: format!(
                    "Could not copy {} into {}: {err}",
                    source_path.display(),
                    destination_path.display()
                ),
            })?;
            copied_files.push(filename.to_string());
        }
    }
    Ok(copied_files)
}

fn legacy_packaged_data_dirs() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    #[cfg(target_os = "windows")]
    {
        if let Some(appdata) = std::env::var_os("APPDATA") {
            let base = PathBuf::from(appdata);
            candidates.push(base.join("tasktracker-extreme-3000"));
            candidates.push(base.join("TaskTracker Extreme 3000"));
            candidates.push(base.join("TaskTracker-Extreme-3000"));
        }
    }
    #[cfg(target_os = "macos")]
    {
        if let Some(home) = std::env::var_os("HOME") {
            let base = PathBuf::from(home)
                .join("Library")
                .join("Application Support");
            candidates.push(base.join("tasktracker-extreme-3000"));
            candidates.push(base.join("TaskTracker Extreme 3000"));
        }
    }
    candidates
}

fn legacy_hardcoded_onedrive_dir() -> Option<PathBuf> {
    let root = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME"))?;
    Some(
        PathBuf::from(root)
            .join("OneDrive - VNANNJ")
            .join("John's TaskTracker")
            .join("data"),
    )
}

fn unique_candidate_dirs(candidates: Vec<PathBuf>, destination: &Path) -> Vec<PathBuf> {
    let mut seen = BTreeSet::new();
    let destination_key = destination.to_string_lossy().to_string();
    let mut unique = Vec::new();
    for candidate in candidates {
        let key = candidate.to_string_lossy().to_string();
        if key == destination_key || !seen.insert(key.clone()) {
            continue;
        }
        unique.push(candidate);
    }
    unique
}

fn select_migration_source(
    previous_settings: &LocalSettings,
    destination: &Path,
    app: &AppHandle,
) -> Option<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(previous_active) = shared_data_dir(previous_settings, app) {
        candidates.push(previous_active);
    }
    if let Ok(local_dir) = local_app_data_dir(app) {
        candidates.push(local_dir);
    }
    candidates.extend(legacy_packaged_data_dirs());
    if let Some(onedrive) = legacy_hardcoded_onedrive_dir() {
        candidates.push(onedrive);
    }

    unique_candidate_dirs(candidates, destination)
        .into_iter()
        .filter_map(|candidate| {
            let score = directory_shared_data_score(&candidate);
            if score.0 == 0 {
                None
            } else {
                Some((candidate, score))
            }
        })
        .max_by(|left, right| left.1.cmp(&right.1))
        .map(|(candidate, _)| candidate)
}

fn migrate_shared_data_if_needed(
    previous_settings: &LocalSettings,
    next_settings: &LocalSettings,
    app: &AppHandle,
) -> Result<Option<String>, AppError> {
    let Some(destination) = next_settings.sync_folder.as_deref() else {
        return Ok(None);
    };
    let destination = PathBuf::from(destination);
    fs::create_dir_all(&destination).map_err(|err| AppError {
        code: "write_failed".to_string(),
        message: format!("Could not create {}: {err}", destination.display()),
    })?;

    if directory_contains_populated_shared_data(&destination) {
        return Ok(Some(format!(
            "Kept the existing shared data in {}. No legacy import was needed.",
            destination.display()
        )));
    }

    let Some(source) = select_migration_source(previous_settings, &destination, app) else {
        return Ok(Some(
            "No legacy shared-data folder was found to import. The new sync folder will start empty."
                .to_string(),
        ));
    };

    let copied_files = copy_shared_data_from_source(&source, &destination)?;

    if copied_files.is_empty() {
        Ok(Some(format!(
            "Kept the files already present in {}. Legacy data from {} was not newer.",
            destination.display(),
            source.display()
        )))
    } else {
        Ok(Some(format!(
            "Imported {} shared-data file{} from {} into {}.",
            copied_files.len(),
            if copied_files.len() == 1 { "" } else { "s" },
            source.display(),
            destination.display()
        )))
    }
}

fn startup_import_candidates(settings: &LocalSettings, destination: &Path) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(sync_folder) = settings.sync_folder.as_deref() {
        candidates.push(PathBuf::from(sync_folder));
    }
    candidates.extend(legacy_packaged_data_dirs());
    if let Some(onedrive) = legacy_hardcoded_onedrive_dir() {
        candidates.push(onedrive);
    }
    unique_candidate_dirs(candidates, destination)
}

fn select_legacy_local_settings_source(
    current: &LocalSettings,
    _app: &AppHandle,
) -> Option<PathBuf> {
    if current.sync_folder.is_some() {
        return None;
    }

    legacy_packaged_data_dirs()
        .into_iter()
        .map(|dir| dir.join(LOCAL_SETTINGS_FILE))
        .filter(|path| path.is_file())
        .filter_map(|path| {
            let settings = read_local_settings_from_path(&path).ok()?;
            let sync_folder = normalize_sync_folder(settings.sync_folder.clone())?;
            let modified = shared_file_modified_time(&path);
            Some((path, sync_folder, modified))
        })
        .max_by(|left, right| left.2.cmp(&right.2))
        .map(|(path, _, _)| path)
}

fn import_legacy_local_settings_if_needed(
    current: &LocalSettings,
    app: &AppHandle,
) -> Result<(LocalSettings, Option<String>), AppError> {
    let Some(source) = select_legacy_local_settings_source(current, app) else {
        return Ok((current.clone(), None));
    };

    let imported = read_local_settings_from_path(&source)?;
    let merged = merge_missing_sync_folder(current, &imported);
    let Some(_) = merged.sync_folder.clone() else {
        return Ok((current.clone(), None));
    };
    Ok((
        merged,
        Some(format!(
            "Recovered the saved sync folder setting from {}.",
            source.display()
        )),
    ))
}

fn import_legacy_shared_data_into_destination(
    settings: &LocalSettings,
    destination: &Path,
    _app: &AppHandle,
) -> Result<Option<String>, AppError> {
    let source = startup_import_candidates(settings, destination)
        .into_iter()
        .filter_map(|candidate| {
            let score = directory_shared_data_score(&candidate);
            if score.0 == 0 {
                None
            } else {
                Some((candidate, score))
            }
        })
        .max_by(|left, right| left.1.cmp(&right.1))
        .map(|(candidate, _)| candidate);

    let Some(source) = source else {
        return Ok(Some(
            "No known legacy task/settings files were found to import.".to_string(),
        ));
    };

    let copied_files = copy_shared_data_from_source(&source, destination)?;
    if copied_files.is_empty() {
        Ok(Some(format!(
            "Found legacy data at {}, but the current destination already had newer shared files.",
            source.display()
        )))
    } else {
        Ok(Some(format!(
            "Imported {} shared-data file{} from {}.",
            copied_files.len(),
            if copied_files.len() == 1 { "" } else { "s" },
            source.display()
        )))
    }
}

fn run_startup_legacy_import(
    settings: &mut LocalSettings,
    app: &AppHandle,
) -> Result<Option<String>, AppError> {
    if settings.startup_legacy_import_done || settings.sync_folder.is_some() {
        return Ok(None);
    }

    let destination = local_app_data_dir(app)?;
    if directory_contains_populated_shared_data(&destination) {
        settings.startup_legacy_import_done = true;
        return Ok(None);
    }

    settings.startup_legacy_import_done = true;
    let notice = import_legacy_shared_data_into_destination(settings, &destination, app)?;
    if let Some(message) = notice {
        if message.starts_with("No known legacy") {
            Ok(None)
        } else {
            Ok(Some(format!("{message} Into the current app data folder.")))
        }
    } else {
        Ok(None)
    }
}

fn import_shared_data_from_source_dir(
    source: &Path,
    destination: &Path,
) -> Result<Option<String>, AppError> {
    if !source.is_dir() {
        return Err(AppError {
            code: "invalid_import_source".to_string(),
            message: format!("{} is not a folder containing shared data.", source.display()),
        });
    }

    let copied_files = copy_shared_data_from_source(source, destination)?;
    if copied_files.is_empty() {
        if directory_shared_data_score(source).0 == 0 {
            Ok(Some(format!(
                "No recognized shared-data files were found in {}.",
                source.display()
            )))
        } else {
            Ok(Some(format!(
                "The current destination already had newer shared files than {}.",
                source.display()
            )))
        }
    } else {
        Ok(Some(format!(
            "Imported {} shared-data file{} from {}.",
            copied_files.len(),
            if copied_files.len() == 1 { "" } else { "s" },
            source.display()
        )))
    }
}

fn choose_latest_task(left: &TaskItem, right: &TaskItem) -> TaskItem {
    match compare_timestamps(left.updated_at.as_deref(), right.updated_at.as_deref()) {
        Ordering::Greater => left.clone(),
        Ordering::Less => right.clone(),
        Ordering::Equal => right.clone(),
    }
}

fn normalize_task_orders(tasks: &mut Vec<TaskItem>) {
    let column_rank = |column: &str| match column {
        "standing" => 0,
        "priority" => 1,
        "inprogress" => 2,
        "todo" => 3,
        "rainyday" => 4,
        "done" => 5,
        _ => 6,
    };

    let mut buckets: BTreeMap<String, Vec<TaskItem>> = BTreeMap::new();
    for task in tasks.drain(..) {
        buckets.entry(task.column.clone()).or_default().push(task);
    }

    let mut normalized = Vec::new();
    let mut ordered_columns: Vec<String> = buckets.keys().cloned().collect();
    ordered_columns.sort_by_key(|column| column_rank(column));

    for column in ordered_columns {
        if let Some(mut entries) = buckets.remove(&column) {
            entries.sort_by(|left, right| {
                left.order
                    .cmp(&right.order)
                    .then(compare_timestamps(
                        left.updated_at.as_deref(),
                        right.updated_at.as_deref(),
                    ))
                    .then(left.id.cmp(&right.id))
            });
            for (index, entry) in entries.iter_mut().enumerate() {
                entry.order = index as i64;
            }
            normalized.extend(entries);
        }
    }

    *tasks = normalized;
}

fn merge_task_documents(
    latest: &TaskDocument,
    incoming: &TaskDocument,
    device_id: &str,
) -> (TaskDocument, Vec<String>) {
    let revisions_differ = latest.revision != incoming.revision;
    let fallback_timestamp = current_iso_timestamp();
    let mut tasks_by_id: BTreeMap<String, TaskItem> = BTreeMap::new();
    let mut tombstones_by_id: BTreeMap<String, TaskTombstone> = BTreeMap::new();
    let mut conflict_ids = Vec::new();

    let mut latest_tasks: BTreeMap<String, TaskItem> = BTreeMap::new();
    for task in &latest.tasks {
        latest_tasks.insert(task.id.clone(), task.clone());
    }
    let mut incoming_tasks: BTreeMap<String, TaskItem> = BTreeMap::new();
    for task in &incoming.tasks {
        incoming_tasks.insert(task.id.clone(), task.clone());
    }

    for tombstone in latest.tombstones.iter().chain(incoming.tombstones.iter()) {
        let candidate = tombstone.clone();
        match tombstones_by_id.get(&candidate.id) {
            Some(existing) if existing.updated_at > candidate.updated_at => {}
            _ => {
                tombstones_by_id.insert(candidate.id.clone(), candidate);
            }
        }
    }

    let mut all_ids: BTreeSet<String> = BTreeSet::new();
    all_ids.extend(latest_tasks.keys().cloned());
    all_ids.extend(incoming_tasks.keys().cloned());
    all_ids.extend(tombstones_by_id.keys().cloned());

    for id in all_ids {
        let latest_task = latest_tasks.get(&id);
        let incoming_task = incoming_tasks.get(&id);
        let tombstone = tombstones_by_id.get(&id);

        let chosen_task = match (latest_task, incoming_task) {
            (Some(left), Some(right)) => {
                if revisions_differ
                    && left != right
                    && compare_timestamps(left.updated_at.as_deref(), right.updated_at.as_deref())
                        != Ordering::Equal
                {
                    conflict_ids.push(id.clone());
                }
                Some(choose_latest_task(left, right))
            }
            (Some(task), None) => Some(task.clone()),
            (None, Some(task)) => Some(task.clone()),
            (None, None) => None,
        };

        let chosen_task_timestamp = chosen_task
            .as_ref()
            .map(|task| default_task_timestamp(task, &fallback_timestamp));

        let tombstone_wins = tombstone
            .map(|entry| {
                entry.updated_at >= chosen_task_timestamp.clone().unwrap_or_else(|| "".to_string())
            })
            .unwrap_or(false);

        if tombstone_wins {
            if let Some(entry) = tombstone.cloned() {
                tombstones_by_id.insert(id.clone(), entry);
            }
            continue;
        }

        if let Some(mut task) = chosen_task {
            if task.updated_at.as_deref().unwrap_or("").is_empty() {
                task.updated_at = Some(fallback_timestamp.clone());
            }
            tasks_by_id.insert(task.id.clone(), task);
            tombstones_by_id.remove(&id);
        }
    }

    let updated_at = [latest.updated_at.as_deref(), incoming.updated_at.as_deref()]
        .into_iter()
        .flatten()
        .max()
        .map(ToOwned::to_owned)
        .unwrap_or_else(current_iso_timestamp);

    let mut merged = TaskDocument {
        schema_version: default_schema_version(),
        revision: next_revision(latest.revision, incoming.revision),
        updated_at: Some(updated_at),
        updated_by: Some(device_id.to_string()),
        tasks: tasks_by_id.into_values().collect(),
        tombstones: tombstones_by_id.into_values().collect(),
    };

    normalize_task_orders(&mut merged.tasks);
    (merged, conflict_ids)
}

fn merge_hidden_tickets_documents(
    latest: &HiddenTicketsDocument,
    incoming: &HiddenTicketsDocument,
    device_id: &str,
) -> HiddenTicketsDocument {
    let mut states_by_ticket: BTreeMap<String, HiddenTicketState> = BTreeMap::new();
    for state in latest.states.iter().chain(incoming.states.iter()) {
        let candidate = state.clone();
        match states_by_ticket.get(&candidate.ticket_number) {
            Some(existing) if existing.updated_at > candidate.updated_at => {}
            _ => {
                states_by_ticket.insert(candidate.ticket_number.clone(), candidate);
            }
        }
    }

    let updated_at = [latest.updated_at.as_deref(), incoming.updated_at.as_deref()]
        .into_iter()
        .flatten()
        .max()
        .map(ToOwned::to_owned)
        .unwrap_or_else(current_iso_timestamp);

    normalize_hidden_tickets_document(HiddenTicketsDocument {
        schema_version: hidden_ticket_schema_version(),
        revision: next_revision(latest.revision, incoming.revision),
        updated_at: Some(updated_at),
        updated_by: Some(device_id.to_string()),
        tickets: Vec::new(),
        states: states_by_ticket.into_values().collect(),
    })
}

fn save_window_state(window: &tauri::WebviewWindow) {
    let Ok(path) = window_state_path(&window.app_handle()) else {
        return;
    };
    if let (Ok(pos), Ok(size)) = (window.outer_position(), window.outer_size()) {
        let data = json!({
            "x": pos.x,
            "y": pos.y,
            "width": size.width,
            "height": size.height,
        });
        let _ = write_json_file(&path, &data);
    }
}

fn monitor_contains_point(monitor: &tauri::Monitor, x: i32, y: i32) -> bool {
    let position = monitor.position();
    let size = monitor.size();
    let right = position.x.saturating_add(size.width as i32);
    let bottom = position.y.saturating_add(size.height as i32);
    x >= position.x && x < right && y >= position.y && y < bottom
}

fn saved_window_state_is_visible(
    window: &tauri::WebviewWindow,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
) -> bool {
    let Ok(monitors) = window.available_monitors() else {
        return true;
    };
    if monitors.is_empty() {
        return true;
    }

    let center_x = x.saturating_add((width / 2) as i32);
    let center_y = y.saturating_add((height / 2) as i32);
    monitors
        .iter()
        .any(|monitor| monitor_contains_point(monitor, center_x, center_y))
}

fn place_main_window_on_sidebar_edge(window: &tauri::WebviewWindow) {
    let monitor = window
        .current_monitor()
        .ok()
        .flatten()
        .or_else(|| window.primary_monitor().ok().flatten());
    let Some(monitor) = monitor else {
        return;
    };
    let Ok(size) = window.outer_size() else {
        return;
    };

    let scale = monitor.scale_factor();
    let edge_margin = (16.0 * scale).round() as i32;
    let top_margin = (72.0 * scale).round() as i32;
    let position = monitor.position();
    let monitor_size = monitor.size();

    let max_x = position
        .x
        .saturating_add(monitor_size.width as i32)
        .saturating_sub(size.width as i32)
        .saturating_sub(edge_margin);
    let max_y = position
        .y
        .saturating_add(monitor_size.height as i32)
        .saturating_sub(size.height as i32);

    let x = max_x.max(position.x);
    let y = position
        .y
        .saturating_add(top_margin)
        .clamp(position.y, max_y.max(position.y));

    let _ = window.set_position(tauri::PhysicalPosition::new(x, y));
}

fn restore_window_state(app: &AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    let Ok(path) = window_state_path(app) else {
        place_main_window_on_sidebar_edge(&window);
        return;
    };
    let Ok(Some(content)) = read_text_file(&path) else {
        place_main_window_on_sidebar_edge(&window);
        return;
    };
    let Ok(state) = serde_json::from_str::<Value>(&content) else {
        place_main_window_on_sidebar_edge(&window);
        return;
    };

    if let (Some(x), Some(y), Some(w), Some(h)) = (
        state.get("x").and_then(Value::as_i64),
        state.get("y").and_then(Value::as_i64),
        state.get("width").and_then(Value::as_u64),
        state.get("height").and_then(Value::as_u64),
    ) {
        let x = x as i32;
        let y = y as i32;
        let width = w as u32;
        let height = h as u32;
        if saved_window_state_is_visible(&window, x, y, width, height) {
            let _ = window.set_size(tauri::PhysicalSize::new(width, height));
            let _ = window.set_position(tauri::PhysicalPosition::new(x, y));
            return;
        }
    }

    place_main_window_on_sidebar_edge(&window);
}

fn toggle_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        if window.is_visible().unwrap_or(false) {
            let _ = window.hide();
        } else {
            let _ = window.show();
            let _ = window.set_focus();
        }
    }
}

fn focus_about_section(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
        let _ = window.emit(
            "navigate-to-tab",
            json!({ "tab": "settings", "section": "about" }),
        );
    }
}

fn open_quick_add(app: &AppHandle) {
    if let Some(existing) = app.get_webview_window("quick-add") {
        let _ = existing.set_focus();
        return;
    }

    let (x, y) = app
        .get_webview_window("main")
        .and_then(|window| window.current_monitor().ok().flatten())
        .map(|monitor| {
            let scale = monitor.scale_factor();
            let logical_width = monitor.size().width as f64 / scale;
            ((logical_width - 420.0) / 2.0, 100.0)
        })
        .unwrap_or((300.0, 100.0));

    let quick_add = tauri::WebviewWindowBuilder::new(
        app,
        "quick-add",
        tauri::WebviewUrl::App("quickadd.html".into()),
    )
    .title("Quick Add")
    .inner_size(420.0, 108.0)
    .position(x, y)
    .decorations(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .build();

    if let Ok(window) = quick_add {
        let app_handle = app.clone();
        window.on_window_event(move |event: &tauri::WindowEvent| {
            if let tauri::WindowEvent::Focused(false) = event {
                if let Some(quick_add_window) = app_handle.get_webview_window("quick-add") {
                    let _ = quick_add_window.close();
                }
            }
        });
    }
}

fn setup_tray(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    use tauri::image::Image;
    use tauri::menu::{Menu, MenuItem};
    use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

    let icon = Image::from_bytes(include_bytes!("../icons/icon.png"))
        .unwrap_or_else(|_| app.default_window_icon().unwrap().clone());

    let toggle = MenuItem::with_id(app, "toggle", "Show or hide", true, None::<&str>)?;
    let about = MenuItem::with_id(app, "about", "About", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&toggle, &about, &quit])?;

    TrayIconBuilder::new()
        .tooltip(PRODUCT_NAME)
        .icon(icon)
        .menu(&menu)
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                toggle_main_window(tray.app_handle());
            }
        })
        .on_menu_event(|app, event| match event.id.as_ref() {
            "toggle" => toggle_main_window(app),
            "about" => focus_about_section(app),
            "quit" => {
                if let Some(window) = app.get_webview_window("main") {
                    save_window_state(&window);
                }
                app.exit(0);
            }
            _ => {}
        })
        .build(app)?;

    Ok(())
}

fn format_iso_timestamp(unix_secs: u64) -> String {
    let s = unix_secs % 60;
    let m = (unix_secs / 60) % 60;
    let h = (unix_secs / 3600) % 24;

    let days = unix_secs / 86400 + 719468;
    let era = days / 146097;
    let doe = days % 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let mo = if mp < 10 { mp + 3 } else { mp - 9 };
    let yr = if mo <= 2 { y + 1 } else { y };

    format!("{yr:04}-{mo:02}-{d:02}T{h:02}:{m:02}:{s:02}.000Z")
}

fn emit_shared_data_change(app: &AppHandle, files: Vec<String>) {
    if files.is_empty() {
        return;
    }
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.emit("shared-data-changed", SharedDataChangeEvent { files });
    }
}

fn relevant_shared_files(paths: &[PathBuf]) -> Vec<String> {
    let mut names = BTreeSet::new();
    for path in paths {
        let Some(filename) = path.file_name().and_then(|entry| entry.to_str()) else {
            continue;
        };
        if SHARED_DATA_FILES.contains(&filename) {
            names.insert(filename.to_string());
        }
    }
    names.into_iter().collect()
}

fn sync_shared_data_watcher(app: &AppHandle) -> Result<(), AppError> {
    let state = app.state::<AppState>();
    let settings = state.local_settings.lock().unwrap().clone();
    let watched_dir = match shared_data_dir(&settings, app) {
        Ok(dir) => dir,
        Err(_) => {
            *state.shared_data_watcher.lock().unwrap() = None;
            return Ok(());
        }
    };

    let app_handle = app.clone();
    let mut watcher =
        notify::recommended_watcher(move |result: Result<notify::Event, notify::Error>| {
            let Ok(event) = result else {
                return;
            };
            match event.kind {
                EventKind::Access(_) => {}
                _ => {
                    let files = relevant_shared_files(&event.paths);
                    if !files.is_empty() {
                        emit_shared_data_change(&app_handle, files);
                    }
                }
            }
        })
        .map_err(|err| AppError {
            code: "watcher_unavailable".to_string(),
            message: format!("Could not create the shared-data watcher: {err}"),
        })?;

    watcher
        .watch(&watched_dir, RecursiveMode::NonRecursive)
        .map_err(|err| AppError {
            code: "watcher_unavailable".to_string(),
            message: format!("Could not watch {}: {err}", watched_dir.display()),
        })?;

    *state.shared_data_watcher.lock().unwrap() = Some(SharedDataWatcher {
        watcher,
        watched_dir,
    });
    Ok(())
}

#[tauri::command]
fn load_tasks(state: State<AppState>, app: AppHandle) -> CommandResponse<TaskDocument> {
    let settings = state.local_settings.lock().unwrap().clone();
    match read_tasks_document(&settings, &app) {
        Ok(document) => CommandResponse::ok(document),
        Err(err) => CommandResponse::err(&err.code, err.message),
    }
}

#[tauri::command]
fn save_tasks(
    document: TaskDocument,
    state: State<AppState>,
    app: AppHandle,
) -> CommandResponse<TaskSaveResult> {
    let settings = state.local_settings.lock().unwrap().clone();
    let device_id = settings
        .device_id
        .clone()
        .unwrap_or_else(generate_device_id);
    let latest = match read_tasks_document(&settings, &app) {
        Ok(document) => document,
        Err(err) => return CommandResponse::err(&err.code, err.message),
    };

    let incoming = normalize_task_document(document);
    let merged = latest.revision != incoming.revision;
    let (mut saved_document, conflict_ids) = merge_task_documents(&latest, &incoming, &device_id);
    saved_document.updated_at = Some(current_iso_timestamp());

    match save_tasks_document(&settings, &app, &saved_document) {
        Ok(_) => CommandResponse::ok(TaskSaveResult {
            document: saved_document,
            merged,
            conflict_ids,
        }),
        Err(err) => CommandResponse::err(&err.code, err.message),
    }
}

#[tauri::command]
fn load_notes(state: State<AppState>, app: AppHandle) -> CommandResponse<NotesDocument> {
    let settings = state.local_settings.lock().unwrap().clone();
    match read_notes_document(&settings, &app) {
        Ok(document) => CommandResponse::ok(document),
        Err(err) => CommandResponse::err(&err.code, err.message),
    }
}

#[tauri::command]
fn save_notes(
    document: NotesDocument,
    state: State<AppState>,
    app: AppHandle,
) -> CommandResponse<NotesSaveResult> {
    let settings = state.local_settings.lock().unwrap().clone();
    let device_id = settings
        .device_id
        .clone()
        .unwrap_or_else(generate_device_id);
    let latest = match read_notes_document(&settings, &app) {
        Ok(document) => document,
        Err(err) => return CommandResponse::err(&err.code, err.message),
    };

    let incoming = NotesDocument {
        schema_version: default_schema_version(),
        revision: document.revision,
        updated_at: Some(current_iso_timestamp()),
        updated_by: Some(device_id.clone()),
        content: document.content,
    };

    if latest.revision != incoming.revision && latest.content != incoming.content {
        return CommandResponse::ok(NotesSaveResult {
            document: latest,
            conflict: true,
        });
    }

    let saved_document = NotesDocument {
        schema_version: default_schema_version(),
        revision: next_revision(latest.revision, incoming.revision),
        updated_at: Some(current_iso_timestamp()),
        updated_by: Some(device_id),
        content: incoming.content,
    };

    match save_notes_document(&settings, &app, &saved_document) {
        Ok(_) => CommandResponse::ok(NotesSaveResult {
            document: saved_document,
            conflict: false,
        }),
        Err(err) => CommandResponse::err(&err.code, err.message),
    }
}

#[tauri::command]
fn load_hidden_tickets(
    state: State<AppState>,
    app: AppHandle,
) -> CommandResponse<HiddenTicketsDocument> {
    let settings = state.local_settings.lock().unwrap().clone();
    match read_hidden_tickets_document(&settings, &app) {
        Ok(document) => CommandResponse::ok(document),
        Err(err) => CommandResponse::err(&err.code, err.message),
    }
}

#[tauri::command]
fn save_hidden_tickets(
    document: HiddenTicketsDocument,
    state: State<AppState>,
    app: AppHandle,
) -> CommandResponse<HiddenTicketsSaveResult> {
    let settings = state.local_settings.lock().unwrap().clone();
    let device_id = settings
        .device_id
        .clone()
        .unwrap_or_else(generate_device_id);
    let latest = match read_hidden_tickets_document(&settings, &app) {
        Ok(document) => document,
        Err(err) => return CommandResponse::err(&err.code, err.message),
    };

    let incoming = normalize_hidden_tickets_document(document);
    let merged = latest.revision != incoming.revision;
    let mut saved_document = merge_hidden_tickets_documents(&latest, &incoming, &device_id);
    saved_document.updated_at = Some(current_iso_timestamp());

    match save_hidden_tickets_document(&settings, &app, &saved_document) {
        Ok(_) => CommandResponse::ok(HiddenTicketsSaveResult {
            document: saved_document,
            merged,
        }),
        Err(err) => CommandResponse::err(&err.code, err.message),
    }
}

#[tauri::command]
fn load_ticket_settings(
    state: State<AppState>,
    app: AppHandle,
) -> CommandResponse<TicketSettingsState> {
    let settings = state.local_settings.lock().unwrap().clone();
    let auth_error = state.ticket_auth_error.lock().unwrap().clone();
    match ticket_state(&settings, &app, auth_error, &KeyringCredentialStore) {
        Ok(ticket_state) => CommandResponse::ok(ticket_state),
        Err(err) => CommandResponse::err(&err.code, err.message),
    }
}

#[tauri::command]
fn save_ticket_settings(
    desk365_domain: String,
    state: State<AppState>,
    app: AppHandle,
) -> CommandResponse<()> {
    let domain = desk365_domain.trim();
    if !is_valid_hostname(domain) {
        return CommandResponse::err(
            "invalid_domain",
            "Enter a Desk365 hostname only, for example yourcompany.desk365.io.",
        );
    }

    let settings = state.local_settings.lock().unwrap().clone();
    let device_id = settings
        .device_id
        .clone()
        .unwrap_or_else(generate_device_id);
    let current_document = match read_ticket_settings_document(&settings, &app) {
        Ok(document) => document,
        Err(err) => return CommandResponse::err(&err.code, err.message),
    };

    let document = TicketSettingsDocument {
        schema_version: ticket_settings_schema_version(),
        revision: current_document.revision.saturating_add(1),
        updated_at: Some(current_iso_timestamp()),
        updated_by: Some(device_id),
        desk365_domain: Some(domain.to_string()),
    };

    match save_ticket_settings_document(&settings, &app, &document) {
        Ok(_) => CommandResponse::ok(()),
        Err(err) => CommandResponse::err(&err.code, err.message),
    }
}

#[tauri::command]
fn save_secure_api_key(api_key: String, state: State<AppState>) -> CommandResponse<()> {
    let value = api_key.trim();
    if value.is_empty() {
        return CommandResponse::err("missing_api_key", "Enter a Desk365 API key.");
    }

    match KeyringCredentialStore.set_api_key(value) {
        Ok(_) => match KeyringCredentialStore.get_api_key() {
            Ok(Some(_)) => {
                *state.ticket_auth_error.lock().unwrap() = None;
                CommandResponse::ok(())
            }
            Ok(None) => CommandResponse::err(
                "credential_store_unavailable",
                "The Desk365 API key did not remain saved after writing it to the secure credential store.",
            ),
            Err(err) => CommandResponse::err(&err.code, err.message),
        },
        Err(err) => CommandResponse::err(&err.code, err.message),
    }
}

#[tauri::command]
fn clear_secure_api_key(state: State<AppState>) -> CommandResponse<()> {
    match KeyringCredentialStore.clear_api_key() {
        Ok(_) => {
            *state.ticket_auth_error.lock().unwrap() = None;
            CommandResponse::ok(())
        }
        Err(err) => CommandResponse::err(&err.code, err.message),
    }
}

#[tauri::command]
fn load_local_settings_cmd(state: State<AppState>) -> CommandResponse<LocalSettings> {
    CommandResponse::ok(state.local_settings.lock().unwrap().clone())
}

#[tauri::command]
fn save_local_settings_cmd(
    settings: LocalSettings,
    state: State<AppState>,
    app: AppHandle,
) -> CommandResponse<StorageStatus> {
    let previous_settings = state.local_settings.lock().unwrap().clone();
    let normalized_settings = normalize_local_settings(settings);

    if let Err(err) = save_local_settings(&normalized_settings, &app) {
        return CommandResponse::err(&err.code, err.message);
    }

    *state.local_settings.lock().unwrap() = normalized_settings.clone();

    let migration_notice = match migrate_shared_data_if_needed(
        &previous_settings,
        &normalized_settings,
        &app,
    ) {
        Ok(notice) => notice,
        Err(err) => Some(err.message),
    };

    let migration_result =
        migrate_legacy_ticket_secret_if_needed(&normalized_settings, &app, &KeyringCredentialStore);
    *state.ticket_auth_error.lock().unwrap() = match migration_result {
        Ok(_) => None,
        Err(err) if err.code == "sync_unavailable" => None,
        Err(err) => Some(err.message),
    };

    let _ = sync_shared_data_watcher(&app);

    let mut status = compute_storage_status(&normalized_settings, &app);
    status.notice = migration_notice;
    CommandResponse::ok(status)
}

#[tauri::command]
fn attempt_legacy_import_cmd(
    state: State<AppState>,
    app: AppHandle,
) -> CommandResponse<StorageStatus> {
    let settings = state.local_settings.lock().unwrap().clone();
    let destination = match shared_data_dir(&settings, &app) {
        Ok(path) => path,
        Err(err) => return CommandResponse::err(&err.code, err.message),
    };

    let import_notice = match import_legacy_shared_data_into_destination(&settings, &destination, &app)
    {
        Ok(notice) => notice,
        Err(err) => return CommandResponse::err(&err.code, err.message),
    };

    let migration_result =
        migrate_legacy_ticket_secret_if_needed(&settings, &app, &KeyringCredentialStore);
    *state.ticket_auth_error.lock().unwrap() = match migration_result {
        Ok(_) => None,
        Err(err) if err.code == "sync_unavailable" => None,
        Err(err) => Some(err.message),
    };

    let mut status = compute_storage_status(&settings, &app);
    status.notice = import_notice;
    CommandResponse::ok(status)
}

#[tauri::command]
fn attempt_legacy_import_from_path_cmd(
    path: String,
    state: State<AppState>,
    app: AppHandle,
) -> CommandResponse<StorageStatus> {
    let selected_path = PathBuf::from(normalize_path_value(&path));
    let previous_settings = state.local_settings.lock().unwrap().clone();

    let notice = if selected_path
        .file_name()
        .and_then(|entry| entry.to_str())
        == Some(LOCAL_SETTINGS_FILE)
    {
        let imported = match read_local_settings_from_path(&selected_path) {
            Ok(settings) => settings,
            Err(err) => return CommandResponse::err(&err.code, err.message),
        };
        let Some(sync_folder) = normalize_sync_folder(imported.sync_folder) else {
            return CommandResponse::err(
                "invalid_import_source",
                format!(
                    "{} does not contain a saved sync folder setting.",
                    selected_path.display()
                ),
            );
        };

        let mut next_settings = previous_settings.clone();
        next_settings.sync_folder = Some(sync_folder);
        let next_settings = normalize_local_settings(next_settings);

        if let Err(err) = save_local_settings(&next_settings, &app) {
            return CommandResponse::err(&err.code, err.message);
        }
        *state.local_settings.lock().unwrap() = next_settings.clone();

        let migration_notice =
            match migrate_shared_data_if_needed(&previous_settings, &next_settings, &app) {
                Ok(notice) => notice,
                Err(err) => Some(err.message),
            };
        let _ = sync_shared_data_watcher(&app);

        let migration_result =
            migrate_legacy_ticket_secret_if_needed(&next_settings, &app, &KeyringCredentialStore);
        *state.ticket_auth_error.lock().unwrap() = match migration_result {
            Ok(_) => None,
            Err(err) if err.code == "sync_unavailable" => None,
            Err(err) => Some(err.message),
        };

        let mut status = compute_storage_status(&next_settings, &app);
        status.notice = Some(match migration_notice {
            Some(extra) => format!(
                "Recovered the sync folder setting from {}. {}",
                selected_path.display(),
                extra
            ),
            None => format!(
                "Recovered the sync folder setting from {}.",
                selected_path.display()
            ),
        });
        return CommandResponse::ok(status);
    } else {
        let source_dir = if selected_path.is_dir() {
            selected_path.clone()
        } else {
            match selected_path.parent() {
                Some(parent) => parent.to_path_buf(),
                None => {
                    return CommandResponse::err(
                        "invalid_import_source",
                        format!("Could not use {} as an import source.", selected_path.display()),
                    )
                }
            }
        };
        let destination = match shared_data_dir(&previous_settings, &app) {
            Ok(path) => path,
            Err(err) => return CommandResponse::err(&err.code, err.message),
        };

        match import_shared_data_from_source_dir(&source_dir, &destination) {
            Ok(notice) => notice,
            Err(err) => return CommandResponse::err(&err.code, err.message),
        }
    };

    let migration_result =
        migrate_legacy_ticket_secret_if_needed(&previous_settings, &app, &KeyringCredentialStore);
    *state.ticket_auth_error.lock().unwrap() = match migration_result {
        Ok(_) => None,
        Err(err) if err.code == "sync_unavailable" => None,
        Err(err) => Some(err.message),
    };

    let mut status = compute_storage_status(&previous_settings, &app);
    status.notice = notice;
    CommandResponse::ok(status)
}

#[tauri::command]
fn get_storage_status(state: State<AppState>, app: AppHandle) -> CommandResponse<StorageStatus> {
    let settings = state.local_settings.lock().unwrap().clone();
    CommandResponse::ok(compute_storage_status(&settings, &app))
}

#[tauri::command]
fn get_app_metadata() -> CommandResponse<AppMetadata> {
    CommandResponse::ok(AppMetadata {
        product_name: PRODUCT_NAME.to_string(),
        marketing_version: env!("CARGO_PKG_VERSION").to_string(),
        build_number: version_build_number(),
        license: APP_LICENSE.to_string(),
        github_url: GITHUB_URL.to_string(),
        primary_platform: PRIMARY_PLATFORM.to_string(),
        copyright: COPYRIGHT_TEXT.to_string(),
    })
}

#[tauri::command]
fn window_minimize(window: tauri::WebviewWindow) -> CommandResponse<()> {
    match window.minimize() {
        Ok(_) => CommandResponse::ok(()),
        Err(err) => CommandResponse::err("window_error", err.to_string()),
    }
}

#[tauri::command]
fn hide_window(window: tauri::WebviewWindow) -> CommandResponse<()> {
    match window.hide() {
        Ok(_) => CommandResponse::ok(()),
        Err(err) => CommandResponse::err("window_error", err.to_string()),
    }
}

#[tauri::command]
fn quit_app(app: AppHandle) -> CommandResponse<()> {
    if let Some(window) = app.get_webview_window("main") {
        save_window_state(&window);
    }
    app.exit(0);
    CommandResponse::ok(())
}

#[tauri::command]
fn toggle_always_on_top(window: tauri::WebviewWindow) -> CommandResponse<bool> {
    let current = window.is_always_on_top().unwrap_or(true);
    match window.set_always_on_top(!current) {
        Ok(_) => CommandResponse::ok(!current),
        Err(err) => CommandResponse::err("window_error", err.to_string()),
    }
}

#[tauri::command]
fn open_external_url(url: String, app: AppHandle) -> CommandResponse<()> {
    if !url.starts_with("https://") && !url.starts_with("http://") {
        return CommandResponse::err("invalid_url", "Only http and https URLs are supported.");
    }

    match app.opener().open_url(url, None::<&str>) {
        Ok(_) => CommandResponse::ok(()),
        Err(err) => CommandResponse::err("open_url_failed", err.to_string()),
    }
}

#[tauri::command]
async fn fetch_tickets(
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<CommandResponse<Value>, String> {
    let settings = state.local_settings.lock().unwrap().clone();
    let ticket_settings = match read_ticket_settings_document(&settings, &app) {
        Ok(document) => document,
        Err(err) => return Ok(CommandResponse::err(&err.code, err.message)),
    };

    let Some(desk365_domain) = ticket_settings.desk365_domain else {
        return Ok(CommandResponse::err(
            "missing_domain",
            "Desk365 is not configured yet. Add your Desk365 hostname and API key first.",
        ));
    };

    let api_key = match KeyringCredentialStore.get_api_key() {
        Ok(Some(value)) => value,
        Ok(None) => {
            return Ok(CommandResponse::err(
                "missing_api_key",
                "Desk365 is not fully configured yet. Add your saved API key first.",
            ))
        }
        Err(err) => return Ok(CommandResponse::err(&err.code, err.message)),
    };

    let client = match reqwest::Client::builder().use_rustls_tls().build() {
        Ok(client) => client,
        Err(err) => return Ok(CommandResponse::err("network_error", err.to_string())),
    };

    let base_url = format!("https://{desk365_domain}/apis/v3/tickets");
    let mut all_tickets: Vec<Value> = Vec::new();
    let mut offset = 0usize;

    loop {
        let url = format!("{base_url}?offset={offset}&order_by=updated_time&order_type=descending");

        let response = match client
            .get(&url)
            .header("Authorization", &api_key)
            .header("Accept", "application/json")
            .send()
            .await
        {
            Ok(response) => response,
            Err(err) => return Ok(CommandResponse::err("network_error", err.to_string())),
        };

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response.text().await.unwrap_or_default();
            let snippet = &body[..body.len().min(200)];
            return Ok(CommandResponse::err(
                "desk365_error",
                format!("Desk365 API error: {status} — {snippet}"),
            ));
        }

        let result: Value = match response.json().await {
            Ok(value) => value,
            Err(err) => return Ok(CommandResponse::err("network_error", err.to_string())),
        };

        let tickets = result
            .get("tickets")
            .or_else(|| result.get("data"))
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();

        let count = tickets.len();
        all_tickets.extend(tickets);

        if count < 30 || all_tickets.len() >= 300 {
            break;
        }

        offset += 30;
        tokio::time::sleep(Duration::from_millis(600)).await;
    }

    let normalized: Vec<Value> = all_tickets
        .iter()
        .map(|ticket| {
            let get_field = |keys: &[&str]| -> Value {
                keys.iter()
                    .find_map(|key| ticket.get(*key))
                    .cloned()
                    .unwrap_or(Value::Null)
            };

            json!({
                "TicketNumber": get_field(&["TicketNumber", "ticket_number", "ticketNumber", "id"]),
                "TicketId": get_field(&["ticket_id", "TicketId", "id", "ticket_number", "TicketNumber"]),
                "Subject": get_field(&["Subject", "subject", "title"]),
                "Status": get_field(&["Status", "status", "ticket_status"]),
                "Priority": get_field(&["Priority", "priority"]),
                "Agent": get_field(&["Agent", "agent", "assigned_to", "assignee"]),
                "Category": get_field(&["Category", "category"]),
                "UpdatedAt": get_field(&["UpdatedAt", "updated_time", "updated_at"]),
            })
        })
        .filter(|ticket| {
            let status = ticket.get("Status").and_then(Value::as_str).unwrap_or("");
            !matches!(status, "Closed" | "Resolved" | "closed" | "resolved")
        })
        .collect();

    Ok(CommandResponse::ok(json!({
        "tickets": normalized,
        "total": all_tickets.len(),
    })))
}

#[tauri::command]
fn show_notification(title: String, body: String, app: AppHandle) -> CommandResponse<()> {
    use tauri_plugin_notification::NotificationExt;

    match app
        .notification()
        .builder()
        .title(&title)
        .body(&body)
        .show()
    {
        Ok(_) => CommandResponse::ok(()),
        Err(err) => CommandResponse::err("notification_error", err.to_string()),
    }
}

#[tauri::command]
fn quick_add_task(title: String, state: State<AppState>, app: AppHandle) -> CommandResponse<()> {
    let trimmed = title.trim();
    if trimmed.is_empty() {
        return CommandResponse::err("missing_title", "Enter a task title first.");
    }

    let settings = state.local_settings.lock().unwrap().clone();
    let device_id = settings
        .device_id
        .clone()
        .unwrap_or_else(generate_device_id);
    let mut latest = match read_tasks_document(&settings, &app) {
        Ok(document) => document,
        Err(err) => return CommandResponse::err(&err.code, err.message),
    };

    let next_order = latest
        .tasks
        .iter()
        .filter(|task| task.column == "todo")
        .count() as i64;
    let now = current_iso_timestamp();
    latest.tasks.push(TaskItem {
        id: format!(
            "t_{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
        ),
        title: trimmed.to_string(),
        notes: String::new(),
        column: "todo".to_string(),
        order: next_order,
        created_at: Some(now.clone()),
        updated_at: Some(now.clone()),
    });
    latest.updated_at = Some(now);
    latest.updated_by = Some(device_id);
    latest.revision = latest.revision.saturating_add(1);
    normalize_task_orders(&mut latest.tasks);

    match save_tasks_document(&settings, &app, &latest) {
        Ok(_) => {
            if let Some(main_window) = app.get_webview_window("main") {
                let _ = main_window.emit("tasks-updated", ());
            }
            if let Some(quick_add_window) = app.get_webview_window("quick-add") {
                let _ = quick_add_window.close();
            }
            CommandResponse::ok(())
        }
        Err(err) => CommandResponse::err(&err.code, err.message),
    }
}

#[tauri::command]
fn close_quick_add(app: AppHandle) -> CommandResponse<()> {
    if let Some(window) = app.get_webview_window("quick-add") {
        match window.close() {
            Ok(_) => CommandResponse::ok(()),
            Err(err) => CommandResponse::err("window_error", err.to_string()),
        }
    } else {
        CommandResponse::ok(())
    }
}

#[tauri::command]
async fn pick_sync_folder(app: AppHandle) -> CommandResponse<Option<String>> {
    use tauri_plugin_dialog::DialogExt;

    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog()
        .file()
        .set_title("Select Sync Folder")
        .pick_folder(move |folder| {
            let _ = tx.send(folder);
        });

    match rx.await {
        Ok(Some(folder)) => match folder.clone().into_path() {
            Ok(path) => CommandResponse::ok(Some(path.to_string_lossy().to_string())),
            Err(_) => CommandResponse::ok(Some(normalize_path_value(&folder.to_string()))),
        },
        Ok(None) => CommandResponse::ok(None),
        Err(err) => CommandResponse::err("dialog_error", err.to_string()),
    }
}

#[tauri::command]
async fn pick_legacy_import_file(app: AppHandle) -> CommandResponse<Option<String>> {
    use tauri_plugin_dialog::DialogExt;

    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog()
        .file()
        .set_title("Select Legacy Task/Settings File")
        .add_filter("JSON", &["json"])
        .pick_file(move |file| {
            let _ = tx.send(file);
        });

    match rx.await {
        Ok(Some(file)) => match file.clone().into_path() {
            Ok(path) => CommandResponse::ok(Some(path.to_string_lossy().to_string())),
            Err(_) => CommandResponse::ok(Some(normalize_path_value(&file.to_string()))),
        },
        Ok(None) => CommandResponse::ok(None),
        Err(err) => CommandResponse::err("dialog_error", err.to_string()),
    }
}

fn main() {
    let credential_store = KeyringCredentialStore;

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    use tauri_plugin_global_shortcut::{Code, ShortcutState};
                    if event.state != ShortcutState::Pressed {
                        return;
                    }
                    match shortcut.key {
                        Code::KeyT => toggle_main_window(app),
                        Code::KeyN => open_quick_add(app),
                        _ => {}
                    }
                })
                .build(),
        )
        .setup(move |app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let initial_local_settings =
                read_local_settings(app.handle()).unwrap_or_else(|_| {
                    normalize_local_settings(LocalSettings::default())
                });
            let (mut local_settings, legacy_local_settings_notice) =
                import_legacy_local_settings_if_needed(&initial_local_settings, app.handle())
                    .unwrap_or((initial_local_settings, None));
            let startup_import_notice = run_startup_legacy_import(&mut local_settings, app.handle())
                .ok()
                .flatten();
            let _ = save_local_settings(&local_settings, app.handle());

            let migration_result = migrate_legacy_ticket_secret_if_needed(
                &local_settings,
                app.handle(),
                &credential_store,
            );
            let ticket_auth_error = match migration_result {
                Ok(_) => None,
                Err(err) if err.code == "sync_unavailable" => None,
                Err(err) => Some(err.message),
            };

            if let Some(notice) = legacy_local_settings_notice {
                eprintln!("{notice}");
            }

            if let Some(notice) = startup_import_notice {
                eprintln!("{notice}");
            }

            app.manage(AppState {
                local_settings: Mutex::new(local_settings),
                ticket_auth_error: Mutex::new(ticket_auth_error),
                shared_data_watcher: Mutex::new(None),
            });

            setup_tray(app)?;

            use tauri_plugin_global_shortcut::GlobalShortcutExt;
            app.handle()
                .global_shortcut()
                .register("CommandOrControl+Shift+T")?;
            app.handle()
                .global_shortcut()
                .register("CommandOrControl+Shift+N")?;

            let _ = sync_shared_data_watcher(app.handle());
            restore_window_state(app.handle());
            Ok(())
        })
        .on_window_event(|window, event| match event {
            tauri::WindowEvent::CloseRequested { api, .. } => {
                if window.label() == "main" {
                    api.prevent_close();
                    #[cfg(target_os = "macos")]
                    {
                        if window.emit("app-close-requested", ()).is_err() {
                            if let Some(main_window) =
                                window.app_handle().get_webview_window("main")
                            {
                                save_window_state(&main_window);
                            }
                            window.app_handle().exit(0);
                        }
                    }
                    #[cfg(not(target_os = "macos"))]
                    {
                        let _ = window.hide();
                    }
                }
            }
            tauri::WindowEvent::Moved(_) | tauri::WindowEvent::Resized(_) => {
                if window.label() == "main" {
                    if let Some(main_window) = window.app_handle().get_webview_window("main") {
                        save_window_state(&main_window);
                    }
                }
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            load_tasks,
            save_tasks,
            load_notes,
            save_notes,
            load_hidden_tickets,
            save_hidden_tickets,
            load_ticket_settings,
            save_ticket_settings,
            save_secure_api_key,
            clear_secure_api_key,
            load_local_settings_cmd,
            save_local_settings_cmd,
            attempt_legacy_import_cmd,
            attempt_legacy_import_from_path_cmd,
            get_storage_status,
            get_app_metadata,
            window_minimize,
            hide_window,
            quit_app,
            toggle_always_on_top,
            open_external_url,
            fetch_tickets,
            show_notification,
            quick_add_task,
            close_quick_add,
            pick_sync_folder,
            pick_legacy_import_file,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::{
        compare_timestamps, compute_storage_status_from_local_dir, copy_shared_data_from_source,
        generate_device_id, hidden_ticket_schema_version, is_valid_hostname,
        merge_missing_sync_folder, normalize_path_value,
        merge_hidden_tickets_documents, merge_task_documents, migrate_legacy_secret_value,
        normalize_hidden_tickets_document, normalize_local_settings, normalize_task_document,
        parse_task_document_content, AppError, CredentialStore, HiddenTicketState,
        HiddenTicketsDocument, LocalSettings, TaskDocument, TaskItem, TaskTombstone,
    };
    use serde_json::json;
    use std::cmp::Ordering;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::Mutex;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[derive(Default)]
    struct MemoryCredentialStore {
        value: Mutex<Option<String>>,
        fail_on_set: bool,
    }

    impl CredentialStore for MemoryCredentialStore {
        fn get_api_key(&self) -> Result<Option<String>, AppError> {
            Ok(self.value.lock().unwrap().clone())
        }

        fn set_api_key(&self, api_key: &str) -> Result<(), AppError> {
            if self.fail_on_set {
                return Err(AppError {
                    code: "credential_store_unavailable".to_string(),
                    message: "store failed".to_string(),
                });
            }
            *self.value.lock().unwrap() = Some(api_key.to_string());
            Ok(())
        }

        fn clear_api_key(&self) -> Result<(), AppError> {
            *self.value.lock().unwrap() = None;
            Ok(())
        }
    }

    fn task(id: &str, title: &str, updated_at: &str) -> TaskItem {
        TaskItem {
            id: id.to_string(),
            title: title.to_string(),
            notes: String::new(),
            column: "todo".to_string(),
            order: 0,
            created_at: Some(updated_at.to_string()),
            updated_at: Some(updated_at.to_string()),
        }
    }

    #[test]
    fn migrates_legacy_secret_only_after_store_success() {
        let store = MemoryCredentialStore::default();
        let normalized = migrate_legacy_secret_value(
            &json!({
                "schemaVersion": 1,
                "desk365Domain": "example.desk365.io",
                "apiKey": "secret-123"
            }),
            &store,
        )
        .unwrap()
        .unwrap();

        assert_eq!(store.get_api_key().unwrap(), Some("secret-123".to_string()));
        assert_eq!(normalized["schemaVersion"], json!(3));
        assert_eq!(normalized["desk365Domain"], json!("example.desk365.io"));
        assert!(normalized.get("apiKey").is_none());
    }

    #[test]
    fn failed_secret_migration_leaves_legacy_data_untouched() {
        let store = MemoryCredentialStore {
            value: Mutex::new(None),
            fail_on_set: true,
        };

        let result = migrate_legacy_secret_value(
            &json!({
                "schemaVersion": 1,
                "desk365Domain": "example.desk365.io",
                "apiKey": "secret-123"
            }),
            &store,
        );

        assert!(result.is_err());
        assert_eq!(store.get_api_key().unwrap(), None);
    }

    #[test]
    fn reports_unavailable_sync_without_local_fallback() {
        let missing_sync = std::env::temp_dir().join("tasktracker-missing-sync-folder");
        let status = compute_storage_status_from_local_dir(
            Ok(PathBuf::from(std::env::temp_dir())),
            &LocalSettings {
                sync_folder: Some(missing_sync.display().to_string()),
                device_id: Some(generate_device_id()),
                startup_legacy_import_done: false,
            },
        );

        assert_eq!(status.mode, "syncUnavailable");
        assert!(!status.shared_data_available);
        assert!(status
            .message
            .unwrap()
            .contains("Local settings and window position still work"));
    }

    #[test]
    fn validates_desk365_hostnames() {
        assert!(is_valid_hostname("example.desk365.io"));
        assert!(is_valid_hostname("helpdesk.internal"));
        assert!(!is_valid_hostname("https://example.desk365.io"));
        assert!(!is_valid_hostname("example.desk365.io/path"));
        assert!(!is_valid_hostname("example desk365 io"));
        assert!(!is_valid_hostname("-bad.example"));
    }

    #[test]
    fn normalizes_local_settings_with_device_id() {
        let normalized = normalize_local_settings(LocalSettings {
            sync_folder: Some("   ".to_string()),
            device_id: None,
            startup_legacy_import_done: false,
        });
        assert!(normalized.sync_folder.is_none());
        assert!(normalized.device_id.is_some());
    }

    #[test]
    fn merges_missing_sync_folder_from_imported_local_settings() {
        let current = normalize_local_settings(LocalSettings {
            sync_folder: None,
            device_id: Some("device-a".to_string()),
            startup_legacy_import_done: true,
        });
        let imported = normalize_local_settings(LocalSettings {
            sync_folder: Some("C:\\Sync\\TaskTracker".to_string()),
            device_id: Some("device-b".to_string()),
            startup_legacy_import_done: false,
        });

        let merged = merge_missing_sync_folder(&current, &imported);
        assert_eq!(merged.sync_folder, Some("C:\\Sync\\TaskTracker".to_string()));
        assert_eq!(merged.device_id, current.device_id);
        assert!(merged.startup_legacy_import_done);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn normalizes_file_uri_style_windows_paths() {
        assert_eq!(
            normalize_path_value("file:///C:/Users/john/Sync/TaskTracker"),
            "C:\\Users\\john\\Sync\\TaskTracker".to_string()
        );
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn normalizes_file_uri_style_unix_paths() {
        assert_eq!(
            normalize_path_value("file:///Users/john/Sync/TaskTracker"),
            "/Users/john/Sync/TaskTracker".to_string()
        );
    }

    #[test]
    fn merges_task_documents_and_keeps_newer_updates() {
        let latest = normalize_task_document(TaskDocument {
            revision: 5,
            updated_at: Some("2026-04-13T10:00:00.000Z".to_string()),
            updated_by: Some("device-a".to_string()),
            tasks: vec![task("1", "Latest", "2026-04-13T10:00:00.000Z")],
            tombstones: vec![],
            ..TaskDocument::default()
        });
        let incoming = normalize_task_document(TaskDocument {
            revision: 4,
            updated_at: Some("2026-04-13T09:00:00.000Z".to_string()),
            updated_by: Some("device-b".to_string()),
            tasks: vec![
                task("1", "Older", "2026-04-13T09:00:00.000Z"),
                task("2", "New task", "2026-04-13T09:30:00.000Z"),
            ],
            tombstones: vec![],
            ..TaskDocument::default()
        });

        let (merged, conflicts) = merge_task_documents(&latest, &incoming, "device-c");
        assert_eq!(merged.revision, 6);
        assert_eq!(merged.tasks.len(), 2);
        assert!(merged.tasks.iter().any(|entry| entry.id == "1" && entry.title == "Latest"));
        assert!(merged.tasks.iter().any(|entry| entry.id == "2"));
        assert_eq!(conflicts, vec!["1".to_string()]);
    }

    #[test]
    fn local_same_revision_changes_do_not_trigger_conflict_warning() {
        let latest = normalize_task_document(TaskDocument {
            revision: 5,
            updated_at: Some("2026-04-13T10:00:00.000Z".to_string()),
            updated_by: Some("device-a".to_string()),
            tasks: vec![task("1", "Original", "2026-04-13T10:00:00.000Z")],
            tombstones: vec![],
            ..TaskDocument::default()
        });
        let incoming = normalize_task_document(TaskDocument {
            revision: 5,
            updated_at: Some("2026-04-13T10:01:00.000Z".to_string()),
            updated_by: Some("device-a".to_string()),
            tasks: vec![task("1", "Dragged locally", "2026-04-13T10:01:00.000Z")],
            tombstones: vec![],
            ..TaskDocument::default()
        });

        let (merged, conflicts) = merge_task_documents(&latest, &incoming, "device-a");
        assert!(conflicts.is_empty());
        assert_eq!(merged.tasks[0].title, "Dragged locally");
    }

    #[test]
    fn tombstones_win_over_older_task_versions() {
        let latest = normalize_task_document(TaskDocument {
            revision: 3,
            updated_at: Some("2026-04-13T10:00:00.000Z".to_string()),
            tasks: vec![task("1", "Keep me", "2026-04-13T09:00:00.000Z")],
            tombstones: vec![],
            ..TaskDocument::default()
        });
        let incoming = normalize_task_document(TaskDocument {
            revision: 2,
            updated_at: Some("2026-04-13T10:05:00.000Z".to_string()),
            tasks: vec![],
            tombstones: vec![TaskTombstone {
                id: "1".to_string(),
                updated_at: "2026-04-13T10:05:00.000Z".to_string(),
                updated_by: Some("device-b".to_string()),
            }],
            ..TaskDocument::default()
        });

        let (merged, _) = merge_task_documents(&latest, &incoming, "device-c");
        assert!(merged.tasks.is_empty());
        assert_eq!(merged.tombstones.len(), 1);
    }

    #[test]
    fn normalizes_legacy_hidden_ticket_lists() {
        let normalized = normalize_hidden_tickets_document(HiddenTicketsDocument {
            schema_version: 1,
            revision: 0,
            updated_at: Some("2026-04-13T10:00:00.000Z".to_string()),
            updated_by: None,
            tickets: vec!["1001".to_string()],
            states: vec![],
        });

        assert_eq!(normalized.schema_version, hidden_ticket_schema_version());
        assert_eq!(normalized.tickets, vec!["1001".to_string()]);
        assert_eq!(normalized.states.len(), 1);
    }

    #[test]
    fn merges_hidden_ticket_state_by_latest_timestamp() {
        let latest = HiddenTicketsDocument {
            revision: 2,
            updated_at: Some("2026-04-13T10:00:00.000Z".to_string()),
            updated_by: Some("device-a".to_string()),
            tickets: vec!["1001".to_string()],
            states: vec![HiddenTicketState {
                ticket_number: "1001".to_string(),
                hidden: true,
                updated_at: "2026-04-13T10:00:00.000Z".to_string(),
            }],
            ..HiddenTicketsDocument::default()
        };
        let incoming = HiddenTicketsDocument {
            revision: 1,
            updated_at: Some("2026-04-13T10:05:00.000Z".to_string()),
            updated_by: Some("device-b".to_string()),
            tickets: vec![],
            states: vec![HiddenTicketState {
                ticket_number: "1001".to_string(),
                hidden: false,
                updated_at: "2026-04-13T10:05:00.000Z".to_string(),
            }],
            ..HiddenTicketsDocument::default()
        };

        let merged = merge_hidden_tickets_documents(&latest, &incoming, "device-c");
        assert!(merged.tickets.is_empty());
        assert_eq!(merged.states[0].hidden, false);
    }

    #[test]
    fn compares_missing_timestamps_consistently() {
        assert_eq!(compare_timestamps(Some("a"), None), Ordering::Greater);
        assert_eq!(compare_timestamps(None, Some("a")), Ordering::Less);
        assert_eq!(compare_timestamps(None, None), Ordering::Equal);
    }

    #[test]
    fn parses_schema_one_task_document_content() {
        let path = PathBuf::from("tasks.json");
        let document = parse_task_document_content(
            &path,
            r#"{"schemaVersion":1,"tasks":[{"id":"t1","title":"Legacy","column":"todo","order":0}]}"#,
        )
        .unwrap();

        assert_eq!(document.tasks.len(), 1);
        assert_eq!(document.tasks[0].title, "Legacy");
        assert_eq!(document.revision, 0);
        assert_eq!(document.schema_version, 2);
    }

    #[test]
    fn parses_raw_task_array_content() {
        let path = PathBuf::from("tasks.json");
        let document = parse_task_document_content(
            &path,
            r#"[{"title":"Raw Legacy","column":"priority"},{"id":"t2","title":"Second"}]"#,
        )
        .unwrap();

        assert_eq!(document.tasks.len(), 2);
        assert_eq!(document.tasks[0].column, "priority");
        assert!(!document.tasks[0].id.is_empty());
    }

    #[test]
    fn copies_shared_data_into_empty_destination() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let root = std::env::temp_dir().join(format!("tasktracker-startup-import-{stamp}"));
        let source = root.join("source");
        let destination = root.join("destination");
        fs::create_dir_all(&source).unwrap();
        fs::create_dir_all(&destination).unwrap();
        fs::write(source.join("tasks.json"), r#"{"schemaVersion":1,"tasks":[{"id":"t1","title":"Legacy"}]}"#).unwrap();
        fs::write(source.join("config.json"), r#"{"desk365Domain":"example.desk365.io"}"#).unwrap();

        let copied = copy_shared_data_from_source(&source, &destination).unwrap();
        assert_eq!(copied.len(), 2);
        assert!(destination.join("tasks.json").is_file());
        assert!(destination.join("config.json").is_file());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn preserves_populated_destination_on_startup_copy() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let root = std::env::temp_dir().join(format!("tasktracker-startup-skip-{stamp}"));
        let source = root.join("source");
        let destination = root.join("destination");
        fs::create_dir_all(&source).unwrap();
        fs::create_dir_all(&destination).unwrap();
        fs::write(source.join("tasks.json"), r#"{"schemaVersion":1,"tasks":[{"id":"t1","title":"Legacy"}]}"#).unwrap();
        fs::write(destination.join("tasks.json"), r#"{"schemaVersion":2,"revision":5,"tasks":[{"id":"t1","title":"Current"}]}"#).unwrap();

        let copied = copy_shared_data_from_source(&source, &destination).unwrap();
        assert!(copied.is_empty());
        let current = fs::read_to_string(destination.join("tasks.json")).unwrap();
        assert!(current.contains("Current"));

        let _ = fs::remove_dir_all(root);
    }

}
