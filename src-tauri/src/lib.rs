// Prevents additional console window on Windows in release builds
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod clipboard;
mod config;
mod database;

use clipboard::{ClipboardMonitor, ClipboardSnapshot};
use config::Config;
use database::{ClipboardItem, Database};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Listener, Manager, State, WindowEvent, Wry};
use tauri::menu::{CheckMenuItem, CheckMenuItemBuilder, MenuBuilder, MenuItem, MenuItemBuilder};
use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri_plugin_autostart::ManagerExt;

const TRAY_OPEN_MAIN: &str = "open-main";
const TRAY_OPEN_SETTINGS: &str = "open-settings";
const TRAY_TOGGLE_THEME: &str = "toggle-theme";
const TRAY_TOGGLE_AUTOSTART: &str = "toggle-autostart";
const TRAY_QUIT: &str = "quit";

struct TrayHandles {
    _icon: TrayIcon<Wry>,
    theme_item: MenuItem<Wry>,
    autostart_item: CheckMenuItem<Wry>,
}

fn theme_display_label(theme: &str) -> &'static str {
    match theme {
        "light" => "浅色",
        "dark" => "深色",
        _ => "自动",
    }
}

fn theme_menu_label(theme: &str) -> String {
    format!("切换主题（当前：{}）", theme_display_label(theme))
}

fn focus_main_window(app: &AppHandle<Wry>) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.set_skip_taskbar(false);
        let _ = window.show();
        let _ = window.set_focus();
    }
}

/// 应用状态
struct AppState {
    db: Arc<Database>,
    config: Arc<Mutex<Config>>,
    _clipboard_monitor: Arc<ClipboardMonitor>,
    tray_handles: Arc<Mutex<Option<TrayHandles>>>,
}

/// 获取历史记录列表
#[tauri::command]
async fn get_history(
    state: State<'_, AppState>,
    limit: i64,
    offset: i64,
) -> Result<Vec<ClipboardItem>, String> {
    state
        .db
        .get_items(limit, offset)
        .map_err(|e| e.to_string())
}

/// 搜索历史记录
#[tauri::command]
async fn search_history(
    state: State<'_, AppState>,
    query: String,
    limit: i64,
) -> Result<Vec<ClipboardItem>, String> {
    state
        .db
        .search_items(&query, limit)
        .map_err(|e| e.to_string())
}

/// 添加剪切板记录
#[tauri::command]
async fn add_clipboard_item(
    state: State<'_, AppState>,
    content_type: String,
    content: String,
    preview: String,
) -> Result<i64, String> {
    let id = state
        .db
        .add_item(&content_type, &content, &preview)
        .map_err(|e| e.to_string())?;

    // 维护历史记录数量上限
    let config = state.config.lock().unwrap();
    state
        .db
        .maintain_limit(config.max_history_items)
        .map_err(|e| e.to_string())?;

    Ok(id)
}

/// 切换收藏状态
#[tauri::command]
async fn toggle_favorite(state: State<'_, AppState>, id: i64) -> Result<bool, String> {
    state.db.toggle_favorite(id).map_err(|e| e.to_string())
}

/// 删除记录
#[tauri::command]
async fn delete_item(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    state.db.delete_item(id).map_err(|e| e.to_string())
}

/// 清空非收藏记录
#[tauri::command]
async fn clear_history(state: State<'_, AppState>) -> Result<(), String> {
    state
        .db
        .clear_non_favorites()
        .map_err(|e| e.to_string())
}

/// 复制到剪切板
#[tauri::command]
async fn copy_to_clipboard(content: String) -> Result<(), String> {
    ClipboardMonitor::set_clipboard_text(&content).map_err(|e| e.to_string())
}

/// 添加标签
#[tauri::command]
async fn add_tag(
    state: State<'_, AppState>,
    item_id: i64,
    tag_name: String,
) -> Result<(), String> {
    state
        .db
        .add_item_tag(item_id, &tag_name)
        .map_err(|e| e.to_string())
}

/// 移除标签
#[tauri::command]
async fn remove_tag(
    state: State<'_, AppState>,
    item_id: i64,
    tag_name: String,
) -> Result<(), String> {
    state
        .db
        .remove_item_tag(item_id, &tag_name)
        .map_err(|e| e.to_string())
}

/// 获取所有标签
#[tauri::command]
async fn get_all_tags(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    state.db.get_all_tags().map_err(|e| e.to_string())
}

