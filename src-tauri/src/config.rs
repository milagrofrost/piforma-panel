use serde::{Deserialize, Serialize};
use std::{env, fs, path::PathBuf};

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
pub struct PanelConfig {
    pub bar: BarConfig,
    pub apple: AppleConfig,
    pub clock: ClockConfig,
    pub applications: ApplicationsConfig,
    pub menus: MenusConfig,
    #[serde(default)]
    pub actions: ActionsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct BarConfig {
    pub width: i32,
    pub height: i32,
    pub x: i32,
    pub y: i32,
    pub radius_top_left: i32,
    pub radius_top_right: i32,
    pub font_family: String,
    pub font_size: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppleConfig {
    pub logo_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ClockConfig {
    pub enabled: bool,
    pub format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ApplicationsConfig {
    pub scan_dirs: Vec<String>,
    pub show_no_display: bool,
    pub group_by_categories: bool,
    pub show_category_labels: bool,
    pub max_menu_height: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct MenusConfig {
    pub show_file: bool,
    pub show_edit: bool,
    pub show_view: bool,
    pub show_special: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ActionsConfig {
    #[serde(default)]
    pub clean_up_window_command: String,
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

pub fn config_path() -> Result<PathBuf, String> {
    let home = env::var("HOME").map_err(|_| "HOME is not set".to_string())?;
    Ok(PathBuf::from(home).join(".local/share/piforma-panel/config.yaml"))
}

pub fn ensure_config() -> Result<PanelConfig, String> {
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

pub fn print_config_diagnostics(config: &PanelConfig) -> Result<(), String> {
    let path = config_path()?;
    println!("config path: {}", path.display());
    println!(
        "config bar: width={}, height={}, x={}, y={}",
        config.bar.width, config.bar.height, config.bar.x, config.bar.y
    );
    Ok(())
}

pub fn config_dimension(value: i32, name: &str) -> Result<u32, String> {
    u32::try_from(value).map_err(|err| format!("invalid {name} after normalization: {err}"))
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
