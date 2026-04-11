// Prevents additional console window on Windows in release — DO NOT REMOVE
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};

// ─── State ────────────────────────────────────────────────────────────────────

/// Machine-specific settings stored in app-data dir — never synced, never committed.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct LocalSettings {
    /// Optional path to a cloud-synced folder (OneDrive, Dropbox, iCloud Drive, etc.)
    /// When set, tasks/notes/config/hidden-tickets are read and written there.
    /// window-state.json always stays machine-local.
    #[serde(default)]
    pub sync_folder: Option<String>,
}

pub struct AppState {
    pub local_settings: Mutex<LocalSettings>,
}

// ─── Path resolution ──────────────────────────────────────────────────────────

fn local_settings_path(app: &AppHandle) -> PathBuf {
    app.path()
        .app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("local-settings.json")
}

fn read_local_settings(app: &AppHandle) -> LocalSettings {
    let path = local_settings_path(app);
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn get_data_dir(state: &State<AppState>, app: &AppHandle) -> PathBuf {
    let settings = state.local_settings.lock().unwrap();
    if let Some(ref folder) = settings.sync_folder {
        let path = PathBuf::from(folder);
        if path.exists() {
            return path;
        }
    }
    let dir = app
        .path()
        .app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."));
    let _ = std::fs::create_dir_all(&dir);
    dir
}

fn resolve_path(filename: &str, state: &State<AppState>, app: &AppHandle) -> PathBuf {
    // window-state.json is always machine-local, never goes into the sync folder
    if filename == "window-state.json" {
        return app
            .path()
            .app_data_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(filename);
    }
    get_data_dir(state, app).join(filename)
}

// ─── File I/O commands ────────────────────────────────────────────────────────

#[tauri::command]
fn load_file(
    filename: String,
    state: State<AppState>,
    app: AppHandle,
) -> Option<serde_json::Value> {
    let path = resolve_path(&filename, &state, &app);
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
}

#[tauri::command]
fn save_file(
    filename: String,
    data: serde_json::Value,
    state: State<AppState>,
    app: AppHandle,
) -> bool {
    let path = resolve_path(&filename, &state, &app);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    serde_json::to_string_pretty(&data)
        .map(|content| std::fs::write(&path, content).is_ok())
        .unwrap_or(false)
}

// ─── Window commands ──────────────────────────────────────────────────────────

#[tauri::command]
fn window_minimize(window: tauri::WebviewWindow) {
    let _ = window.minimize();
}

#[tauri::command]
fn hide_window(window: tauri::WebviewWindow) {
    let _ = window.hide();
}

#[tauri::command]
fn toggle_always_on_top(window: tauri::WebviewWindow) -> bool {
    let current = window.is_always_on_top().unwrap_or(true);
    let _ = window.set_always_on_top(!current);
    !current
}

// ─── Open external URL ────────────────────────────────────────────────────────

#[tauri::command]
fn open_external(url: String) -> Result<(), String> {
    if !url.starts_with("https://") && !url.starts_with("http://") {
        return Err("Only http/https URLs are supported".to_string());
    }

    #[cfg(target_os = "windows")]
    std::process::Command::new("cmd")
        .args(["/c", "start", "", &url])
        .spawn()
        .map_err(|e| e.to_string())?;

    #[cfg(target_os = "macos")]
    std::process::Command::new("open")
        .arg(&url)
        .spawn()
        .map_err(|e| e.to_string())?;

    #[cfg(target_os = "linux")]
    std::process::Command::new("xdg-open")
        .arg(&url)
        .spawn()
        .map_err(|e| e.to_string())?;

    Ok(())
}

// ─── Desk365 API fetch ────────────────────────────────────────────────────────

#[tauri::command]
async fn fetch_tickets(api_key: String, desk365_domain: String) -> serde_json::Value {
    let client = match reqwest::Client::builder().use_rustls_tls().build() {
        Ok(c) => c,
        Err(e) => return serde_json::json!({ "success": false, "error": e.to_string() }),
    };

    let base_url = format!("https://{}/apis/v3/tickets", desk365_domain);
    let mut all_tickets: Vec<serde_json::Value> = Vec::new();
    let mut offset = 0usize;

    loop {
        let url = format!(
            "{base_url}?offset={offset}&order_by=updated_time&order_type=descending"
        );

        let resp = match client
            .get(&url)
            .header("Authorization", &api_key)
            .header("Accept", "application/json")
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => return serde_json::json!({ "success": false, "error": e.to_string() }),
        };

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            let snippet = &body[..body.len().min(200)];
            return serde_json::json!({
                "success": false,
                "error": format!("API error: {status} — {snippet}")
            });
        }

        let result: serde_json::Value = match resp.json().await {
            Ok(v) => v,
            Err(e) => return serde_json::json!({ "success": false, "error": e.to_string() }),
        };

        let tickets = result
            .get("tickets")
            .or_else(|| result.get("data"))
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        let count = tickets.len();
        all_tickets.extend(tickets);

        if count < 30 || all_tickets.len() >= 300 {
            break;
        }
        offset += 30;

        // Respect Desk365's rate limit (max 2 req/sec)
        tokio::time::sleep(std::time::Duration::from_millis(600)).await;
    }

    // Normalize field names — Desk365 API may use varying case conventions
    let get_field = |t: &serde_json::Value, keys: &[&str]| -> serde_json::Value {
        for k in keys {
            if let Some(v) = t.get(*k) {
                if !v.is_null() {
                    return v.clone();
                }
            }
        }
        serde_json::Value::Null
    };

    let normalized: Vec<serde_json::Value> = all_tickets
        .iter()
        .map(|t| {
            serde_json::json!({
                "TicketNumber": get_field(t, &["TicketNumber", "ticket_number", "ticketNumber", "id"]),
                "TicketId":     get_field(t, &["ticket_id", "TicketId", "id", "ticket_number", "TicketNumber"]),
                "Subject":      get_field(t, &["Subject", "subject", "title"]),
                "Status":       get_field(t, &["Status", "status", "ticket_status"]),
                "Priority":     get_field(t, &["Priority", "priority"]),
                "Agent":        get_field(t, &["Agent", "agent", "assigned_to", "assignee"]),
                "Category":     get_field(t, &["Category", "category"]),
                "UpdatedAt":    get_field(t, &["UpdatedAt", "updated_time", "updated_at"]),
            })
        })
        .collect();

    let unresolved: Vec<serde_json::Value> = normalized
        .into_iter()
        .filter(|t| {
            let s = t.get("Status").and_then(|v| v.as_str()).unwrap_or("");
            !matches!(s, "Closed" | "Resolved" | "closed" | "resolved")
        })
        .collect();

    serde_json::json!({
        "success": true,
        "tickets": unresolved,
        "total": all_tickets.len()
    })
}

