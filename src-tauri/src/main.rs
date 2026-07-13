mod config;
mod desktop_entries;
mod launcher;
mod panel_actions;
mod panel_model;
mod panel_window;
mod popup_windows;
mod system_actions;
mod window_manager;

use base64::Engine;
use config::{ensure_config, print_config_diagnostics};
use desktop_entries::{scan_desktop_apps, DesktopApp};
use launcher::{
    clean_exec_command, resolve_desktop_dir, run_short_command, spawn_detached,
    spawn_detached_shell, spawn_detached_with_fallbacks,
};
use panel_actions::{ActionResult, PanelAction};
use panel_model::PanelGeometry;
use panel_window::{configure_main_window, MAIN_WINDOW_MIN_HEIGHT, MAIN_WINDOW_MIN_WIDTH};
use popup_windows::{
    ensure_menu_flyout_window, ensure_menu_popup_window, hide_menu_flyout_window,
    hide_menu_popup_window, log_popup_actual_size, reset_popup_gtk_size, resize_menu_popup_window,
    FLYOUT_MENU_POPUP_LABEL, PRIMARY_MENU_POPUP_LABEL,
};
use serde::Serialize;
use std::{env, fs, path::PathBuf};
use tauri::{Emitter, Manager};
use window_manager::{activate_remembered_window, WindowMemory};

