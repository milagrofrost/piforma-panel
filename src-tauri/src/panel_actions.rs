use crate::{
    launcher::{
        clean_exec_command, resolve_desktop_dir, run_short_command, spawn_detached,
        spawn_detached_shell, spawn_detached_with_fallbacks,
    },
    popup_windows::hide_menu_popup_window,
    system_actions::{self, SystemActionId},
    window_manager::{activate_remembered_window, WindowMemory},
};
use serde::{Deserialize, Serialize};
use std::{env, fs, path::PathBuf};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PanelAction {
    ShowAbout,
    LaunchApp {
        exec: String,
        name: String,
    },
    LaunchCalculator,
    OpenFolder {
        folder: FolderId,
    },
    NewTerminalWindow,
    SendShortcut {
        action: ShortcutId,
    },
    RunSystemAction {
        action: SystemActionId,
        confirmed: bool,
    },
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FolderId {
    Applications,
    Home,
    Desktop,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ShortcutId {
    Undo,
    Cut,
    Copy,
    Paste,
    Clear,
    SelectAll,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ActionResult {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_kind: Option<ActionErrorKind>,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[allow(dead_code)]
pub enum ActionErrorKind {
    Unsupported,
    Cancelled,
    AuthorizationFailed,
    CommandFailed,
    TargetUnavailable,
    InvalidRequest,
}

impl ActionResult {
    pub fn success(message: impl Into<Option<String>>) -> Self {
        Self {
            success: true,
            message: message.into(),
            error_kind: None,
        }
    }

    pub fn failure(kind: ActionErrorKind, message: impl Into<String>) -> Self {
        Self {
            success: false,
            message: Some(message.into()),
            error_kind: Some(kind),
        }
    }

    pub fn from_command_result(result: Result<(), String>) -> Self {
        match result {
            Ok(()) => Self::success(None),
            Err(err) => Self::failure(ActionErrorKind::CommandFailed, err),
        }
    }

    pub fn into_legacy_result(self) -> Result<(), String> {
        if self.success || self.error_kind == Some(ActionErrorKind::Cancelled) {
            Ok(())
        } else {
            Err(self
                .message
                .unwrap_or_else(|| "panel action failed".to_string()))
        }
    }
}

pub fn execute_panel_action(
    app: &tauri::AppHandle,
    memory: &tauri::State<WindowMemory>,
    action: PanelAction,
) -> ActionResult {
    match action {
        PanelAction::ShowAbout => {
            ActionResult::from_command_result(spawn_detached("about-piforma", &[]))
        }
        PanelAction::LaunchApp { exec, name } => {
            let command = clean_exec_command(&exec, &name);
            ActionResult::from_command_result(spawn_detached_shell(&command))
        }
        PanelAction::LaunchCalculator => {
            ActionResult::from_command_result(spawn_detached_with_fallbacks(&[
                vec!["xcalc".to_string()],
                vec!["galculator".to_string()],
                vec!["gnome-calculator".to_string()],
                vec!["mate-calc".to_string()],
                vec!["kcalc".to_string()],
            ]))
        }
        PanelAction::OpenFolder { folder } => open_folder(folder),
        PanelAction::NewTerminalWindow => {
            ActionResult::from_command_result(spawn_detached_with_fallbacks(&[
                vec!["x-terminal-emulator".to_string()],
                vec!["lxterminal".to_string()],
                vec!["xfce4-terminal".to_string()],
                vec!["gnome-terminal".to_string()],
                vec!["konsole".to_string()],
                vec!["mate-terminal".to_string()],
            ]))
        }
        PanelAction::SendShortcut { action } => send_shortcut(app, memory, action),
        PanelAction::RunSystemAction { action, confirmed } => {
            system_actions::run_system_action_id(app, memory, action, confirmed)
        }
    }
}

fn open_folder(folder: FolderId) -> ActionResult {
    let target = match resolve_folder(folder) {
        Ok(target) => target,
        Err(err) => return ActionResult::failure(ActionErrorKind::TargetUnavailable, err),
    };

    ActionResult::from_command_result(spawn_detached_with_fallbacks(&[
        vec!["xdg-open".to_string(), target.clone()],
        vec!["gio".to_string(), "open".to_string(), target.clone()],
        vec!["pcmanfm".to_string(), target],
    ]))
}

fn resolve_folder(folder: FolderId) -> Result<String, String> {
    match folder {
        FolderId::Applications => {
            let home = env::var("HOME").map_err(|_| "HOME is not set".to_string())?;
            let path = PathBuf::from(home).join(".local/share/applications");
            fs::create_dir_all(&path)
                .map_err(|err| format!("failed to create {}: {err}", path.display()))?;
            Ok(path.display().to_string())
        }
        FolderId::Home => env::var("HOME").map_err(|_| "HOME is not set".to_string()),
        FolderId::Desktop => {
            let home = env::var("HOME").map_err(|_| "HOME is not set".to_string())?;
            resolve_desktop_dir(&home).map(|path| path.display().to_string())
        }
    }
}

fn send_shortcut(
    app: &tauri::AppHandle,
    memory: &tauri::State<WindowMemory>,
    action: ShortcutId,
) -> ActionResult {
    let key = match action {
        ShortcutId::Undo => "ctrl+z",
        ShortcutId::Cut => "ctrl+x",
        ShortcutId::Copy => "ctrl+c",
        ShortcutId::Paste => "ctrl+v",
        ShortcutId::Clear => "Delete",
        ShortcutId::SelectAll => "ctrl+a",
    };

    hide_menu_popup_window(app, true);
    if let Err(err) = activate_remembered_window(memory) {
        return ActionResult::failure(ActionErrorKind::TargetUnavailable, err);
    }
    ActionResult::from_command_result(run_short_command("xdotool", &["key", key]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserializes_stable_action_identifier() {
        let action: PanelAction =
            serde_json::from_str(r#"{"kind":"send_shortcut","action":"copy"}"#).unwrap();

        assert_eq!(
            action,
            PanelAction::SendShortcut {
                action: ShortcutId::Copy
            }
        );
    }

    #[test]
    fn rejects_unknown_action_identifier() {
        let result = serde_json::from_str::<PanelAction>(r#"{"kind":"delete_everything"}"#);

        assert!(result.is_err());
    }

    #[test]
    fn maps_command_failure_to_structured_result() {
        let result = ActionResult::from_command_result(Err("xmessage failed".to_string()));

        assert!(!result.success);
        assert_eq!(result.error_kind, Some(ActionErrorKind::CommandFailed));
        assert_eq!(result.message.as_deref(), Some("xmessage failed"));
    }

    #[test]
    fn cancelled_result_is_legacy_success() {
        let result = ActionResult::failure(ActionErrorKind::Cancelled, "cancelled");

        assert!(result.into_legacy_result().is_ok());
    }
}