// ─── Notifications ────────────────────────────────────────────────────────────

#[tauri::command]
fn show_notification(title: String, body: String, app: AppHandle) {
    use tauri_plugin_notification::NotificationExt;
    let _ = app.notification().builder().title(&title).body(&body).show();
}

// ─── Quick-add task ───────────────────────────────────────────────────────────

#[tauri::command]
fn quick_add_task(title: String, state: State<AppState>, app: AppHandle) -> bool {
    let path = resolve_path("tasks.json", &state, &app);

    let mut data: serde_json::Value = std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| serde_json::json!({ "schemaVersion": 1, "tasks": [] }));

    if let Some(tasks) = data["tasks"].as_array_mut() {
        // Shift existing to-do tasks down so the new one appears first
        for task in tasks.iter_mut() {
            if task.get("column").and_then(|c| c.as_str()) == Some("todo") {
                if let Some(n) = task.get("order").and_then(|o| o.as_i64()) {
                    task["order"] = serde_json::json!(n + 1);
                }
            }
        }

        let ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let now = format_iso_timestamp(ms as u64 / 1000);

        tasks.push(serde_json::json!({
            "id":        format!("t_{ms}"),
            "title":     title,
            "notes":     "",
            "column":    "todo",
            "order":     0,
            "createdAt": now,
            "updatedAt": now,
        }));
    }

    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let saved = serde_json::to_string_pretty(&data)
        .map(|content| std::fs::write(&path, content).is_ok())
        .unwrap_or(false);

    if saved {
        // Tell the main window to reload its tasks
        if let Some(main_win) = app.get_webview_window("main") {
            let _ = main_win.emit("tasks-updated", ());
        }
        if let Some(qa_win) = app.get_webview_window("quick-add") {
            let _ = qa_win.close();
        }
    }

    saved
}

