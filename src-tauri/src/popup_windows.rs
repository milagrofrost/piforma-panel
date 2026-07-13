use crate::panel_window::{format_size_result, MAIN_WINDOW_MIN_HEIGHT, MAIN_WINDOW_MIN_WIDTH};
use gtk::prelude::{ContainerExt, GtkWindowExt, WidgetExt};
use tauri::{Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

pub const PRIMARY_MENU_POPUP_LABEL: &str = "menu-popup";
pub const FLYOUT_MENU_POPUP_LABEL: &str = "menu-flyout";

pub fn ensure_menu_popup_window(app: &tauri::AppHandle) -> Result<tauri::WebviewWindow, String> {
    ensure_popup_window(
        app,
        PRIMARY_MENU_POPUP_LABEL,
        "index.html?popup=menu",
        "PiForma Menu",
    )
}

pub fn ensure_menu_flyout_window(app: &tauri::AppHandle) -> Result<tauri::WebviewWindow, String> {
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

pub fn hide_menu_popup_window(app: &tauri::AppHandle, emit_closed: bool) {
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

pub fn hide_menu_flyout_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window(FLYOUT_MENU_POPUP_LABEL) {
        if let Err(err) = window.hide() {
            eprintln!("failed to hide flyout menu popup: {err}");
        } else {
            println!("flyout hidden");
            log_popup_actual_size_unchecked(&window, FLYOUT_MENU_POPUP_LABEL, "hidden");
        }
    }
}

pub fn resize_menu_popup_window(
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

pub fn reset_popup_gtk_size(window: &tauri::WebviewWindow) -> Result<(), String> {
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

pub fn log_popup_actual_size(
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
