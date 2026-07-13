use crate::panel_actions::{ActionErrorKind, ActionResult};
use crate::{
    config::ensure_config,
    launcher::{
        command_exists, run_short_command, spawn_detached_shell, spawn_short_with_fallbacks,
    },
    popup_windows::hide_menu_popup_window,
    window_manager::{activate_remembered_window, WindowMemory},
};
use serde::{Deserialize, Serialize};
use std::process::{Command, Stdio};

#[path = "window_management.rs"]
mod window_management;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SystemActionId {
    SleepDisplay,
    ShowAllWindows,
    ShowDesktop,
    Refresh,
    CleanUpWindow,
    Restart,
    ShutDown,
    ShowClipboard,
}

#[derive(Debug, Clone, Copy)]
enum PowerAction {
    Restart,
    ShutDown,
}

pub fn run_system_action(
    app: &tauri::AppHandle,
    memory: &tauri::State<WindowMemory>,
    action: &str,
    confirmed: bool,
) -> Result<(), String> {
    let action = parse_system_action(action)?;
    run_system_action_id(app, memory, action, confirmed).into_legacy_result()
}

pub fn run_system_action_id(
    app: &tauri::AppHandle,
    memory: &tauri::State<WindowMemory>,
    action: SystemActionId,
    confirmed: bool,
) -> ActionResult {
    match action {
        SystemActionId::SleepDisplay => ActionResult::from_command_result(
            spawn_short_with_fallbacks(&[
                vec!["xset".to_string(), "dpms".to_string(), "force".to_string(), "off".to_string()],
                vec!["xset".to_string(), "s".to_string(), "activate".to_string()],
            ]),
        ),
        SystemActionId::ShowAllWindows => {
            hide_menu_popup_window(app, true);
            ActionResult::from_command_result(window_management::show_all_windows())
        }
        SystemActionId::ShowDesktop => {
            hide_menu_popup_window(app, true);
            ActionResult::from_command_result(window_management::show_desktop())
        }
        SystemActionId::Refresh => {
            hide_menu_popup_window(app, true);
            if activate_remembered_window(memory).is_ok() {
                ActionResult::from_command_result(run_short_command("xdotool", &["key", "F5"]))
            } else {
                ActionResult::from_command_result(run_short_command("xrefresh", &[]))
            }
        }
        SystemActionId::CleanUpWindow => {
            hide_menu_popup_window(app, true);
            let config = match ensure_config() {
                Ok(config) => config,
                Err(err) => return ActionResult::failure(ActionErrorKind::InvalidRequest, err),
            };
            if let Err(err) = activate_remembered_window(memory) {
                eprintln!("clean up window: no remembered target to reactivate: {err}");
            }
            if command_exists("xdotool") {
                let _ = run_short_command("xdotool", &["key", "F5"]);
            }
            if config.actions.clean_up_window_command.trim().is_empty() {
                ActionResult::success(None)
            } else {
                ActionResult::from_command_result(spawn_detached_shell(
                    &config.actions.clean_up_window_command,
                ))
            }
        }
        SystemActionId::ShowClipboard => ActionResult::from_command_result(show_clipboard()),
        SystemActionId::Restart => {
            hide_menu_popup_window(app, true);
            if !confirmed {
                return ActionResult::failure(
                    ActionErrorKind::InvalidRequest,
                    "restart was not explicitly selected",
                );
            }
            run_power_action(PowerAction::Restart)
        }
        SystemActionId::ShutDown => {
            hide_menu_popup_window(app, true);
            if !confirmed {
                return ActionResult::failure(
                    ActionErrorKind::InvalidRequest,
                    "shutdown was not explicitly selected",
                );
            }
            run_power_action(PowerAction::ShutDown)
        }
    }
}

pub fn confirm_system_action(app: &tauri::AppHandle, action: &str) -> Result<(), String> {
    confirm_system_action_result(app, action).into_legacy_result()
}

