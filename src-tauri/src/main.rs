use base64::Engine;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};
use tauri::Manager;

const DEFAULT_CONFIG: &str = r#"bar:
  width: 656
  height: 20
  x: 77
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
    tauri::Builder::default()
        .setup(|app| {
            let config = ensure_config().map_err(|err| err.to_string())?;
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_size(tauri::Size::Physical(tauri::PhysicalSize {
                    width: config.bar.width,
                    height: config.bar.height,
                }));
                let _ = window.set_position(tauri::Position::Physical(tauri::PhysicalPosition {
                    x: config.bar.x,
                    y: config.bar.y,
                }));
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_config,
            get_apple_logo_data_url,
            resize_panel_window,
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

#[tauri::command]
fn get_config() -> Result<PanelConfig, String> {
    ensure_config()
}

#[tauri::command]
fn resize_panel_window(app: tauri::AppHandle, menu_height: Option<u32>) -> Result<(), String> {
    let config = ensure_config()?;
    let height = match menu_height {
        Some(menu_height) => config
            .bar
            .height
            .saturating_add(menu_height.min(config.applications.max_menu_height)),
        None => config.bar.height,
    };

    if let Some(window) = app.get_webview_window("main") {
        window
            .set_size(tauri::Size::Physical(tauri::PhysicalSize {
                width: config.bar.width,
                height,
            }))
            .map_err(|err| err.to_string())?;
        window
            .set_position(tauri::Position::Physical(tauri::PhysicalPosition {
                x: config.bar.x,
                y: config.bar.y,
            }))
            .map_err(|err| err.to_string())?;
    }

    Ok(())
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