#[derive(Debug, Clone, Serialize)]
struct SelectedMenuAction {
    label: String,
    action: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
struct BuildInfo {
    commit: String,
    branch: String,
    built_at: String,
    dirty: bool,
}

#[derive(Debug, Clone, Serialize)]
struct PanelState {
    config: config::PanelConfig,
    geometry: PanelGeometry,
}

fn main() {
    print_build_info();
    print_tauri_asset_diagnostics();
    tauri::Builder::default()
        .manage(WindowMemory::default())
        .setup(|app| {
            let config = ensure_config().map_err(|err| err.to_string())?;
            print_config_diagnostics(&config)?;
            if let Some(window) = app.get_webview_window("main") {
                configure_main_window(&window, &config, "setup")?;
            }
            ensure_menu_popup_window(app.handle())?;
            ensure_menu_flyout_window(app.handle())?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_build_info,
            get_config,
            get_panel_state,
            get_apple_logo_data_url,
            initialize_main_window,
            frontend_log,
            open_menu_popup,
            open_menu_flyout,
            close_menu_popup,
            close_menu_flyout,
            menu_popup_rendered,
            menu_flyout_rendered,
            menu_flyout_pointer_entered,
            select_menu_action,
            list_applications,
            list_control_panels,
            launch_app,
            launch_calculator,
            open_folder,
            new_terminal_window,
            remember_active_window,
            show_about_piforma,
            send_shortcut,
            run_system_action,
            confirm_system_action,
            confirm_system_action_result,
            execute_panel_action,
        ])
        .run(tauri::generate_context!())
        .expect("error while running piforma-panel");
}

fn build_info() -> BuildInfo {
    BuildInfo {
        commit: option_env!("PIFORMA_BUILD_COMMIT")
            .unwrap_or("unknown")
            .to_string(),
        branch: option_env!("PIFORMA_BUILD_BRANCH")
            .unwrap_or("unknown")
            .to_string(),
        built_at: option_env!("PIFORMA_BUILD_BUILT_AT")
            .unwrap_or("unknown")
            .to_string(),
        dirty: option_env!("PIFORMA_BUILD_DIRTY").unwrap_or("false") == "true",
    }
}

fn print_build_info() {
    let info = build_info();
    println!(
        "PiForma Panel build info: commit={}, branch={}, dirty={}, built_at={}",
        info.commit, info.branch, info.dirty, info.built_at
    );
}

fn print_tauri_asset_diagnostics() {
    let config = serde_json::from_str::<serde_json::Value>(include_str!("../tauri.conf.json"));
    match config {
        Ok(config) => {
            let frontend_dist = config
                .pointer("/build/frontendDist")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("missing");
            let csp = config
                .pointer("/app/security/csp")
                .map(|value| {
                    if value.is_null() {
                        "null".to_string()
                    } else {
                        value.to_string()
                    }
                })
                .unwrap_or_else(|| "missing".to_string());
            println!(
                "tauri asset config: frontendDist={frontend_dist}, frontendDist_is_bundled_dist={}, app.security.csp={csp}",
                frontend_dist == "../dist"
            );
        }
        Err(err) => {
            eprintln!("tauri asset config: failed to parse tauri.conf.json: {err}");
        }
    }
}

#[tauri::command]
fn get_build_info() -> BuildInfo {
    build_info()
}

#[tauri::command]
fn get_config() -> Result<config::PanelConfig, String> {
    ensure_config()
}

#[tauri::command]
fn get_panel_state(app: tauri::AppHandle) -> Result<PanelState, String> {
    let config = ensure_config()?;
    let geometry =
        panel_window::effective_panel_geometry(app.get_webview_window("main").as_ref(), &config)?;
    Ok(PanelState { config, geometry })
}

#[tauri::command]
fn initialize_main_window(app: tauri::AppHandle) -> Result<(), String> {
    let config = ensure_config()?;

    println!(
        "startup main panel set to bar-only: width={}, height={}, x={}, y={}",
        config.bar.width, config.bar.height, config.bar.x, config.bar.y
    );

    if let Some(window) = app.get_webview_window("main") {
        configure_main_window(&window, &config, "frontend-init")?;
    }

    Ok(())
}

#[tauri::command]
fn frontend_log(message: String) {
    println!("frontend: {message}");
}

#[tauri::command]
async fn open_menu_popup(
    app: tauri::AppHandle,
    label: String,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    items: serde_json::Value,
) -> Result<(), String> {
    println!("open_menu_popup start: label={label}, x={x}, y={y}, width={width}, height={height}");
    let popup = ensure_menu_popup_window(&app)?;

    popup.hide().map_err(|err| err.to_string())?;
    hide_menu_flyout_window(&app);
    reset_popup_gtk_size(&popup)?;
    popup.set_resizable(true).map_err(|err| err.to_string())?;
    popup
        .set_min_size(Some(tauri::Size::Physical(tauri::PhysicalSize {
            width: MAIN_WINDOW_MIN_WIDTH,
            height: MAIN_WINDOW_MIN_HEIGHT,
        })))
        .map_err(|err| err.to_string())?;
    popup
        .set_max_size(Option::<tauri::Size>::None)
        .map_err(|err| err.to_string())?;
    popup
        .set_position(tauri::Position::Physical(tauri::PhysicalPosition { x, y }))
        .map_err(|err| err.to_string())?;
    println!(
        "primary menu popup requested before render: label={label}, x={x}, y={y}, width={width}, height={height}"
    );
    popup
        .emit(
            "render-menu-popup",
            serde_json::json!({ "label": label, "items": items, "width": width, "height": height, "x": x, "y": y }),
        )
        .map_err(|err| err.to_string())?;

    Ok(())
}

#[tauri::command]
async fn open_menu_flyout(
    app: tauri::AppHandle,
    label: String,
    submenu: String,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    items: serde_json::Value,
) -> Result<(), String> {
    let item_count = items.as_array().map_or(0, Vec::len);
    println!(
        "flyout requested: label={label}, submenu={submenu}, x={x}, y={y}, width={width}, height={height}, item_count={item_count}"
    );
    let flyout = ensure_menu_flyout_window(&app)?;

    flyout.hide().map_err(|err| err.to_string())?;
    reset_popup_gtk_size(&flyout)?;
    flyout.set_resizable(true).map_err(|err| err.to_string())?;
    flyout
        .set_min_size(Some(tauri::Size::Physical(tauri::PhysicalSize {
            width: MAIN_WINDOW_MIN_WIDTH,
            height: MAIN_WINDOW_MIN_HEIGHT,
        })))
        .map_err(|err| err.to_string())?;
    flyout
        .set_max_size(Option::<tauri::Size>::None)
        .map_err(|err| err.to_string())?;
    flyout
        .set_position(tauri::Position::Physical(tauri::PhysicalPosition { x, y }))
        .map_err(|err| err.to_string())?;
    flyout
        .emit(
            "render-menu-flyout",
            serde_json::json!({ "label": label, "submenu": submenu, "items": items, "width": width, "height": height, "x": x, "y": y }),
        )
        .map_err(|err| err.to_string())?;

    Ok(())
}

#[tauri::command]
fn close_menu_popup(app: tauri::AppHandle) -> Result<(), String> {
    hide_menu_popup_window(&app, true);
    Ok(())
}

#[tauri::command]
fn close_menu_flyout(app: tauri::AppHandle) -> Result<(), String> {
    hide_menu_flyout_window(&app);
    Ok(())
}

#[tauri::command]
fn menu_popup_rendered(
    app: tauri::AppHandle,
    label: String,
    width: u32,
    height: u32,
) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(PRIMARY_MENU_POPUP_LABEL) {
        println!(
            "primary menu popup requested after render: label={label}, width={width}, height={height}"
        );
        resize_menu_popup_window(&window, width, height)?;
        window.set_focus().map_err(|err| err.to_string())?;
        window.set_resizable(false).map_err(|err| err.to_string())?;
        window.show().map_err(|err| err.to_string())?;
        println!("primary menu popup shown: label={label}");
        log_popup_actual_size(&window, PRIMARY_MENU_POPUP_LABEL, "final", width, height);
    }
    Ok(())
}

