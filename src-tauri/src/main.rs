use base64::Engine;
use gtk::prelude::{ContainerExt, GtkWindowExt, WidgetExt};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};
use tauri::{Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

const PRIMARY_MENU_POPUP_LABEL: &str = "menu-popup";
const MAIN_WINDOW_MIN_WIDTH: u32 = 1;
const MAIN_WINDOW_MIN_HEIGHT: u32 = 1;

const DEFAULT_CONFIG: &str = r#"bar:
  width: 658
  height: 20
  x: 76
  y: 0
  radius_top_left: 18
  radius_top_right: 18
  font_family: ChicagoFLF
  font_size: 13

apple:
  logo_path: /home/frost/.local/share/piforma-panel/apple-color.png

clock:
  enabled: true
  format: "%I:%M %p"

applications:
  scan_dirs:
    - /home/frost/.local/share/applications
    - /usr/local/share/applications
    - /usr/share/applications
  show_no_display: false
  group_by_categories: true
  show_category_labels: false
  max_menu_height: 420

menus:
  show_file: true
  show_edit: true
  show_view: true
  show_special: true
"#;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PanelConfig {
    bar: BarConfig,
    apple: AppleConfig,
    clock: ClockConfig,
    applications: ApplicationsConfig,
    menus: MenusConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BarConfig {
    width: u32,
    height: u32,
    x: i32,
    y: i32,
    radius_top_left: u32,
    radius_top_right: u32,
    font_family: String,
    font_size: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AppleConfig {
    logo_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ClockConfig {
    enabled: bool,
    format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ApplicationsConfig {
    scan_dirs: Vec<String>,
    show_no_display: bool,
    group_by_categories: bool,
    show_category_labels: bool,
    max_menu_height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MenusConfig {
    show_file: bool,
    show_edit: bool,
    show_view: bool,
    show_special: bool,
}

#[derive(Debug, Clone, Serialize)]
struct DesktopApp {
    id: String,
    name: String,
    exec: String,
    icon: Option<String>,
    categories: Vec<String>,
    group: String,
    is_control_panel: bool,
}

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

#[derive(Debug, Clone, Copy)]
enum SystemAction {
    SleepDisplay,
    ShowDesktop,
    Refresh,
    CleanUpWindow,
    Restart,
    ShutDown,
    ShowClipboard,
}

fn main() {
    print_build_info();
    print_tauri_asset_diagnostics();
    tauri::Builder::default()
        .setup(|app| {
            let config = ensure_config().map_err(|err| err.to_string())?;
            print_config_diagnostics(&config)?;
            if let Some(window) = app.get_webview_window("main") {
                configure_main_window(&window, &config, "setup")?;
            }
            ensure_menu_popup_window(app.handle())?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_build_info,
            get_config,
            get_apple_logo_data_url,
            initialize_main_window,
            frontend_log,
            open_menu_popup,
            close_menu_popup,
            menu_popup_rendered,
            select_menu_action,
            list_applications,
            list_control_panels,
            launch_app,
            open_folder,
            new_terminal_window,
            send_shortcut,
            run_system_action,
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

fn config_path() -> Result<PathBuf, String> {
    let home = env::var("HOME").map_err(|_| "HOME is not set".to_string())?;
    Ok(PathBuf::from(home).join(".local/share/piforma-panel/config.yaml"))
}

fn ensure_config() -> Result<PanelConfig, String> {
    let path = config_path()?;
    if !path.exists() {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|err| err.to_string())?;
        }
        fs::write(&path, DEFAULT_CONFIG).map_err(|err| err.to_string())?;
    }

    let contents = fs::read_to_string(&path).map_err(|err| err.to_string())?;
    serde_yaml::from_str(&contents).map_err(|err| format!("invalid config.yaml: {err}"))
}

fn configure_main_window(
    window: &tauri::WebviewWindow,
    config: &PanelConfig,
    phase: &str,
) -> Result<(), String> {
    window.set_resizable(true).map_err(|err| err.to_string())?;
    window
        .set_min_size(Some(tauri::Size::Physical(tauri::PhysicalSize {
            width: MAIN_WINDOW_MIN_WIDTH,
            height: MAIN_WINDOW_MIN_HEIGHT,
        })))
        .map_err(|err| err.to_string())?;
    window
        .set_max_size(Option::<tauri::Size>::None)
        .map_err(|err| err.to_string())?;
    window
        .set_size(tauri::Size::Physical(tauri::PhysicalSize {
            width: config.bar.width,
            height: config.bar.height,
        }))
        .map_err(|err| err.to_string())?;
    apply_tight_gtk_size(window, config.bar.width, config.bar.height, "main")?;
    window
        .set_position(tauri::Position::Physical(tauri::PhysicalPosition {
            x: config.bar.x,
            y: config.bar.y,
        }))
        .map_err(|err| err.to_string())?;
    log_main_window_actual_size(window, phase);
    window.show().map_err(|err| err.to_string())?;
    window.set_resizable(false).map_err(|err| err.to_string())
}

fn apply_tight_gtk_size(
    window: &tauri::WebviewWindow,
    width: u32,
    height: u32,
    label: &str,
) -> Result<(), String> {
    let width_i32 = i32::try_from(width).map_err(|err| err.to_string())?;
    let height_i32 = i32::try_from(height).map_err(|err| err.to_string())?;
    let gtk_window = window.gtk_window().map_err(|err| err.to_string())?;
    gtk_window.set_size_request(width_i32, height_i32);
    gtk_window.set_default_size(width_i32, height_i32);
    gtk_window.resize(width_i32, height_i32);

    if let Ok(vbox) = window.default_vbox() {
        vbox.set_size_request(width_i32, height_i32);
        for child in vbox.children() {
            child.set_size_request(width_i32, height_i32);
        }
    }

    println!("gtk tight size applied: label={label}, width={width}, height={height}");
    Ok(())
}

fn log_main_window_actual_size(window: &tauri::WebviewWindow, phase: &str) {
    println!(
        "main window {phase}: actual inner_size={}; actual outer_size={}",
        format_size_result(window.inner_size()),
        format_size_result(window.outer_size())
    );
}

fn format_size_result(size: tauri::Result<tauri::PhysicalSize<u32>>) -> String {
    match size {
        Ok(size) => format!("{}x{}", size.width, size.height),
        Err(err) => format!("unavailable ({err})"),
    }
}

fn print_config_diagnostics(config: &PanelConfig) -> Result<(), String> {
    let path = config_path()?;
    println!("config path: {}", path.display());
    println!(
        "config bar: width={}, height={}, x={}, y={}",
        config.bar.width, config.bar.height, config.bar.x, config.bar.y
    );
    Ok(())
}

#[tauri::command]
fn get_build_info() -> BuildInfo {
    build_info()
}

#[tauri::command]
fn get_config() -> Result<PanelConfig, String> {
    ensure_config()
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
        .set_size(tauri::Size::Physical(tauri::PhysicalSize { width, height }))
        .map_err(|err| err.to_string())?;
    apply_tight_gtk_size(&popup, width, height, PRIMARY_MENU_POPUP_LABEL)?;
    popup
        .set_position(tauri::Position::Physical(tauri::PhysicalPosition { x, y }))
        .map_err(|err| err.to_string())?;
    println!(
        "primary menu popup updated: label={label}, x={x}, y={y}, width={width}, height={height}"
    );
    log_popup_actual_size(&popup, "updated");
    popup
        .emit(
            "render-menu-popup",
            serde_json::json!({ "label": label, "items": items }),
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
fn menu_popup_rendered(app: tauri::AppHandle, label: String) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(PRIMARY_MENU_POPUP_LABEL) {
        println!("primary menu popup rendered: label={label}");
        window.show().map_err(|err| err.to_string())?;
        window.set_focus().map_err(|err| err.to_string())?;
        println!("primary menu popup shown: label={label}");
        log_popup_actual_size(&window, "shown");
    }
    Ok(())
}

#[tauri::command]
fn select_menu_action(
    app: tauri::AppHandle,
    label: String,
    action: serde_json::Value,
) -> Result<(), String> {
    println!("selected menu action: {label}");
    hide_menu_popup_window(&app, true);
    app.emit("menu-action-selected", SelectedMenuAction { label, action })
        .map_err(|err| err.to_string())
}

fn ensure_menu_popup_window(app: &tauri::AppHandle) -> Result<tauri::WebviewWindow, String> {
    if let Some(window) = app.get_webview_window(PRIMARY_MENU_POPUP_LABEL) {
        return Ok(window);
    }

    let popup = WebviewWindowBuilder::new(
        app,
        PRIMARY_MENU_POPUP_LABEL,
        WebviewUrl::App("index.html?popup=menu".into()),
    )
    .title("PiForma Menu")
    .inner_size(1.0, 1.0)
    .position(0.0, 0.0)
    .resizable(false)
    .fullscreen(false)
    .decorations(false)
    .transparent(true)
    .shadow(false)
    .skip_taskbar(true)
    .focused(false)
    .visible(false)
    .build()
    .map_err(|err| err.to_string())?;

    println!("primary menu popup created once hidden");
    Ok(popup)
}

fn hide_menu_popup_window(app: &tauri::AppHandle, emit_closed: bool) {
    if let Some(window) = app.get_webview_window(PRIMARY_MENU_POPUP_LABEL) {
        if let Err(err) = window.hide() {
            eprintln!("failed to hide primary menu popup: {err}");
        } else {
            println!("primary menu popup hidden");
            log_popup_actual_size(&window, "hidden");
        }
    }
    if emit_closed {
        if let Err(err) = app.emit("menu-popup-closed", ()) {
            eprintln!("failed to emit menu-popup-closed: {err}");
        }
    }
}

fn log_popup_actual_size(window: &tauri::WebviewWindow, phase: &str) {
    println!(
        "primary menu popup {phase}: actual inner_size={}; actual outer_size={}",
        format_size_result(window.inner_size()),
        format_size_result(window.outer_size())
    );
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
    spawn_shell(&command)
}

#[tauri::command]
fn open_folder(folder: String) -> Result<(), String> {
    let target = match folder.as_str() {
        "applications" => "/usr/share/applications".to_string(),
        "home" => env::var("HOME").map_err(|_| "HOME is not set".to_string())?,
        "desktop" => {
            let home = env::var("HOME").map_err(|_| "HOME is not set".to_string())?;
            format!("{home}/Desktop")
        }
        _ => return Err(format!("unknown folder: {folder}")),
    };

    spawn_with_fallbacks(&[
        vec!["xdg-open".to_string(), target.clone()],
        vec!["gio".to_string(), "open".to_string(), target],
    ])
}

#[tauri::command]
fn new_terminal_window() -> Result<(), String> {
    spawn_with_fallbacks(&[
        vec!["x-terminal-emulator".to_string()],
        vec!["lxterminal".to_string()],
        vec!["xfce4-terminal".to_string()],
        vec!["gnome-terminal".to_string()],
    ])
}

#[tauri::command]
fn send_shortcut(action: String) -> Result<(), String> {
    let key = match action.as_str() {
        "undo" => "ctrl+z",
        "cut" => "ctrl+x",
        "copy" => "ctrl+c",
        "paste" => "ctrl+v",
        "clear" => "Delete",
        "select_all" => "ctrl+a",
        _ => return Err(format!("unknown shortcut action: {action}")),
    };

    spawn_with_fallbacks(&[vec!["xdotool".to_string(), "key".to_string(), key.to_string()]])
}

#[tauri::command]
fn run_system_action(action: String, confirmed: bool) -> Result<(), String> {
    let action = parse_system_action(&action)?;
    match action {
        SystemAction::SleepDisplay => spawn_with_fallbacks(&[vec![
            "xset".to_string(),
            "dpms".to_string(),
            "force".to_string(),
            "off".to_string(),
        ]]),
        SystemAction::ShowDesktop => spawn_with_fallbacks(&[
            vec!["wmctrl".to_string(), "-k".to_string(), "on".to_string()],
            vec![
                "xdotool".to_string(),
                "key".to_string(),
                "Super+d".to_string(),
            ],
        ]),
        SystemAction::Refresh => Ok(()),
        SystemAction::CleanUpWindow => Ok(()),
        SystemAction::ShowClipboard => Ok(()),
        SystemAction::Restart => {
            if !confirmed {
                return Err("restart requires confirmation".to_string());
            }
            spawn_with_fallbacks(&[vec!["systemctl".to_string(), "reboot".to_string()]])
        }
        SystemAction::ShutDown => {
            if !confirmed {
                return Err("shutdown requires confirmation".to_string());
            }
            spawn_with_fallbacks(&[vec!["systemctl".to_string(), "poweroff".to_string()]])
        }
    }
}

fn parse_system_action(action: &str) -> Result<SystemAction, String> {
    match action {
        "sleep_display" => Ok(SystemAction::SleepDisplay),
        "show_desktop" => Ok(SystemAction::ShowDesktop),
        "refresh" => Ok(SystemAction::Refresh),
        "clean_up_window" => Ok(SystemAction::CleanUpWindow),
        "restart" => Ok(SystemAction::Restart),
        "shut_down" => Ok(SystemAction::ShutDown),
        "show_clipboard" => Ok(SystemAction::ShowClipboard),
        _ => Err(format!("unknown system action: {action}")),
    }
}

fn scan_desktop_apps(config: &PanelConfig) -> Result<Vec<DesktopApp>, String> {
    let mut apps = BTreeMap::new();

    for dir in &config.applications.scan_dirs {
        let path = PathBuf::from(dir);
        if !path.is_dir() {
            continue;
        }
        scan_desktop_dir(&path, config, &mut apps)?;
    }

    let mut apps: Vec<DesktopApp> = apps.into_values().collect();
    apps.sort_by(|a, b| {
        a.group
            .cmp(&b.group)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    Ok(apps)
}

fn scan_desktop_dir(
    dir: &Path,
    config: &PanelConfig,
    apps: &mut BTreeMap<String, DesktopApp>,
) -> Result<(), String> {
    let entries = fs::read_dir(dir).map_err(|err| err.to_string())?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            scan_desktop_dir(&path, config, apps)?;
            continue;
        }
        if path.extension().and_then(|ext| ext.to_str()) != Some("desktop") {
            continue;
        }
        if let Some(app) = parse_desktop_file(&path, config)? {
            apps.entry(app.id.clone()).or_insert(app);
        }
    }
    Ok(())
}

fn parse_desktop_file(path: &Path, config: &PanelConfig) -> Result<Option<DesktopApp>, String> {
    let contents = fs::read_to_string(path).map_err(|err| err.to_string())?;
    let mut in_desktop_entry = false;
    let mut values = BTreeMap::new();

    for raw_line in contents.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            in_desktop_entry = line == "[Desktop Entry]";
            continue;
        }
        if !in_desktop_entry {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            values.insert(key.to_string(), value.to_string());
        }
    }

    if values.get("Type").is_some_and(|value| value != "Application") {
        return Ok(None);
    }
    if parse_bool(values.get("Hidden")) {
        return Ok(None);
    }
    if parse_bool(values.get("NoDisplay")) && !config.applications.show_no_display {
        return Ok(None);
    }

    let Some(name) = values.get("Name").cloned() else {
        return Ok(None);
    };
    let Some(exec) = values.get("Exec").cloned() else {
        return Ok(None);
    };

    let categories = values
        .get("Categories")
        .map(|value| {
            value
                .split(';')
                .filter(|item| !item.trim().is_empty())
                .map(|item| item.trim().to_string())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let group = broad_group(&categories);
    let is_control_panel = is_control_panel(&categories);
    let id = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(&name)
        .to_string();

    Ok(Some(DesktopApp {
        id,
        name,
        exec,
        icon: values.get("Icon").cloned(),
        categories,
        group,
        is_control_panel,
    }))
}

fn parse_bool(value: Option<&String>) -> bool {
    value.is_some_and(|value| value.eq_ignore_ascii_case("true"))
}

fn is_control_panel(categories: &[String]) -> bool {
    categories.iter().any(|category| {
        matches!(
            category.as_str(),
            "Settings" | "System" | "DesktopSettings" | "HardwareSettings"
        )
    })
}

fn broad_group(categories: &[String]) -> String {
    for category in categories {
        match category.as_str() {
            "Settings" | "DesktopSettings" | "HardwareSettings" => return "Settings".to_string(),
            "System" | "Utility" => return "System".to_string(),
            "Network" | "WebBrowser" | "Email" => return "Internet".to_string(),
            "Office" | "WordProcessor" | "Spreadsheet" => return "Office".to_string(),
            "Graphics" | "Photography" => return "Graphics".to_string(),
            "AudioVideo" | "Audio" | "Video" => return "Sound & Video".to_string(),
            "Development" => return "Development".to_string(),
            "Game" => return "Games".to_string(),
            "Education" => return "Education".to_string(),
            _ => {}
        }
    }
    "Other".to_string()
}

fn clean_exec_command(exec: &str, name: &str) -> String {
    let mut command = exec.to_string();
    for token in ["%u", "%U", "%f", "%F", "%i", "%k"] {
        command = command.replace(token, "");
    }
    command = command.replace("%c", &shell_quote(name));
    command.trim().to_string()
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn spawn_shell(command: &str) -> Result<(), String> {
    Command::new("sh")
        .arg("-c")
        .arg(command)
        .spawn()
        .map(|_| ())
        .map_err(|err| format!("failed to run {command}: {err}"))
}

fn spawn_with_fallbacks(commands: &[Vec<String>]) -> Result<(), String> {
    let mut errors = Vec::new();
    for command in commands {
        if command.is_empty() {
            continue;
        }
        match Command::new(&command[0]).args(&command[1..]).spawn() {
            Ok(_) => return Ok(()),
            Err(err) => errors.push(format!("{}: {err}", command.join(" "))),
        }
    }
    Err(errors.join("; "))
}