#[tauri::command]
fn close_quick_add(app: AppHandle) {
    if let Some(w) = app.get_webview_window("quick-add") {
        let _ = w.close();
    }
}

// ─── Local settings commands ──────────────────────────────────────────────────

#[tauri::command]
fn load_local_settings_cmd(state: State<AppState>) -> LocalSettings {
    state.local_settings.lock().unwrap().clone()
}

#[tauri::command]
fn save_local_settings_cmd(
    settings: LocalSettings,
    state: State<AppState>,
    app: AppHandle,
) -> bool {
    let path = local_settings_path(&app);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let saved = serde_json::to_string_pretty(&settings)
        .map(|content| std::fs::write(&path, content).is_ok())
        .unwrap_or(false);
    if saved {
        *state.local_settings.lock().unwrap() = settings;
    }
    saved
}

#[tauri::command]
async fn pick_sync_folder(app: AppHandle) -> Option<String> {
    use tauri_plugin_dialog::DialogExt;
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog()
        .file()
        .set_title("Select Sync Folder")
        .pick_folder(move |folder| {
            let _ = tx.send(folder);
        });
    rx.await
        .ok()
        .flatten()
        .map(|fp| fp.to_string())
}

// ─── Window state persistence ─────────────────────────────────────────────────

fn save_window_state(window: &tauri::WebviewWindow) {
    if let (Ok(pos), Ok(size)) = (window.outer_position(), window.outer_size()) {
        let data = serde_json::json!({
            "x": pos.x, "y": pos.y,
            "width": size.width, "height": size.height,
        });
        let path = window
            .app_handle()
            .path()
            .app_data_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("window-state.json");
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(content) = serde_json::to_string_pretty(&data) {
            let _ = std::fs::write(&path, content);
        }
    }
}

fn restore_window_state(app: &AppHandle) {
    let path = app
        .path()
        .app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("window-state.json");

    let Ok(content) = std::fs::read_to_string(&path) else {
        return;
    };
    let Ok(state) = serde_json::from_str::<serde_json::Value>(&content) else {
        return;
    };
    let Some(window) = app.get_webview_window("main") else {
        return;
    };

    if let (Some(x), Some(y), Some(w), Some(h)) = (
        state["x"].as_i64(),
        state["y"].as_i64(),
        state["width"].as_u64(),
        state["height"].as_u64(),
    ) {
        let _ = window.set_position(tauri::PhysicalPosition::new(x as i32, y as i32));
        let _ = window.set_size(tauri::PhysicalSize::new(w as u32, h as u32));
    }
}

// ─── Tray & shortcuts helpers ─────────────────────────────────────────────────

fn toggle_main_window(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        if w.is_visible().unwrap_or(false) {
            let _ = w.hide();
        } else {
            let _ = w.show();
            let _ = w.set_focus();
        }
    }
}