#[tauri::command]
fn menu_flyout_rendered(
    app: tauri::AppHandle,
    label: String,
    width: u32,
    height: u32,
) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(FLYOUT_MENU_POPUP_LABEL) {
        println!("flyout rendered: label={label}, width={width}, height={height}");
        resize_menu_popup_window(&window, width, height)?;
        window.set_resizable(false).map_err(|err| err.to_string())?;
        window.show().map_err(|err| err.to_string())?;
        if let Err(err) = app.emit("menu-flyout-rendered", ()) {
            eprintln!("failed to emit menu-flyout-rendered: {err}");
        }
        println!("flyout shown: label={label}, width={width}, height={height}");
        log_popup_actual_size(&window, FLYOUT_MENU_POPUP_LABEL, "final", width, height);
    }
    Ok(())
}

#[tauri::command]
fn menu_flyout_pointer_entered(app: tauri::AppHandle) {
    if let Err(err) = app.emit("menu-flyout-entered", ()) {
        eprintln!("failed to emit menu-flyout-entered: {err}");
    }
}

#[tauri::command]
fn select_menu_action(
    app: tauri::AppHandle,
    label: String,
    action: serde_json::Value,
) -> Result<(), String> {
    println!("selected menu action: {label}");
    if action
        .get("kind")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|kind| kind == "launch_app")
    {
        println!("flyout application selected: {label}");
    }
    hide_menu_popup_window(&app, true);
    app.emit("menu-action-selected", SelectedMenuAction { label, action })
        .map_err(|err| err.to_string())
}

#[tauri::command]
fn get_apple_logo_data_url() -> Result<Option<String>, String> {
    let config = ensure_config()?;
    let path = PathBuf::from(config.apple.logo_path);
    if !path.exists() {
        return Ok(None);
    }

    let bytes = fs::read(&path).map_err(|err| err.to_string())?;
    let mime = match path.extension().and_then(|ext| ext.to_str()) {
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("svg") => "image/svg+xml",
        _ => "image/png",
    };
    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
    Ok(Some(format!("data:{mime};base64,{encoded}")))
}

#[tauri::command]
fn list_applications() -> Result<Vec<DesktopApp>, String> {
    let config = ensure_config()?;
    Ok(scan_desktop_apps(&config)?
        .into_iter()
        .filter(|app| !app.is_control_panel)
        .collect())
}

