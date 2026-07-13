use base64::Engine;
use gtk::prelude::{ContainerExt, GtkWindowExt, WidgetExt};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::Mutex,
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tauri::{Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

const PRIMARY_MENU_POPUP_LABEL: &str = "menu-popup";
const FLYOUT_MENU_POPUP_LABEL: &str = "menu-flyout";
const MAIN_WINDOW_MIN_WIDTH: u32 = 1;
const MAIN_WINDOW_MIN_HEIGHT: u32 = 1;
const PANEL_WIDTH_MAX: i32 = 8192;
const PANEL_HEIGHT_MAX: i32 = 512;
const PANEL_RADIUS_MAX: i32 = 256;
const PANEL_FONT_SIZE_MAX: i32 = 96;
const MENU_HEIGHT_MAX: i32 = 4096;

const DEFAULT_CONFIG: &str = r#"# PiForma Panel config.
# Missing sections and fields use the defaults shown here.
bar:
  width: 658
  height: 20
  x: 76
  y: 0
  radius_top_left: 18
  radius_top_right: 18
  font_family: ChicagoFLF
  font_size: 13

apple:
  logo_path: ~/.local/share/piforma-panel/apple-color.png

clock:
  enabled: true
  format: "%I:%M %p"

applications:
  scan_dirs:
    - ~/.local/share/applications
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

actions:
  clean_up_window_command: ""
"#;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
struct PanelConfig {
    bar: BarConfig,
    apple: AppleConfig,
    clock: ClockConfig,
    applications: ApplicationsConfig,
    menus: MenusConfig,
    #[serde(default)]
    actions: ActionsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
struct BarConfig {
    width: i32,
    height: i32,
    x: i32,
    y: i32,
    radius_top_left: i32,
    radius_top_right: i32,
    font_family: String,
    font_size: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
struct AppleConfig {
    logo_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
struct ClockConfig {
    enabled: bool,
    format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
struct ApplicationsConfig {
    scan_dirs: Vec<String>,
    show_no_display: bool,
    group_by_categories: bool,
    show_category_labels: bool,
    max_menu_height: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
struct MenusConfig {
    show_file: bool,
    show_edit: bool,
    show_view: bool,
    show_special: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
struct ActionsConfig {
    #[serde(default)]
    clean_up_window_command: String,
}

impl Default for PanelConfig {
    fn default() -> Self {
        Self {
            bar: BarConfig::default(),
            apple: AppleConfig::default(),
            clock: ClockConfig::default(),
            applications: ApplicationsConfig::default(),
            menus: MenusConfig::default(),
            actions: ActionsConfig::default(),
        }
    }
}

impl Default for BarConfig {
    fn default() -> Self {
        Self {
            width: 658,
            height: 20,
            x: 76,
            y: 0,
            radius_top_left: 18,
            radius_top_right: 18,
            font_family: "ChicagoFLF".to_string(),
            font_size: 13,
        }
    }
}

impl Default for AppleConfig {
    fn default() -> Self {
        Self {
            logo_path: "~/.local/share/piforma-panel/apple-color.png".to_string(),
        }
    }
}

impl Default for ClockConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            format: "%I:%M %p".to_string(),
        }
    }
}

impl Default for ApplicationsConfig {
    fn default() -> Self {
        Self {
            scan_dirs: vec![
                "~/.local/share/applications".to_string(),
                "/usr/local/share/applications".to_string(),
                "/usr/share/applications".to_string(),
            ],
            show_no_display: false,
            group_by_categories: true,
            show_category_labels: false,
            max_menu_height: 420,
        }
    }
}

impl Default for MenusConfig {
    fn default() -> Self {
        Self {
            show_file: true,
            show_edit: true,
            show_view: true,
            show_special: true,
        }
    }
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

#[derive(Default)]
struct WindowMemory {
    previous_active_window: Mutex<Option<String>>,
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
    load_config_from_str(&contents)
}

fn load_config_from_str(contents: &str) -> Result<PanelConfig, String> {
    let mut config: PanelConfig = if contents.trim().is_empty() {
        PanelConfig::default()
    } else {
        serde_yaml::from_str(contents).map_err(|err| format!("invalid config.yaml: {err}"))?
    };
    normalize_config(&mut config);
    Ok(config)
}

fn normalize_config(config: &mut PanelConfig) {
    normalize_range("bar.width", &mut config.bar.width, 1, PANEL_WIDTH_MAX);
    normalize_range("bar.height", &mut config.bar.height, 1, PANEL_HEIGHT_MAX);
    normalize_range(
        "bar.radius_top_left",
        &mut config.bar.radius_top_left,
        0,
        PANEL_RADIUS_MAX,
    );
    normalize_range(
        "bar.radius_top_right",
        &mut config.bar.radius_top_right,
        0,
        PANEL_RADIUS_MAX,
    );
    normalize_range(
        "bar.font_size",
        &mut config.bar.font_size,
        1,
        PANEL_FONT_SIZE_MAX,
    );
    normalize_range(
        "applications.max_menu_height",
        &mut config.applications.max_menu_height,
        1,
        MENU_HEIGHT_MAX,
    );
    config.apple.logo_path = expand_user_path(&config.apple.logo_path);
    config.applications.scan_dirs = config
        .applications
        .scan_dirs
        .iter()
        .map(|path| expand_user_path(path))
        .collect();
}

fn normalize_range(name: &str, value: &mut i32, min: i32, max: i32) {
    let original = *value;
    if original < min {
        *value = min;
    } else if original > max {
        *value = max;
    }

    if *value != original {
        eprintln!("config warning: {name}={original} normalized to {}", *value);
    }
}

fn expand_user_path(path: &str) -> String {
    let Some(home) = env::var_os("HOME") else {
        return path.to_string();
    };
    let home = PathBuf::from(home);
    if path == "~" {
        return home.display().to_string();
    }
    if let Some(rest) = path.strip_prefix("~/") {
        return home.join(rest).display().to_string();
    }
    path.to_string()
}

fn config_dimension(value: i32, name: &str) -> Result<u32, String> {
    u32::try_from(value).map_err(|err| format!("invalid {name} after normalization: {err}"))
}

fn configure_main_window(
    window: &tauri::WebviewWindow,
    config: &PanelConfig,
    phase: &str,
) -> Result<(), String> {
    let width = config_dimension(config.bar.width, "bar.width")?;
    let height = config_dimension(config.bar.height, "bar.height")?;
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
        .set_size(tauri::Size::Physical(tauri::PhysicalSize { width, height }))
        .map_err(|err| err.to_string())?;
    apply_main_gtk_size(window, width, height)?;
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

fn apply_main_gtk_size(
    window: &tauri::WebviewWindow,
    width: u32,
    height: u32,
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

    println!("gtk tight size applied: label=main, width={width}, height={height}");
    Ok(())
}

fn apply_popup_gtk_size(
    window: &tauri::WebviewWindow,
    width: u32,
    height: u32,
) -> Result<(), String> {
    let width_i32 = i32::try_from(width).map_err(|err| err.to_string())?;
    let height_i32 = i32::try_from(height).map_err(|err| err.to_string())?;
    let gtk_window = window.gtk_window().map_err(|err| err.to_string())?;

    gtk_window.set_size_request(-1, -1);

    if let Ok(vbox) = window.default_vbox() {
        vbox.set_size_request(-1, -1);
        for child in vbox.children() {
            child.set_size_request(-1, -1);
        }
    }

    gtk_window.set_default_size(width_i32, height_i32);
    window
        .set_size(tauri::Size::Physical(tauri::PhysicalSize { width, height }))
        .map_err(|err| err.to_string())?;
    gtk_window.resize(width_i32, height_i32);
    gtk_window.set_size_request(width_i32, height_i32);

    if let Ok(vbox) = window.default_vbox() {
        vbox.set_size_request(width_i32, height_i32);
        for child in vbox.children() {
            child.set_size_request(width_i32, height_i32);
        }
    }

    gtk_window.resize(width_i32, height_i32);

    println!("gtk popup size applied: width={width}, height={height}");
    Ok(())
}

fn reset_popup_gtk_size(window: &tauri::WebviewWindow) -> Result<(), String> {
    let gtk_window = window.gtk_window().map_err(|err| err.to_string())?;
    gtk_window.set_size_request(-1, -1);
    if let Ok(vbox) = window.default_vbox() {
        vbox.set_size_request(-1, -1);
        for child in vbox.children() {
            child.set_size_request(-1, -1);
        }
    }
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

fn ensure_menu_popup_window(app: &tauri::AppHandle) -> Result<tauri::WebviewWindow, String> {
    ensure_popup_window(
        app,
        PRIMARY_MENU_POPUP_LABEL,
        "index.html?popup=menu",
        "PiForma Menu",
    )
}

fn ensure_menu_flyout_window(app: &tauri::AppHandle) -> Result<tauri::WebviewWindow, String> {
    ensure_popup_window(
        app,
        FLYOUT_MENU_POPUP_LABEL,
        "index.html?popup=flyout",
        "PiForma Menu Flyout",
    )
}

fn ensure_popup_window(
    app: &tauri::AppHandle,
    label: &'static str,
    url: &str,
    title: &str,
) -> Result<tauri::WebviewWindow, String> {
    if let Some(window) = app.get_webview_window(label) {
        return Ok(window);
    }

    let popup = WebviewWindowBuilder::new(app, label, WebviewUrl::App(url.into()))
        .title(title)
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

    if label == FLYOUT_MENU_POPUP_LABEL {
        println!("flyout popup created once hidden");
    } else {
        println!("primary menu popup created once hidden");
    }
    Ok(popup)
}

fn hide_menu_popup_window(app: &tauri::AppHandle, emit_closed: bool) {
    hide_menu_flyout_window(app);
    if let Some(window) = app.get_webview_window(PRIMARY_MENU_POPUP_LABEL) {
        if let Err(err) = window.hide() {
            eprintln!("failed to hide primary menu popup: {err}");
        } else {
            println!("primary menu popup hidden");
            log_popup_actual_size_unchecked(&window, PRIMARY_MENU_POPUP_LABEL, "hidden");
        }
    }
    if emit_closed {
        if let Err(err) = app.emit("menu-popup-closed", ()) {
            eprintln!("failed to emit menu-popup-closed: {err}");
        }
    }
}

fn hide_menu_flyout_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window(FLYOUT_MENU_POPUP_LABEL) {
        if let Err(err) = window.hide() {
            eprintln!("failed to hide flyout menu popup: {err}");
        } else {
            println!("flyout hidden");
            log_popup_actual_size_unchecked(&window, FLYOUT_MENU_POPUP_LABEL, "hidden");
        }
    }
}

fn resize_menu_popup_window(
    window: &tauri::WebviewWindow,
    width: u32,
    height: u32,
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
        .set_size(tauri::Size::Physical(tauri::PhysicalSize { width, height }))
        .map_err(|err| err.to_string())?;
    apply_popup_gtk_size(window, width, height)
}

fn log_popup_actual_size(
    window: &tauri::WebviewWindow,
    label: &str,
    phase: &str,
    width: u32,
    height: u32,
) {
    let inner_size = window.inner_size();
    let differs = inner_size
        .as_ref()
        .map(|size| size.width != width || size.height != height)
        .unwrap_or(true);
    println!(
        "{label} {phase}: requested={}x{}, actual inner_size={}, actual outer_size={}, differs_from_requested={}",
        width,
        height,
        format_size_result(inner_size),
        format_size_result(window.outer_size()),
        differs
    );
}

fn log_popup_actual_size_unchecked(window: &tauri::WebviewWindow, label: &str, phase: &str) {
    println!(
        "{label} {phase}: actual inner_size={}, actual outer_size={}",
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
    let window_id = command_stdout("xdotool", &["getactivewindow"])?;
    if window_id.is_empty() {
        return Err("xdotool did not return an active window".to_string());
    }
    if is_panel_window(&app, &window_id) {
        return Ok(());
    }
    let mut remembered = memory
        .previous_active_window
        .lock()
        .map_err(|_| "remembered window lock poisoned".to_string())?;
    *remembered = Some(window_id);
    Ok(())
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
    let action = parse_system_action(&action)?;
    match action {
        SystemAction::SleepDisplay => spawn_short_with_fallbacks(&[
            vec![
                "xset".to_string(),
                "dpms".to_string(),
                "force".to_string(),
                "off".to_string(),
            ],
            vec!["xset".to_string(), "s".to_string(), "activate".to_string()],
        ]),
        SystemAction::ShowDesktop => {
            hide_menu_popup_window(&app, true);
            spawn_short_with_fallbacks(&[
                vec!["wmctrl".to_string(), "-k".to_string(), "on".to_string()],
                vec![
                    "xdotool".to_string(),
                    "key".to_string(),
                    "Super+d".to_string(),
                ],
            ])
        }
        SystemAction::Refresh => {
            hide_menu_popup_window(&app, true);
            if activate_remembered_window(&memory).is_ok() {
                run_short_command("xdotool", &["key", "F5"])
            } else {
                run_short_command("xrefresh", &[])
            }
        }
        SystemAction::CleanUpWindow => {
            hide_menu_popup_window(&app, true);
            let config = ensure_config()?;
            if let Err(err) = activate_remembered_window(&memory) {
                eprintln!("clean up window: no remembered target to reactivate: {err}");
            }
            if command_exists("xdotool") {
                let _ = run_short_command("xdotool", &["key", "F5"]);
            }
            if config.actions.clean_up_window_command.trim().is_empty() {
                Ok(())
            } else {
                spawn_detached_shell(&config.actions.clean_up_window_command)
            }
        }
        SystemAction::ShowClipboard => show_clipboard(),
        SystemAction::Restart => {
            if !confirmed {
                return Err("restart requires confirmation".to_string());
            }
            run_short_with_fallbacks(&[
                vec!["systemctl".to_string(), "reboot".to_string()],
                vec!["loginctl".to_string(), "reboot".to_string()],
            ])
        }
        SystemAction::ShutDown => {
            if !confirmed {
                return Err("shutdown requires confirmation".to_string());
            }
            run_short_with_fallbacks(&[
                vec!["systemctl".to_string(), "poweroff".to_string()],
                vec!["loginctl".to_string(), "poweroff".to_string()],
            ])
        }
    }
}

#[tauri::command]
fn confirm_system_action(app: tauri::AppHandle, action: String) -> Result<(), String> {
    hide_menu_popup_window(&app, true);

    let action = parse_system_action(&action)?;
    let confirmation = match action {
        SystemAction::Restart => ConfirmationSpec {
            title: "Restart",
            text: "Are you sure you want to restart PiForma?",
            ok_label: "Restart",
            xmessage_buttons: "Cancel:1,Restart:0",
        },
        SystemAction::ShutDown => ConfirmationSpec {
            title: "Shut Down",
            text: "Are you sure you want to shut down PiForma?",
            ok_label: "Shut Down",
            xmessage_buttons: "Cancel:1,Shut Down:0",
        },
        _ => return Err("confirmation is only supported for restart and shut_down".to_string()),
    };

    if !confirm_with_desktop_dialog(&confirmation)? {
        return Ok(());
    }

    match action {
        SystemAction::Restart => run_short_with_fallbacks(&[
            vec!["systemctl".to_string(), "reboot".to_string()],
            vec!["loginctl".to_string(), "reboot".to_string()],
        ]),
        SystemAction::ShutDown => run_short_with_fallbacks(&[
            vec!["systemctl".to_string(), "poweroff".to_string()],
            vec!["loginctl".to_string(), "poweroff".to_string()],
        ]),
        _ => unreachable!("non-power actions returned before confirmation"),
    }
}

struct ConfirmationSpec {
    title: &'static str,
    text: &'static str,
    ok_label: &'static str,
    xmessage_buttons: &'static str,
}

fn confirm_with_desktop_dialog(spec: &ConfirmationSpec) -> Result<bool, String> {
    let dialogs = [
        vec![
            "zenity".to_string(),
            "--question".to_string(),
            format!("--title={}", spec.title),
            format!("--text={}", spec.text),
            format!("--ok-label={}", spec.ok_label),
            "--cancel-label=Cancel".to_string(),
        ],
        vec![
            "yad".to_string(),
            "--question".to_string(),
            format!("--title={}", spec.title),
            format!("--text={}", spec.text),
            format!("--button={}:0", spec.ok_label),
            "--button=Cancel:1".to_string(),
        ],
        vec![
            "xmessage".to_string(),
            "-buttons".to_string(),
            spec.xmessage_buttons.to_string(),
            "-default".to_string(),
            "Cancel".to_string(),
            spec.text.to_string(),
        ],
    ];

    let mut errors = Vec::new();
    for dialog in dialogs {
        let program = &dialog[0];
        if !command_exists(program) {
            errors.push(format!("{program} not found"));
            continue;
        }

        let status = Command::new(program)
            .args(&dialog[1..])
            .stdin(Stdio::null())
            .status()
            .map_err(|err| format!("failed to run {program}: {err}"))?;

        match status.code() {
            Some(0) => return Ok(true),
            Some(1) => return Ok(false),
            Some(code) => errors.push(format!("{program} exited with status {code}")),
            None => errors.push(format!("{program} terminated without an exit code")),
        }
    }

    Err(format!(
        "failed to show confirmation dialog: {}",
        errors.join("; ")
    ))
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

    if values
        .get("Type")
        .is_some_and(|value| value != "Application")
    {
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
    remove_remaining_field_codes(&command).trim().to_string()
}

fn remove_remaining_field_codes(command: &str) -> String {
    let mut cleaned = String::new();
    let mut chars = command.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch != '%' {
            cleaned.push(ch);
            continue;
        }
        match chars.peek() {
            Some('%') => {
                chars.next();
                cleaned.push('%');
            }
            Some(_) => {
                chars.next();
            }
            None => {}
        }
    }
    cleaned
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn resolve_desktop_dir(home: &str) -> Result<PathBuf, String> {
    if command_exists("xdg-user-dir") {
        if let Ok(path) = command_stdout("xdg-user-dir", &["DESKTOP"]) {
            let path = PathBuf::from(path);
            if path.is_dir() {
                return Ok(path);
            }
        }
    }
    let path = PathBuf::from(home).join("Desktop");
    fs::create_dir_all(&path)
        .map_err(|err| format!("failed to create {}: {err}", path.display()))?;
    Ok(path)
}

fn show_clipboard() -> Result<(), String> {
    let command = r#"
set -eu
tmp="${TMPDIR:-/tmp}/piforma-clipboard-$$.txt"
trap 'rm -f "$tmp"' EXIT
if command -v xclip >/dev/null 2>&1; then
  xclip -selection clipboard -o >"$tmp" 2>/dev/null || printf '%s\n' 'Clipboard is empty or does not contain text.' >"$tmp"
elif command -v xsel >/dev/null 2>&1; then
  xsel --clipboard --output >"$tmp" 2>/dev/null || printf '%s\n' 'Clipboard is empty or does not contain text.' >"$tmp"
elif command -v wl-paste >/dev/null 2>&1; then
  wl-paste --no-newline >"$tmp" 2>/dev/null || printf '%s\n' 'Clipboard is empty or does not contain text.' >"$tmp"
else
  printf '%s\n' 'No clipboard reader found. Install xclip, xsel, or wl-clipboard.' >"$tmp"
fi
if [ ! -s "$tmp" ]; then
  printf '%s\n' 'Clipboard is empty or does not contain text.' >"$tmp"
fi
if command -v zenity >/dev/null 2>&1; then
  exec zenity --text-info --title='Clipboard' --filename="$tmp"
elif command -v yad >/dev/null 2>&1; then
  exec yad --text-info --title='Clipboard' --filename="$tmp"
elif command -v xmessage >/dev/null 2>&1; then
  exec xmessage -file "$tmp"
else
  exit 127
fi
"#;
    spawn_detached_shell(command)
}

fn activate_remembered_window(memory: &tauri::State<WindowMemory>) -> Result<String, String> {
    let window_id = memory
        .previous_active_window
        .lock()
        .map_err(|_| "remembered window lock poisoned".to_string())?
        .clone()
        .ok_or_else(|| "no previously active window remembered".to_string())?;
    run_short_command("xdotool", &["windowactivate", "--sync", &window_id])?;
    thread::sleep(Duration::from_millis(75));
    Ok(window_id)
}

fn is_panel_window(_app: &tauri::AppHandle, window_id: &str) -> bool {
    let Ok(name) = command_stdout("xdotool", &["getwindowname", window_id]) else {
        return false;
    };
    matches!(
        name.as_str(),
        "PiForma Panel" | "PiForma Menu" | "PiForma Menu Flyout" | "Classic PiForma menu bar"
    ) || name.contains("piforma-panel")
}

fn command_stdout(program: &str, args: &[&str]) -> Result<String, String> {
    let output = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .map_err(|err| format!("failed to run {program}: {err}"))?;
    if !output.status.success() {
        return Err(format!("{program} exited with status {}", output.status));
    }
    String::from_utf8(output.stdout)
        .map(|value| value.trim().to_string())
        .map_err(|err| format!("{program} returned non-UTF-8 output: {err}"))
}

fn run_short_command(program: &str, args: &[&str]) -> Result<(), String> {
    let status = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|err| format!("failed to run {program}: {err}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{program} exited with status {status}"))
    }
}

fn spawn_short_with_fallbacks(commands: &[Vec<String>]) -> Result<(), String> {
    let mut errors = Vec::new();
    for command in commands {
        if command.is_empty() {
            continue;
        }
        match Command::new(&command[0])
            .args(&command[1..])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(_) => return Ok(()),
            Err(err) => errors.push(format!("{}: {err}", command.join(" "))),
        }
    }
    Err(errors.join("; "))
}

fn run_short_with_fallbacks(commands: &[Vec<String>]) -> Result<(), String> {
    let mut errors = Vec::new();
    for command in commands {
        if command.is_empty() {
            continue;
        }
        match Command::new(&command[0])
            .args(&command[1..])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
        {
            Ok(status) if status.success() => return Ok(()),
            Ok(status) => errors.push(format!("{} exited with status {status}", command.join(" "))),
            Err(err) => errors.push(format!("{}: {err}", command.join(" "))),
        }
    }
    Err(errors.join("; "))
}

fn spawn_detached_with_fallbacks(commands: &[Vec<String>]) -> Result<(), String> {
    let mut errors = Vec::new();
    for command in commands {
        if command.is_empty() {
            continue;
        }
        if !command_exists(&command[0]) {
            errors.push(format!("{} not found", command[0]));
            continue;
        }
        match spawn_detached(&command[0], &command[1..]) {
            Ok(()) => return Ok(()),
            Err(err) => errors.push(err),
        }
    }
    Err(errors.join("; "))
}

fn spawn_detached_shell(command: &str) -> Result<(), String> {
    spawn_detached(
        "sh",
        &[
            "-c".to_string(),
            "exec sh -c \"$1\"".to_string(),
            "piforma-shell".to_string(),
            command.to_string(),
        ],
    )
}

fn spawn_detached(program: &str, args: &[String]) -> Result<(), String> {
    if command_exists("systemd-run") {
        let unit = unique_launch_unit_name();
        let mut command_args = vec![
            "--user".to_string(),
            "--scope".to_string(),
            "--collect".to_string(),
            "--quiet".to_string(),
            format!("--unit={unit}"),
        ];
        for key in [
            "DISPLAY",
            "XAUTHORITY",
            "WAYLAND_DISPLAY",
            "XDG_RUNTIME_DIR",
            "DBUS_SESSION_BUS_ADDRESS",
        ] {
            if let Ok(value) = env::var(key) {
                command_args.push(format!("--setenv={key}={value}"));
            }
        }
        command_args.push("--".to_string());
        command_args.push(program.to_string());
        command_args.extend(args.iter().cloned());
        match Command::new("systemd-run")
            .args(&command_args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(_) => return Ok(()),
            Err(err) => eprintln!("systemd-run detached launch failed: {err}"),
        }
    }

    if command_exists("setsid") {
        let mut command = Command::new("setsid");
        command.arg("-f").arg("--").arg(program).args(args);
        match command
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(_) => return Ok(()),
            Err(err) => eprintln!("setsid detached launch failed: {err}"),
        }
    }

    Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(|err| format!("failed to launch {program}: {err}"))
}

fn unique_launch_unit_name() -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    format!("piforma-launch-{}-{timestamp}", std::process::id())
}

fn command_exists(name: &str) -> bool {
    let Some(path) = env::var_os("PATH") else {
        return false;
    };
    env::split_paths(&path).any(|dir| dir.join(name).is_file())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_config_loads_defaults() {
        let config = load_config_from_str("").expect("empty config should load");

        assert_eq!(config.bar.width, 658);
        assert_eq!(config.bar.height, 20);
        assert_eq!(config.bar.x, 76);
        assert_eq!(config.clock.format, "%I:%M %p");
        assert!(config.menus.show_file);
        assert_eq!(config.applications.max_menu_height, 420);
    }

    #[test]
    fn partial_config_fills_missing_sections_and_fields() {
        let config = load_config_from_str(
            r#"
clock:
  enabled: false
"#,
        )
        .expect("partial config should load");

        assert!(!config.clock.enabled);
        assert_eq!(config.clock.format, "%I:%M %p");
        assert_eq!(config.bar.width, 658);
        assert!(config.menus.show_special);
        assert_eq!(config.applications.scan_dirs.len(), 3);
    }

    #[test]
    fn valid_overrides_are_preserved() {
        let config = load_config_from_str(
            r#"
bar:
  width: 700
  height: 24
  x: -12
  y: 3
  radius_top_left: 4
  font_size: 15
menus:
  show_view: false
applications:
  scan_dirs:
    - /tmp/apps
  max_menu_height: 300
actions:
  clean_up_window_command: "arrange-windows"
"#,
        )
        .expect("override config should load");

        assert_eq!(config.bar.width, 700);
        assert_eq!(config.bar.height, 24);
        assert_eq!(config.bar.x, -12);
        assert_eq!(config.bar.y, 3);
        assert_eq!(config.bar.radius_top_left, 4);
        assert_eq!(config.bar.radius_top_right, 18);
        assert_eq!(config.bar.font_size, 15);
        assert!(!config.menus.show_view);
        assert_eq!(config.applications.scan_dirs, vec!["/tmp/apps"]);
        assert_eq!(config.applications.max_menu_height, 300);
        assert_eq!(config.actions.clean_up_window_command, "arrange-windows");
    }

    #[test]
    fn malformed_yaml_returns_clear_error() {
        let error = load_config_from_str("bar: [").expect_err("malformed YAML should fail");

        assert!(error.starts_with("invalid config.yaml:"));
    }

    #[test]
    fn invalid_numeric_values_are_clamped() {
        let config = load_config_from_str(
            r#"
bar:
  width: 0
  height: -5
  radius_top_left: -1
  radius_top_right: 999
  font_size: 0
applications:
  max_menu_height: 99999
"#,
        )
        .expect("invalid numeric config should be recoverable");

        assert_eq!(config.bar.width, 1);
        assert_eq!(config.bar.height, 1);
        assert_eq!(config.bar.radius_top_left, 0);
        assert_eq!(config.bar.radius_top_right, PANEL_RADIUS_MAX);
        assert_eq!(config.bar.font_size, 1);
        assert_eq!(config.applications.max_menu_height, MENU_HEIGHT_MAX);
    }

    #[test]
    fn user_paths_are_expanded() {
        let config = load_config_from_str(
            r#"
apple:
  logo_path: ~/logo.png
applications:
  scan_dirs:
    - ~/apps
    - /usr/share/applications
"#,
        )
        .expect("config with user paths should load");

        let home = env::var("HOME").expect("HOME should be available for tests");
        assert_eq!(config.apple.logo_path, format!("{home}/logo.png"));
        assert_eq!(config.applications.scan_dirs[0], format!("{home}/apps"));
        assert_eq!(config.applications.scan_dirs[1], "/usr/share/applications");
    }

    #[test]
    fn generated_default_config_matches_model_defaults() {
        let config = load_config_from_str(DEFAULT_CONFIG).expect("default config should load");
        let defaults = PanelConfig::default();
        let home = env::var("HOME").expect("HOME should be available for tests");

        assert_eq!(config.bar.width, defaults.bar.width);
        assert_eq!(config.bar.height, defaults.bar.height);
        assert_eq!(config.bar.x, defaults.bar.x);
        assert_eq!(config.bar.y, defaults.bar.y);
        assert_eq!(config.clock.format, defaults.clock.format);
        assert_eq!(config.menus.show_file, defaults.menus.show_file);
        assert_eq!(
            config.apple.logo_path,
            format!("{home}/.local/share/piforma-panel/apple-color.png")
        );
        assert_eq!(
            config.applications.scan_dirs[0],
            format!("{home}/.local/share/applications")
        );
    }
}