/// 按标签获取项目
#[tauri::command]
async fn get_items_by_tag(
    state: State<'_, AppState>,
    tag_name: String,
    limit: i64,
) -> Result<Vec<ClipboardItem>, String> {
    state
        .db
        .get_items_by_tag(&tag_name, limit)
        .map_err(|e| e.to_string())
}

/// 获取配置
#[tauri::command]
async fn get_config(state: State<'_, AppState>) -> Result<Config, String> {
    let config = state.config.lock().unwrap();
    Ok(config.clone())
}

/// 更新配置
#[tauri::command]
async fn update_config(
    state: State<'_, AppState>,
    new_config: Config,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let sanitized = new_config.clone().sanitized();
    let config_path = app_handle
        .path()
        .app_config_dir()
        .map_err(|e| e.to_string())?
        .join("config.json");

    sanitized
        .save(config_path)
        .map_err(|e| e.to_string())?;

    {
        let mut config = state.config.lock().unwrap();
        *config = sanitized.clone();
    }

    if let Ok(handles_guard) = state.tray_handles.lock() {
        if let Some(handles) = handles_guard.as_ref() {
            let _ = handles
                .autostart_item
                .set_checked(sanitized.auto_start);
            let _ = handles
                .theme_item
                .set_text(theme_menu_label(&sanitized.theme));
        }
    }

    Ok(())
}

/// 更新开机自启设置并返回最新配置
#[tauri::command]
async fn set_autostart(
    state: State<'_, AppState>,
    enabled: bool,
    app_handle: tauri::AppHandle,
) -> Result<Config, String> {
    if enabled {
        app_handle
            .autolaunch()
            .enable()
            .map_err(|e| e.to_string())?;
    } else {
        app_handle
            .autolaunch()
            .disable()
            .map_err(|e| e.to_string())?;
    }

    let config_path = app_handle
        .path()
        .app_config_dir()
        .map_err(|e| e.to_string())?
        .join("config.json");

    let updated = {
        let mut config = state.config.lock().unwrap();
        config.auto_start = enabled;
        config
            .save(config_path)
            .map_err(|e| e.to_string())?;
        config.clone()
    };

    if let Ok(handles_guard) = state.tray_handles.lock() {
        if let Some(handles) = handles_guard.as_ref() {
            let _ = handles.autostart_item.set_checked(enabled);
        }
    }

    Ok(updated)
}