fn open_quick_add(app: &AppHandle) {
    if let Some(existing) = app.get_webview_window("quick-add") {
        let _ = existing.set_focus();
        return;
    }

    let (x, y) = app
        .get_webview_window("main")
        .and_then(|w| w.current_monitor().ok().flatten())
        .map(|m| {
            let scale = m.scale_factor();
            let lw = m.size().width as f64 / scale;
            ((lw - 400.0) / 2.0, 100.0)
        })
        .unwrap_or((300.0, 100.0));

    let _ = tauri::WebviewWindowBuilder::new(
        app,
        "quick-add",
        tauri::WebviewUrl::App("quickadd.html".into()),
    )
    .title("Quick Add")
    .inner_size(400.0, 80.0)
    .position(x, y)
    .decorations(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .transparent(true)
    .on_window_event(|window, event| {
        if let tauri::WindowEvent::Focused(false) = event {
            let _ = window.close();
        }
    })
    .build();
}

fn setup_tray(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    use tauri::image::Image;
    use tauri::menu::{Menu, MenuItem};
    use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

    let icon = Image::from_bytes(include_bytes!("../icons/icon.png"))
        .unwrap_or_else(|_| app.default_window_icon().unwrap().clone());

    let toggle = MenuItem::with_id(app, "toggle", "Show/Hide", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&toggle, &quit])?;

    TrayIconBuilder::new()
        .tooltip("TaskTracker Extreme 3000")
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
            "quit" => {
                if let Some(w) = app.get_webview_window("main") {
                    save_window_state(&w);
                }
                app.exit(0);
            }
            _ => {}
        })
        .build(app)?;

    Ok(())
}

// ─── Minimal ISO 8601 timestamp (no external crate needed) ───────────────────

fn format_iso_timestamp(unix_secs: u64) -> String {
    let s = unix_secs % 60;
    let m = (unix_secs / 60) % 60;
    let h = (unix_secs / 3600) % 24;

    // Gregorian calendar from Julian day number (Euclidean algorithm)
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

// ─── Entry point ──────────────────────────────────────────────────────────────

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // If a second instance is launched, focus the existing window
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.show();
                let _ = w.set_focus();
            }
        }))
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_global_shortcut::init())
        .setup(|app| {
            // On macOS, run as a tray/menu-bar-only app — no Dock icon.
            // The sidebar is accessed via the tray icon or global shortcuts.
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            // Load local (machine-specific) settings
            let settings = read_local_settings(app.handle());
            app.manage(AppState {
                local_settings: Mutex::new(settings),
            });

            // Set up system tray
            setup_tray(app)?;

            // Register global shortcuts
            use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
            let handle1 = app.handle().clone();
            let handle2 = app.handle().clone();

            app.handle().global_shortcut().on_shortcut(
                "CommandOrControl+Shift+T",
                move |_app, _shortcut, event| {
                    if event.state == ShortcutState::Pressed {
                        toggle_main_window(&handle1);
                    }
                },
            )?;

            app.handle().global_shortcut().on_shortcut(
                "CommandOrControl+Shift+N",
                move |_app, _shortcut, event| {
                    if event.state == ShortcutState::Pressed {
                        open_quick_add(&handle2);
                    }
                },
            )?;

            // Restore window position from last session
            restore_window_state(app.handle());

            Ok(())
        })
        .on_window_event(|window, event| match event {
            // Intercept close on the main window — hide to tray instead
            tauri::WindowEvent::CloseRequested { api, .. } => {
                if window.label() == "main" {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
            // Persist window position/size whenever the user moves or resizes
            tauri::WindowEvent::Moved(_) | tauri::WindowEvent::Resized(_) => {
                if window.label() == "main" {
                    save_window_state(window);
                }
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            load_file,
            save_file,
            window_minimize,
            hide_window,
            toggle_always_on_top,
            open_external,
            fetch_tickets,
            show_notification,
            quick_add_task,
            close_quick_add,
            load_local_settings_cmd,
            save_local_settings_cmd,
            pick_sync_folder,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