#[tauri::command]
fn list_control_panels() -> Result<Vec<DesktopApp>, String> {
    let config = ensure_config()?;
    Ok(scan_desktop_apps(&config)?
        .into_iter()
        .filter(|app| app.is_control_panel)
        .collect())
}

#[tauri::command]
fn launch_app(exec: String, name: String) -> Result<(), String> {
    let command = clean_exec_command(&exec, &name);
    spawn_detached_shell(&command)
}

#[tauri::command]
fn launch_calculator() -> Result<(), String> {
    spawn_detached_with_fallbacks(&[
        vec!["xcalc".to_string()],
        vec!["galculator".to_string()],
        vec!["gnome-calculator".to_string()],
        vec!["mate-calc".to_string()],
        vec!["kcalc".to_string()],
    ])
}

#[tauri::command]
fn open_folder(folder: String) -> Result<(), String> {
    let target = match folder.as_str() {
        "applications" => {
            let home = env::var("HOME").map_err(|_| "HOME is not set".to_string())?;
            let path = PathBuf::from(home).join(".local/share/applications");
            fs::create_dir_all(&path)
                .map_err(|err| format!("failed to create {}: {err}", path.display()))?;
            path.display().to_string()
        }
        "home" => env::var("HOME").map_err(|_| "HOME is not set".to_string())?,
        "desktop" => {
            let home = env::var("HOME").map_err(|_| "HOME is not set".to_string())?;
            let path = resolve_desktop_dir(&home)?;
            path.display().to_string()
        }
        _ => return Err(format!("unknown folder: {folder}")),
    };

    spawn_detached_with_fallbacks(&[
        vec!["xdg-open".to_string(), target.clone()],
        vec!["gio".to_string(), "open".to_string(), target.clone()],
        vec!["pcmanfm".to_string(), target],
    ])
}

#[tauri::command]
fn new_terminal_window() -> Result<(), String> {
    spawn_detached_with_fallbacks(&[
        vec!["x-terminal-emulator".to_string()],
        vec!["lxterminal".to_string()],
        vec!["xfce4-terminal".to_string()],
        vec!["gnome-terminal".to_string()],
        vec!["konsole".to_string()],
        vec!["mate-terminal".to_string()],
    ])
}

#[tauri::command]
fn remember_active_window(
    app: tauri::AppHandle,
    memory: tauri::State<WindowMemory>,
) -> Result<(), String> {
    window_manager::remember_active_window(&app, &memory)
}

#[tauri::command]
fn show_about_piforma() -> Result<(), String> {
    spawn_detached("about-piforma", &[])
}

#[tauri::command]
fn send_shortcut(
    app: tauri::AppHandle,
    memory: tauri::State<WindowMemory>,
    action: String,
) -> Result<(), String> {
    let key = match action.as_str() {
        "undo" => "ctrl+z",
        "cut" => "ctrl+x",
        "copy" => "ctrl+c",
        "paste" => "ctrl+v",
        "clear" => "Delete",
        "select_all" => "ctrl+a",
        _ => return Err(format!("unknown shortcut action: {action}")),
    };

    hide_menu_popup_window(&app, true);
    activate_remembered_window(&memory)?;
    run_short_command("xdotool", &["key", key])
}

#[tauri::command]
fn run_system_action(
    app: tauri::AppHandle,
    memory: tauri::State<WindowMemory>,
    action: String,
    confirmed: bool,
) -> Result<(), String> {
    system_actions::run_system_action(&app, &memory, &action, confirmed)
}

#[tauri::command]
fn confirm_system_action(app: tauri::AppHandle, action: String) -> Result<(), String> {
    system_actions::confirm_system_action(&app, &action)
}

#[tauri::command]
fn confirm_system_action_result(app: tauri::AppHandle, action: String) -> ActionResult {
    system_actions::confirm_system_action_result(&app, &action)
}

#[tauri::command]
fn execute_panel_action(
    app: tauri::AppHandle,
    memory: tauri::State<WindowMemory>,
    action: PanelAction,
) -> ActionResult {
    panel_actions::execute_panel_action(&app, &memory, action)
}
