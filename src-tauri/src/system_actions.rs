use crate::{
    config::ensure_config,
    launcher::{
        command_exists, run_short_command, run_short_with_fallbacks, spawn_detached_shell,
        spawn_short_with_fallbacks,
    },
    popup_windows::hide_menu_popup_window,
    window_manager::{activate_remembered_window, WindowMemory},
};
use std::process::{Command, Stdio};

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

pub fn run_system_action(
    app: &tauri::AppHandle,
    memory: &tauri::State<WindowMemory>,
    action: &str,
    confirmed: bool,
) -> Result<(), String> {
    let action = parse_system_action(action)?;
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
            hide_menu_popup_window(app, true);
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
            hide_menu_popup_window(app, true);
            if activate_remembered_window(memory).is_ok() {
                run_short_command("xdotool", &["key", "F5"])
            } else {
                run_short_command("xrefresh", &[])
            }
        }
        SystemAction::CleanUpWindow => {
            hide_menu_popup_window(app, true);
            let config = ensure_config()?;
            if let Err(err) = activate_remembered_window(memory) {
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

pub fn confirm_system_action(app: &tauri::AppHandle, action: &str) -> Result<(), String> {
    hide_menu_popup_window(app, true);

    let action = parse_system_action(action)?;
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
