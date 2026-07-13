use crate::config::{config_dimension, PanelConfig};
use gtk::prelude::{ContainerExt, GtkWindowExt, WidgetExt};

pub const MAIN_WINDOW_MIN_WIDTH: u32 = 1;
pub const MAIN_WINDOW_MIN_HEIGHT: u32 = 1;

pub fn configure_main_window(
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

fn log_main_window_actual_size(window: &tauri::WebviewWindow, phase: &str) {
    println!(
        "main window {phase}: actual inner_size={}; actual outer_size={}",
        format_size_result(window.inner_size()),
        format_size_result(window.outer_size())
    );
}

pub(crate) fn format_size_result(size: tauri::Result<tauri::PhysicalSize<u32>>) -> String {
    match size {
        Ok(size) => format!("{}x{}", size.width, size.height),
        Err(err) => format!("unavailable ({err})"),
    }
}
