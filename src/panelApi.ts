import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { ActionResult, BackendPanelAction, DesktopApp, MenuAction, PanelConfig, PanelState, RenderMenuPopupPayload, SystemStatus } from "./panelModel";

let verboseDiagnostics = false;

export function setVerboseDiagnostics(enabled: boolean) {
  verboseDiagnostics = enabled;
}

export async function frontendLog(message: string) {
  if (!verboseDiagnostics) {
    return true;
  }
  console.debug(message);
  try {
    await invoke("frontend_log", { message, verbose: true });
    return true;
  } catch (error) {
    console.error("frontend_log failed", error);
    return false;
  }
}

export async function frontendStatus(message: string) {
  console.info(message);
  try {
    await invoke("frontend_log", { message, verbose: false });
    return true;
  } catch (error) {
    console.error("frontend_log failed", error);
    return false;
  }
}

export async function getConfig() {
  return await invoke<PanelConfig>("get_config");
}

export async function getPanelState() {
  return await invoke<PanelState>("get_panel_state");
}

export async function getSystemStatus() {
  return await invoke<SystemStatus>("get_system_status");
}

export async function setSystemVolume(volume: number) {
  await invoke("set_system_volume", { volume: Math.round(volume) });
}

export async function openNetworkSettings() {
  await invoke("open_network_settings");
}

export async function initializeMainWindow() {
  await invoke("initialize_main_window");
}

export async function getAppleLogoDataUrl() {
  return await invoke<string | null>("get_apple_logo_data_url");
}

export async function listApplications() {
  return await invoke<DesktopApp[]>("list_applications");
}

export async function listControlPanels() {
  return await invoke<DesktopApp[]>("list_control_panels");
}

export async function rememberActiveWindow() {
  await invoke("remember_active_window");
}

export async function openMenuPopup(payload: {
  label: string;
  x: number;
  y: number;
  width: number;
  height: number;
  items: unknown;
}) {
  await invoke("open_menu_popup", payload);
}

export async function openMenuFlyout(payload: {
  label: string;
  submenu: string;
  x: number;
  y: number;
  width: number;
  height: number;
  items: unknown;
}) {
  await invoke("open_menu_flyout", payload);
}

export async function closeMenuPopup() {
  await invoke("close_menu_popup");
}

export async function closeMenuFlyout() {
  await invoke("close_menu_flyout");
}

export async function menuPopupRendered(payload: { label: string; width: number; height: number }) {
  await invoke("menu_popup_rendered", payload);
}

export async function menuFlyoutRendered(payload: { label: string; width: number; height: number }) {
  await invoke("menu_flyout_rendered", payload);
}

export async function menuFlyoutPointerEntered() {
  await invoke("menu_flyout_pointer_entered");
}

export async function selectMenuAction(label: string, action: MenuAction) {
  await invoke("select_menu_action", { label, action });
}

export async function runMenuAction(action: MenuAction) {
  switch (action.kind) {
    case "placeholder":
      window.alert(action.message);
      return { success: true } satisfies ActionResult;
    case "confirmed_system_action":
      return await invoke<ActionResult>("confirm_system_action_result", { action: action.action });
    default:
      return await executePanelAction(action);
  }
}

async function executePanelAction(action: BackendPanelAction) {
  return await invoke<ActionResult>("execute_panel_action", { action });
}

export async function onMenuActionSelected(
  handler: (payload: { label: string; action: MenuAction }) => void
) {
  return await listen<{ label: string; action: MenuAction }>("menu-action-selected", (event) => {
    handler(event.payload);
  });
}

export async function onMenuPopupClosed(handler: () => void) {
  return await listen("menu-popup-closed", handler);
}

export async function onRenderMenuPopup(handler: (payload: RenderMenuPopupPayload) => void | Promise<void>) {
  return await listen<RenderMenuPopupPayload>("render-menu-popup", (event) => {
    void handler(event.payload);
  });
}

export async function onRenderMenuFlyout(handler: (payload: RenderMenuPopupPayload) => void | Promise<void>) {
  return await listen<RenderMenuPopupPayload>("render-menu-flyout", (event) => {
    void handler(event.payload);
  });
}

export async function onMenuFlyoutEntered(handler: () => void) {
  return await listen("menu-flyout-entered", handler);
}

export async function onMenuFlyoutRendered(handler: () => void) {
  return await listen("menu-flyout-rendered", handler);
}
