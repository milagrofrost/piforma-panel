use crate::launcher::{command_stdout, run_short_command};
use std::{sync::Mutex, thread, time::Duration};

#[derive(Default)]
pub struct WindowMemory {
    previous_active_window: Mutex<Option<String>>,
}

pub fn remember_active_window(
    app: &tauri::AppHandle,
    memory: &tauri::State<WindowMemory>,
) -> Result<(), String> {
    let window_id = command_stdout("xdotool", &["getactivewindow"])?;
    if window_id.is_empty() {
        return Err("xdotool did not return an active window".to_string());
    }
    if is_panel_window(app, &window_id) {
        return Ok(());
    }
    let mut remembered = memory
        .previous_active_window
        .lock()
        .map_err(|_| "remembered window lock poisoned".to_string())?;
    *remembered = Some(window_id);
    Ok(())
}

pub fn activate_remembered_window(memory: &tauri::State<WindowMemory>) -> Result<String, String> {
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