pub fn confirm_system_action_result(app: &tauri::AppHandle, action: &str) -> ActionResult {
    hide_menu_popup_window(app, true);

    let action = match parse_system_action(action) {
        Ok(action) => action,
        Err(err) => return ActionResult::failure(ActionErrorKind::InvalidRequest, err),
    };
    let confirmation = match action {
        SystemActionId::Restart => ConfirmationSpec {
            title: "Restart",
            text: "Are you sure you want to restart PiForma?",
            ok_label: "Restart",
            xmessage_buttons: "Cancel:1,Restart:0",
        },
        SystemActionId::ShutDown => ConfirmationSpec {
            title: "Shut Down",
            text: "Are you sure you want to shut down PiForma?",
            ok_label: "Shut Down",
            xmessage_buttons: "Cancel:1,Shut Down:0",
        },
        _ => {
            return ActionResult::failure(
                ActionErrorKind::InvalidRequest,
                "confirmation is only supported for restart and shut_down",
            )
        }
    };

    let confirmed = match confirm_with_desktop_dialog(&confirmation) {
        Ok(confirmed) => confirmed,
        Err(err) => return ActionResult::failure(ActionErrorKind::CommandFailed, err),
    };
    if !confirmed {
        return ActionResult::failure(ActionErrorKind::Cancelled, "action cancelled");
    }

    match action {
        SystemActionId::Restart => run_power_action(PowerAction::Restart),
        SystemActionId::ShutDown => run_power_action(PowerAction::ShutDown),
        _ => unreachable!("non-power actions returned before confirmation"),
    }
}

fn run_power_action(action: PowerAction) -> ActionResult {
    let verb = match action {
        PowerAction::Restart => "reboot",
        PowerAction::ShutDown => "poweroff",
    };
    let label = match action {
        PowerAction::Restart => "restart",
        PowerAction::ShutDown => "shut down",
    };
    let commands = [
        ("systemctl", vec!["--no-ask-password", verb]),
        ("loginctl", vec!["--no-ask-password", verb]),
    ];

    let mut errors = Vec::new();
    let mut authorization_failed = false;

    for (program, args) in commands {
        if !command_exists(program) {
            errors.push(format!("{program} not found"));
            continue;
        }

        let output = match Command::new(program)
            .args(&args)
            .stdin(Stdio::null())
            .output()
        {
            Ok(output) => output,
            Err(err) => {
                errors.push(format!("failed to run {program}: {err}"));
                continue;
            }
        };

        if output.status.success() {
            return ActionResult::success(Some(format!("system {label} requested")));
        }

        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let details = if !stderr.is_empty() {
            stderr
        } else if !stdout.is_empty() {
            stdout
        } else {
            format!("exited with status {}", output.status)
        };
        if is_authorization_error(&details) {
            authorization_failed = true;
        }
        errors.push(format!("{program} {}: {details}", args.join(" ")));
    }

    let message = format!(
        "could not {label} the system: {}",
        if errors.is_empty() {
            "no supported power command is available".to_string()
        } else {
            errors.join("; ")
        }
    );

    if authorization_failed {
        ActionResult::failure(ActionErrorKind::AuthorizationFailed, message)
    } else {
        ActionResult::failure(ActionErrorKind::CommandFailed, message)
    }
}

fn is_authorization_error(message: &str) -> bool {
    let message = message.to_ascii_lowercase();
    [
        "access denied",
        "authentication is required",
        "interactive authentication required",
        "not authorized",
        "not permitted",
        "permission denied",
    ]
    .iter()
    .any(|needle| message.contains(needle))
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

fn parse_system_action(action: &str) -> Result<SystemActionId, String> {
    match action {
        "sleep_display" => Ok(SystemActionId::SleepDisplay),
        "show_all_windows" => Ok(SystemActionId::ShowAllWindows),
        "show_desktop" => Ok(SystemActionId::ShowDesktop),
        "refresh" => Ok(SystemActionId::Refresh),
        "clean_up_window" => Ok(SystemActionId::CleanUpWindow),
        "restart" => Ok(SystemActionId::Restart),
        "shut_down" => Ok(SystemActionId::ShutDown),
        "show_clipboard" => Ok(SystemActionId::ShowClipboard),
        _ => Err(format!("unknown system action: {action}")),
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_common_authorization_errors() {
        for message in [
            "Access denied",
            "Authentication is required",
            "Interactive authentication required",
            "Permission denied",
        ] {
            assert!(is_authorization_error(message));
        }
        assert!(!is_authorization_error("connection timed out"));
    }
}
