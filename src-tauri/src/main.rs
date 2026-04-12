// Prevents additional console window on Windows in release — DO NOT REMOVE
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod version_manifest;

use keyring::Entry;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
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

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
struct LocalSettings {
    #[serde(default)]
    sync_folder: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct TaskDocument {
    #[serde(default = "default_schema_version")]
    schema_version: u32,
    #[serde(default)]
    tasks: Vec<Value>,
}

impl Default for TaskDocument {
    fn default() -> Self {
        Self {
            schema_version: default_schema_version(),
            tasks: Vec::new(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct NotesDocument {
    #[serde(default = "default_schema_version")]
    schema_version: u32,
    #[serde(default)]
    content: String,
    #[serde(default)]
    updated_at: Option<String>,
}

impl Default for NotesDocument {
    fn default() -> Self {
        Self {
            schema_version: default_schema_version(),
            content: String::new(),
            updated_at: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct HiddenTicketsDocument {
    #[serde(default = "default_schema_version")]
    schema_version: u32,
    #[serde(default)]
    tickets: Vec<String>,
}

impl Default for HiddenTicketsDocument {
    fn default() -> Self {
        Self {
            schema_version: default_schema_version(),
            tickets: Vec::new(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct TicketSettingsDocument {
    #[serde(default = "ticket_settings_schema_version")]
    schema_version: u32,
    #[serde(default)]
    desk365_domain: Option<String>,
}

impl Default for TicketSettingsDocument {
    fn default() -> Self {
        Self {
            schema_version: ticket_settings_schema_version(),
            desk365_domain: None,
        }
    }
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct TicketSettingsState {
    schema_version: u32,
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

#[derive(Debug)]
struct AppState {
    local_settings: Mutex<LocalSettings>,
    ticket_auth_error: Mutex<Option<String>>,
}

fn default_schema_version() -> u32 {
    1
}

fn ticket_settings_schema_version() -> u32 {
    2
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
        self.entry()?
            .set_password(api_key)
            .map_err(|err| AppError {
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

fn local_app_data_dir(app: &AppHandle) -> Result<PathBuf, AppError> {
    let dir = app.path().app_data_dir().map_err(|err| AppError {
        code: "app_data_unavailable".to_string(),
        message: format!("Could not resolve the app data directory: {err}"),
    })?;

    fs::create_dir_all(&dir).map_err(|err| AppError {
        code: "app_data_unavailable".to_string(),
        message: format!("Could not create the app data directory at {}: {err}", dir.display()),
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
            },
            Err(err) => StorageStatus {
                mode: "localUnavailable".to_string(),
                configured_path: None,
                active_path: None,
                shared_data_available: false,
                message: Some(err.message),
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

    fs::write(path, content).map_err(|err| AppError {
        code: "write_failed".to_string(),
        message: format!("Could not write {}: {err}", path.display()),
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

fn normalize_ticket_settings_value(value: &Value) -> TicketSettingsDocument {
    TicketSettingsDocument {
        schema_version: ticket_settings_schema_version(),
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
    let mut normalized = serde_json::to_value(normalize_ticket_settings_value(value)).map_err(|err| {
        AppError {
            code: "invalid_data".to_string(),
            message: format!("Could not normalize legacy Desk365 settings: {err}"),
        }
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

fn read_tasks_document(settings: &LocalSettings, app: &AppHandle) -> Result<TaskDocument, AppError> {
    let path = shared_data_path(TASKS_FILE, settings, app)?;
    read_or_default(&path)
}

fn save_tasks_document(
    settings: &LocalSettings,
    app: &AppHandle,
    document: &TaskDocument,
) -> Result<(), AppError> {
    let path = shared_data_path(TASKS_FILE, settings, app)?;
    write_json_file(&path, document)
}

fn read_notes_document(settings: &LocalSettings, app: &AppHandle) -> Result<NotesDocument, AppError> {
    let path = shared_data_path(NOTES_FILE, settings, app)?;
    read_or_default(&path)
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
    read_or_default(&path)
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
    read_or_default(&path)
}

fn save_local_settings(settings: &LocalSettings, app: &AppHandle) -> Result<(), AppError> {
    let path = local_settings_path(app)?;
    write_json_file(&path, settings)
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
        desk365_domain: document.desk365_domain,
        has_api_key,
        auth_error: auth_error.or(store_error),
    })
}

fn version_build_number() -> u64 {
    env!("TASKTRACKER_BUILD_NUMBER").parse().unwrap_or(0)
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

fn restore_window_state(app: &AppHandle) {
    let Ok(path) = window_state_path(app) else {
        return;
    };
    let Ok(Some(content)) = read_text_file(&path) else {
        return;
    };
    let Ok(state) = serde_json::from_str::<Value>(&content) else {
        return;
    };
    let Some(window) = app.get_webview_window("main") else {
        return;
    };

    if let (Some(x), Some(y), Some(w), Some(h)) = (
        state.get("x").and_then(Value::as_i64),
        state.get("y").and_then(Value::as_i64),
        state.get("width").and_then(Value::as_u64),
        state.get("height").and_then(Value::as_u64),
    ) {
        let _ = window.set_position(tauri::PhysicalPosition::new(x as i32, y as i32));
        let _ = window.set_size(tauri::PhysicalSize::new(w as u32, h as u32));
    }
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
        let _ = window.emit("navigate-to-tab", json!({ "tab": "settings", "section": "about" }));
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
) -> CommandResponse<()> {
    let settings = state.local_settings.lock().unwrap().clone();
    match save_tasks_document(&settings, &app, &document) {
        Ok(_) => CommandResponse::ok(()),
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
) -> CommandResponse<()> {
    let settings = state.local_settings.lock().unwrap().clone();
    match save_notes_document(&settings, &app, &document) {
        Ok(_) => CommandResponse::ok(()),
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
) -> CommandResponse<()> {
    let settings = state.local_settings.lock().unwrap().clone();
    match save_hidden_tickets_document(&settings, &app, &document) {
        Ok(_) => CommandResponse::ok(()),
        Err(err) => CommandResponse::err(&err.code, err.message),
    }
}

#[tauri::command]
fn load_ticket_settings(state: State<AppState>, app: AppHandle) -> CommandResponse<TicketSettingsState> {
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
    let document = TicketSettingsDocument {
        schema_version: ticket_settings_schema_version(),
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
        Ok(_) => {
            *state.ticket_auth_error.lock().unwrap() = None;
            CommandResponse::ok(())
        }
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
    match save_local_settings(&settings, &app) {
        Ok(_) => {
            *state.local_settings.lock().unwrap() = settings.clone();
            let migration_result =
                migrate_legacy_ticket_secret_if_needed(&settings, &app, &KeyringCredentialStore);
            *state.ticket_auth_error.lock().unwrap() = match migration_result {
                Ok(_) => None,
                Err(err) if err.code == "sync_unavailable" => None,
                Err(err) => Some(err.message),
            };

            CommandResponse::ok(compute_storage_status(&settings, &app))
        }
        Err(err) => CommandResponse::err(&err.code, err.message),
    }
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
        let url = format!(
            "{base_url}?offset={offset}&order_by=updated_time&order_type=descending"
        );

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
        tokio::time::sleep(std::time::Duration::from_millis(600)).await;
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

    match app.notification().builder().title(&title).body(&body).show() {
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
    let mut document = match read_tasks_document(&settings, &app) {
        Ok(document) => document,
        Err(err) => return CommandResponse::err(&err.code, err.message),
    };

    for task in &mut document.tasks {
        if task.get("column").and_then(Value::as_str) == Some("todo") {
            if let Some(order) = task.get("order").and_then(Value::as_i64) {
                task["order"] = json!(order + 1);
            }
        }
    }

    let ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let now = format_iso_timestamp(ms as u64 / 1000);

    document.tasks.push(json!({
        "id": format!("t_{ms}"),
        "title": trimmed,
        "notes": "",
        "column": "todo",
        "order": 0,
        "createdAt": now,
        "updatedAt": now,
    }));

    match save_tasks_document(&settings, &app, &document) {
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
    app.dialog().file().set_title("Select Sync Folder").pick_folder(move |folder| {
        let _ = tx.send(folder);
    });

    match rx.await {
        Ok(Some(folder)) => CommandResponse::ok(Some(folder.to_string())),
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

            let local_settings = read_local_settings(app.handle()).unwrap_or_default();
            let migration_result =
                migrate_legacy_ticket_secret_if_needed(&local_settings, app.handle(), &credential_store);
            let ticket_auth_error = match migration_result {
                Ok(_) => None,
                Err(err) if err.code == "sync_unavailable" => None,
                Err(err) => Some(err.message),
            };

            app.manage(AppState {
                local_settings: Mutex::new(local_settings),
                ticket_auth_error: Mutex::new(ticket_auth_error),
            });

            setup_tray(app)?;

            use tauri_plugin_global_shortcut::GlobalShortcutExt;
            app.handle()
                .global_shortcut()
                .register("CommandOrControl+Shift+T")?;
            app.handle()
                .global_shortcut()
                .register("CommandOrControl+Shift+N")?;

            restore_window_state(app.handle());
            Ok(())
        })
        .on_window_event(|window, event| match event {
            tauri::WindowEvent::CloseRequested { api, .. } => {
                if window.label() == "main" {
                    api.prevent_close();
                    let _ = window.hide();
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
            get_storage_status,
            get_app_metadata,
            window_minimize,
            hide_window,
            toggle_always_on_top,
            open_external_url,
            fetch_tickets,
            show_notification,
            quick_add_task,
            close_quick_add,
            pick_sync_folder,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::{
        compute_storage_status_from_local_dir, is_valid_hostname, migrate_legacy_secret_value,
        AppError, CredentialStore, LocalSettings,
    };
    use serde_json::json;
    use std::path::PathBuf;
    use std::sync::Mutex;

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
        assert_eq!(normalized["schemaVersion"], json!(2));
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
            },
        );

        assert_eq!(status.mode, "syncUnavailable");
        assert!(!status.shared_data_available);
        assert!(status.message.unwrap().contains("Local settings and window position still work"));
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
}
