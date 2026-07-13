use gtk::prelude::GtkWindowExt;

pub const PIFORMA_PANEL_APP_ID: &str = "org.piforma.panel";
pub const PIFORMA_PANEL_WM_CLASS: &str = "org.piforma.panel";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellWindowRole {
    MainPanel,
    MenuPopup,
    MenuFlyout,
}

impl ShellWindowRole {
    pub fn tauri_label(self) -> &'static str {
        match self {
            Self::MainPanel => "main",
            Self::MenuPopup => "menu-popup",
            Self::MenuFlyout => "menu-flyout",
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Self::MainPanel => "PiForma Panel",
            Self::MenuPopup => "PiForma Menu",
            Self::MenuFlyout => "PiForma Menu Flyout",
        }
    }

    pub fn window_role(self) -> &'static str {
        match self {
            Self::MainPanel => "piforma-panel.main-panel",
            Self::MenuPopup => "piforma-panel.menu-popup",
            Self::MenuFlyout => "piforma-panel.menu-flyout",
        }
    }
}

pub fn apply_shell_window_identity(
    window: &tauri::WebviewWindow,
    role: ShellWindowRole,
) -> Result<(), String> {
    window
        .set_title(role.title())
        .map_err(|err| err.to_string())?;
    let gtk_window = window.gtk_window().map_err(|err| err.to_string())?;
    gtk_window.set_role(role.window_role());
    println!(
        "shell window identity applied: label={}, title={}, app_id={}, wm_class={}, window_role={}",
        role.tauri_label(),
        role.title(),
        PIFORMA_PANEL_APP_ID,
        PIFORMA_PANEL_WM_CLASS,
        role.window_role()
    );
    Ok(())
}

pub fn is_piforma_panel_title(name: &str) -> bool {
    matches!(
        name,
        "PiForma Panel" | "PiForma Menu" | "PiForma Menu Flyout" | "Classic PiForma menu bar"
    ) || name.contains("piforma-panel")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roles_have_stable_labels_titles_and_window_roles() {
        assert_eq!(ShellWindowRole::MainPanel.tauri_label(), "main");
        assert_eq!(ShellWindowRole::MainPanel.title(), "PiForma Panel");
        assert_eq!(
            ShellWindowRole::MainPanel.window_role(),
            "piforma-panel.main-panel"
        );
        assert_eq!(ShellWindowRole::MenuPopup.tauri_label(), "menu-popup");
        assert_eq!(
            ShellWindowRole::MenuFlyout.window_role(),
            "piforma-panel.menu-flyout"
        );
    }

    #[test]
    fn title_matching_is_a_fallback_only() {
        assert!(is_piforma_panel_title("PiForma Panel"));
        assert!(is_piforma_panel_title("piforma-panel"));
        assert!(!is_piforma_panel_title("Terminal"));
    }
}