/// 重置应用数据
#[tauri::command]
async fn reset_application(
    state: State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<Config, String> {
    state
        .db
        .reset_all()
        .map_err(|e| e.to_string())?;

    if let Err(err) = app_handle.autolaunch().disable() {
        eprintln!("Failed to disable autostart during reset: {err:?}");
    }

    let default_config = Config::default().sanitized();
    let config_path = app_handle
        .path()
        .app_config_dir()
        .map_err(|e| e.to_string())?
        .join("config.json");

    default_config
        .save(config_path)
        .map_err(|e| e.to_string())?;

    {
        let mut config_guard = state.config.lock().unwrap();
        *config_guard = default_config.clone();
    }

    if let Ok(handles_guard) = state.tray_handles.lock() {
        if let Some(handles) = handles_guard.as_ref() {
            let _ = handles.autostart_item.set_checked(default_config.auto_start);
            let _ = handles
                .theme_item
                .set_text(theme_menu_label(&default_config.theme));
        }
    }

    Ok(default_config)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
    .plugin(tauri_plugin_clipboard_manager::init())
    .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .setup(|app| {
            // 初始化数据路径
            let app_data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&app_data_dir)?;

            let db_path = app_data_dir.join("clipboard.db");
            let config_path = app_data_dir.join("config.json");

            // 初始化数据库与配置
            let db = Arc::new(Database::new(db_path)?);
            let config = Arc::new(Mutex::new(Config::load(config_path)?));

            // 初始化剪切板监听器
            let clipboard_monitor = Arc::new(ClipboardMonitor::new());
            let tray_handles: Arc<Mutex<Option<TrayHandles>>> = Arc::new(Mutex::new(None));

            // 启动剪切板监听
            let app_handle = app.handle().clone();
            clipboard_monitor.start(app_handle.clone());

            // 注册剪切板变化事件处理器
            let db_for_event = Arc::clone(&db);
            let config_for_event = Arc::clone(&config);
            let notify_handle = app_handle.clone();

            app.listen("clipboard-changed", move |event| {
                let payload = event.payload();
                match serde_json::from_str::<ClipboardSnapshot>(payload) {
                    Ok(snapshot) => {
                        if let Ok(id) = db_for_event
                            .add_item(&snapshot.content_type, &snapshot.content, &snapshot.preview)
                        {
                            if let Ok(cfg) = config_for_event.lock() {
                                if let Err(err) = db_for_event.maintain_limit(cfg.max_history_items) {
                                    eprintln!("Failed to enforce history limit: {err:?}");
                                }
                            }

                            if let Err(err) = notify_handle.emit("history-updated", id) {
                                eprintln!("Failed to emit history-updated event: {err:?}");
                            }
                            println!("Captured clipboard item #{id} ({})", snapshot.content_type);
                        }
                    }
                    Err(err) => {
                        eprintln!("Failed to parse clipboard payload: {err:?} -> {payload}");
                    }
                }
            });

            // 构建系统托盘
            {
                let initial_config = {
                    let guard = config.lock().unwrap();
                    guard.clone()
                };

                let open_main_item = MenuItemBuilder::with_id(TRAY_OPEN_MAIN, "打开 Cat History")
                    .build(&app_handle)?;
                let open_settings_item =
                    MenuItemBuilder::with_id(TRAY_OPEN_SETTINGS, "打开设置").build(&app_handle)?;
                let theme_item = MenuItemBuilder::with_id(
                    TRAY_TOGGLE_THEME,
                    theme_menu_label(&initial_config.theme),
                )
                .build(&app_handle)?;
                let autostart_item = CheckMenuItemBuilder::with_id(
                    TRAY_TOGGLE_AUTOSTART,
                    "开机自启",
                )
                .checked(initial_config.auto_start)
                .build(&app_handle)?;
                let quit_item = MenuItemBuilder::with_id(TRAY_QUIT, "退出").build(&app_handle)?;

                let tray_menu = MenuBuilder::new(&app_handle)
                    .item(&open_main_item)
                    .item(&open_settings_item)
                    .item(&theme_item)
                    .item(&autostart_item)
                    .separator()
                    .item(&quit_item)
                    .build()?;

                let menu_tray_handles = Arc::clone(&tray_handles);

                let mut tray_builder = TrayIconBuilder::new()
                    .menu(&tray_menu)
                    .tooltip("Cat History")
                    .on_menu_event(|app, event| match event.id().as_ref() {
                        TRAY_OPEN_MAIN => {
                            focus_main_window(app);
                            let _ = app.emit("tray-open-main", ());
                        }
                        TRAY_OPEN_SETTINGS => {
                            focus_main_window(app);
                            let _ = app.emit("tray-open-settings", ());
                        }
                        TRAY_TOGGLE_THEME => {
                            let _ = app.emit("tray-toggle-theme", ());
                        }
                        TRAY_TOGGLE_AUTOSTART => {
                            let _ = app.emit("tray-toggle-autostart", ());
                        }
                        TRAY_QUIT => app.exit(0),
                        _ => {}
                    })
                    .on_tray_icon_event(|icon, event| {
                        match event {
                            TrayIconEvent::DoubleClick { .. } => {
                                focus_main_window(icon.app_handle());
                                let _ = icon.app_handle().emit("tray-open-main", ());
                            }
                            TrayIconEvent::Click {
                                button: MouseButton::Left,
                                button_state: MouseButtonState::Up,
                                ..
                            } => {
                                focus_main_window(icon.app_handle());
                                let _ = icon.app_handle().emit("tray-open-main", ());
                            }
                            _ => {}
                        }
                    });

                if let Some(icon_image) = app.default_window_icon().cloned() {
                    tray_builder = tray_builder.icon(icon_image);
                }

                let tray_icon = tray_builder.build(&app_handle)?;

                let mut guard = menu_tray_handles
                    .lock()
                    .expect("tray handles mutex poisoned");
                *guard = Some(TrayHandles {
                    _icon: tray_icon,
                    theme_item,
                    autostart_item,
                });
            }

            // 保存状态
            app.manage(AppState {
                db,
                config,
                _clipboard_monitor: clipboard_monitor,
                tray_handles,
            });

            if let Some(main_window) = app.get_webview_window("main") {
                let window_handle = main_window.clone();
                main_window.on_window_event(move |event| {
                    if let WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = window_handle.hide();
                        let _ = window_handle.set_skip_taskbar(true);
                    }
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_history,
            search_history,
            add_clipboard_item,
            toggle_favorite,
            delete_item,
            clear_history,
            copy_to_clipboard,
            add_tag,
            remove_tag,
            get_all_tags,
            get_items_by_tag,
            get_config,
            update_config,
            set_autostart,
            reset_application,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
