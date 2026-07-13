import type { DesktopApp, MenuAction, MenuItem } from "./panelModel";

export function appleMenu(applications: DesktopApp[], controlPanels: DesktopApp[]): MenuItem[] {
  return [
    { kind: "item", label: "About This PiForma", action: { kind: "show_about" } },
    { kind: "separator" },
    { kind: "submenu", label: "Applications", submenu: "applications", items: desktopAppsToMenuItems(applications, "(No Applications)") },
    {
      kind: "submenu",
      label: "Control Panels",
      submenu: "control_panels",
      items: desktopAppsToMenuItems(controlPanels, "(No Control Panels)")
    },
    { kind: "item", label: "Calculator", action: launchNamedAction(applications, "calculator") }
  ];
}

export function fileMenu(): MenuItem[] {
  return [
    { kind: "item", label: "Open Applications Folder", action: { kind: "open_folder", folder: "applications" } },
    { kind: "item", label: "Open Home Folder", action: { kind: "open_folder", folder: "home" } },
    { kind: "item", label: "Open Desktop", action: { kind: "open_folder", folder: "desktop" } },
    { kind: "separator" },
    { kind: "item", label: "New Terminal Window", action: { kind: "new_terminal_window" } }
  ];
}

export function editMenu(): MenuItem[] {
  return [
    shortcut("Undo", "undo"),
    { kind: "separator" },
    shortcut("Cut", "cut"),
    shortcut("Copy", "copy"),
    shortcut("Paste", "paste"),
    shortcut("Clear", "clear"),
    shortcut("Select All", "select_all"),
    { kind: "separator" },
    { kind: "item", label: "Show Clipboard", action: { kind: "run_system_action", action: "show_clipboard", confirmed: false } }
  ];
}

export function viewMenu(): MenuItem[] {
  return [
    { kind: "item", label: "Show Desktop", action: { kind: "run_system_action", action: "show_desktop", confirmed: false } },
    { kind: "item", label: "Refresh", action: { kind: "run_system_action", action: "refresh", confirmed: false } }
  ];
}

export function specialMenu(): MenuItem[] {
  return [
    { kind: "item", label: "Clean Up Window", action: { kind: "run_system_action", action: "clean_up_window", confirmed: false } },
    { kind: "separator" },
    { kind: "item", label: "Sleep Display", action: { kind: "run_system_action", action: "sleep_display", confirmed: false } },
    { kind: "item", label: "Restart", action: { kind: "confirmed_system_action", action: "restart", message: "Restart PiForma?" } },
    { kind: "item", label: "Shut Down", action: { kind: "confirmed_system_action", action: "shut_down", message: "Shut down PiForma?" } }
  ];
}

function shortcut(label: string, action: string): MenuItem {
  return { kind: "item", label, action: { kind: "send_shortcut", action } };
}

function desktopAppsToMenuItems(apps: DesktopApp[], emptyLabel: string): MenuItem[] {
  const sortedApps = [...apps].sort((a, b) => a.name.localeCompare(b.name, undefined, { sensitivity: "base" }));
  if (sortedApps.length === 0) {
    return [{ kind: "item", label: emptyLabel, enabled: false, action: { kind: "placeholder", message: emptyLabel } }];
  }
  return sortedApps.map((app) => ({
    kind: "item",
    label: app.name,
    action: {
      kind: "launch_app",
      exec: app.exec,
      name: app.name
    }
  }));
}

function launchNamedAction(applications: DesktopApp[], match: string): MenuAction {
  const app = applications.find((item) => item.name.toLowerCase().includes(match));
  if (app) {
    return { kind: "launch_app", exec: app.exec, name: app.name };
  }
  return { kind: "launch_calculator" };
}
