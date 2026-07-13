use crate::{
    config::PanelConfig,
    panel_model::{MonitorGeometry, PanelGeometry},
    shell_identity::{apply_shell_window_identity, ShellWindowRole},
};
use gtk::prelude::{ContainerExt, GtkWindowExt, WidgetExt};

pub const MAIN_WINDOW_MIN_WIDTH: u32 = 1;
pub const MAIN_WINDOW_MIN_HEIGHT: u32 = 1;

pub fn configure_main_window(
    window: &tauri::WebviewWindow,
    config: &PanelConfig,
    phase: &str,
) -> Result<PanelGeometry, String> {
    let geometry = effective_panel_geometry(Some(window), config)?;
    apply_shell_window_identity(window, ShellWindowRole::MainPanel)?;
    let verbose = crate::diagnostics::verbose_for_config(config);
    if verbose {
        log_panel_geometry(config, &geometry);
    }
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
        .set_size(tauri::Size::Physical(tauri::PhysicalSize {
            width: geometry.width,
            height: geometry.height,
        }))
        .map_err(|err| err.to_string())?;
    apply_main_gtk_size(window, geometry.width, geometry.height, verbose)?;
    window
        .set_position(tauri::Position::Physical(tauri::PhysicalPosition {
            x: geometry.x,
            y: geometry.y,
        }))
        .map_err(|err| err.to_string())?;
    if verbose {
        log_main_window_actual_size(window, phase);
    }
    window.show().map_err(|err| err.to_string())?;
    window.set_resizable(false).map_err(|err| err.to_string())?;
    Ok(geometry)
}

pub fn effective_panel_geometry(
    window: Option<&tauri::WebviewWindow>,
    config: &PanelConfig,
) -> Result<PanelGeometry, String> {
    PanelGeometry::from_config(config, window.and_then(monitor_geometry))
}

fn monitor_geometry(window: &tauri::WebviewWindow) -> Option<MonitorGeometry> {
    let Ok(Some(monitor)) = window.current_monitor() else {
        return None;
    };
    Some(MonitorGeometry {
        id: monitor.name().map(ToString::to_string),
        origin_x: monitor.position().x,
        origin_y: monitor.position().y,
        width: Some(monitor.size().width),
        height: Some(monitor.size().height),
        scale_factor: monitor.scale_factor(),
    })
}

fn log_panel_geometry(config: &PanelConfig, geometry: &PanelGeometry) {
    println!(
        "panel geometry configured: x={}, y={}, width={}, height={}",
        config.bar.x, config.bar.y, config.bar.width, config.bar.height
    );
    println!(
        "panel geometry effective: x={}, y={}, width={}, height={}, monitor_id={}, monitor_origin={},{} monitor_size={}x{}, scale_factor={}, coordinate_space={}",
        geometry.x,
        geometry.y,
        geometry.width,
        geometry.height,
        geometry.monitor_id.as_deref().unwrap_or("unknown"),
        geometry.monitor_origin_x,
        geometry.monitor_origin_y,
        geometry
            .monitor_width
            .map(|value| value.to_string())
            .unwrap_or_else(|| "unknown".to_string()),
        geometry
            .monitor_height
            .map(|value| value.to_string())
            .unwrap_or_else(|| "unknown".to_string()),
        geometry.scale_factor,
        geometry.coordinate_space
    );
}

fn apply_main_gtk_size(
    window: &tauri::WebviewWindow,
    width: u32,
    height: u32,
    verbose: bool,
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

    if verbose {
        println!("gtk tight size applied: label=main, width={width}, height={height}");
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

pub(crate) fn format_size_result(size: tauri::Result<tauri::PhysicalSize<u32>>) -> String {
    match size {
        Ok(size) => format!("{}x{}", size.width, size.height),
        Err(err) => format!("unavailable ({err})"),
    }
}
