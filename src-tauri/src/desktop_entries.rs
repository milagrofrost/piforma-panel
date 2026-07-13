use crate::config::PanelConfig;
use serde::Serialize;
use std::{collections::BTreeMap, fs, path::Path};

#[derive(Debug, Clone, Serialize)]
pub struct DesktopApp {
    pub id: String,
    pub name: String,
    pub exec: String,
    pub icon: Option<String>,
    pub categories: Vec<String>,
    pub group: String,
    pub is_control_panel: bool,
}

pub fn scan_desktop_apps(config: &PanelConfig) -> Result<Vec<DesktopApp>, String> {
    let mut apps = BTreeMap::new();

    for dir in &config.applications.scan_dirs {
        let path = std::path::PathBuf::from(dir);
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn temp_desktop_dir() -> PathBuf {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be valid")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "piforma-panel-desktop-test-{}-{timestamp}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("test desktop dir should be created");
        path
    }

    fn write_desktop(dir: &Path, name: &str, contents: &str) {
        fs::write(dir.join(name), contents).expect("desktop test file should be written");
    }

    #[test]
    fn scans_visible_desktop_applications() {
        let dir = temp_desktop_dir();
        write_desktop(
            &dir,
            "calculator.desktop",
            r#"
[Desktop Entry]
Type=Application
Name=Calculator
Exec=xcalc %U
Icon=accessories-calculator
Categories=Utility;Calculator;
"#,
        );
        let mut config = PanelConfig::default();
        config.applications.scan_dirs = vec![dir.display().to_string()];

        let apps = scan_desktop_apps(&config).expect("desktop scan should succeed");

        assert_eq!(apps.len(), 1);
        assert_eq!(apps[0].id, "calculator.desktop");
        assert_eq!(apps[0].name, "Calculator");
        assert_eq!(apps[0].exec, "xcalc %U");
        assert_eq!(apps[0].icon.as_deref(), Some("accessories-calculator"));
        assert_eq!(apps[0].group, "System");
        fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn filters_no_display_apps_unless_enabled() {
        let dir = temp_desktop_dir();
        write_desktop(
            &dir,
            "hidden.desktop",
            r#"
[Desktop Entry]
Type=Application
Name=Hidden
Exec=hidden
NoDisplay=true
"#,
        );
        let mut config = PanelConfig::default();
        config.applications.scan_dirs = vec![dir.display().to_string()];

        assert!(scan_desktop_apps(&config)
            .expect("desktop scan should succeed")
            .is_empty());

        config.applications.show_no_display = true;
        assert_eq!(
            scan_desktop_apps(&config)
                .expect("desktop scan should succeed")
                .len(),
            1
        );
        fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn identifies_control_panel_categories() {
        let dir = temp_desktop_dir();
        write_desktop(
            &dir,
            "settings.desktop",
            r#"
[Desktop Entry]
Type=Application
Name=Settings
Exec=settings
Categories=Settings;HardwareSettings;
"#,
        );
        let mut config = PanelConfig::default();
        config.applications.scan_dirs = vec![dir.display().to_string()];

        let apps = scan_desktop_apps(&config).expect("desktop scan should succeed");

        assert_eq!(apps.len(), 1);
        assert!(apps[0].is_control_panel);
        assert_eq!(apps[0].group, "Settings");
        fs::remove_dir_all(dir).ok();
    }
}
