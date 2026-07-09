import { invoke } from "@tauri-apps/api/core";
import "./styles.css";

type PanelConfig = {
  bar: {
    width: number;
    height: number;
    x: number;
    y: number;
    radius_top_left: number;
    radius_top_right: number;
    font_family: string;
    font_size: number;
  };
  clock: {
    enabled: boolean;
    format: string;
  };
  applications: {
    max_menu_height: number;
  };
  menus: {
    show_file: boolean;
    show_edit: boolean;
    show_view: boolean;
    show_special: boolean;
  };
};

type DesktopApp = {
  id: string;
  name: string;
  exec: string;
  icon?: string;
  categories: string[];
  group: string;
  is_control_panel: boolean;
};

type MenuItem =
  | { kind: "item"; label: string; action: () => void; enabled?: boolean }
  | { kind: "separator" }
  | { kind: "submenu"; label: string; items: MenuItem[]; scrollable?: boolean };

const app = document.querySelector<HTMLDivElement>("#app");

if (!app) {
  throw new Error("missing #app");
}

const appRoot = app;

let config: PanelConfig;
let openMenu: HTMLElement | null = null;
let openButton: HTMLElement | null = null;

void init();

async function init() {
  config = await invoke<PanelConfig>("get_config");
  const logo = await invoke<string | null>("get_apple_logo_data_url");
  const [applications, controlPanels] = await Promise.all([
    invoke<DesktopApp[]>("list_applications"),
    invoke<DesktopApp[]>("list_control_panels")
  ]);

  document.documentElement.style.setProperty("--bar-width", `${config.bar.width}px`);
  document.documentElement.style.setProperty("--bar-height", `${config.bar.height}px`);
  document.documentElement.style.setProperty("--radius-tl", `${config.bar.radius_top_left}px`);
  document.documentElement.style.setProperty("--radius-tr", `${config.bar.radius_top_right}px`);
  document.documentElement.style.setProperty("--panel-font", config.bar.font_family);
  document.documentElement.style.setProperty("--panel-font-size", `${config.bar.font_size}px`);
  document.documentElement.style.setProperty("--menu-max-height", `${config.applications.max_menu_height}px`);

  renderPanel(logo, applications, controlPanels);
  updateClock();
  window.setInterval(updateClock, 1000);
}

function renderPanel(logo: string | null, applications: DesktopApp[], controlPanels: DesktopApp[]) {
  const bar = document.createElement("div");
  bar.className = "menu-bar";

  const left = document.createElement("div");
  left.className = "menu-left";
  const right = document.createElement("div");
  right.className = "menu-right";

  const appleButton = makeMenuButton("apple-button");
  appleButton.setAttribute("aria-label", "Apple menu");
  if (logo) {
    const image = document.createElement("img");
    image.src = logo;
    image.alt = "";
    appleButton.append(image);
  } else {
    appleButton.textContent = "Apple";
    appleButton.classList.add("apple-fallback");
  }
  appleButton.addEventListener("click", () => {
    toggleMenu(appleButton, [
      { kind: "item", label: "About This PiForma", action: () => placeholder("About This PiForma") },
      { kind: "separator" },
      { kind: "submenu", label: "Applications", items: appItems(applications), scrollable: true },
      { kind: "submenu", label: "Control Panels", items: appItems(controlPanels), scrollable: true },
      { kind: "item", label: "Calculator", action: () => launchNamed(applications, "calculator") }
    ]);
  });
  left.append(appleButton);

  if (config.menus.show_file) {
    left.append(menuTitle("File", [
      { kind: "item", label: "Open Applications Folder", action: () => invoke("open_folder", { folder: "applications" }) },
      { kind: "item", label: "Open Home Folder", action: () => invoke("open_folder", { folder: "home" }) },
      { kind: "item", label: "Open Desktop", action: () => invoke("open_folder", { folder: "desktop" }) },
      { kind: "separator" },
      { kind: "item", label: "New Terminal Window", action: () => invoke("new_terminal_window") }
    ]));
  }

  if (config.menus.show_edit) {
    left.append(menuTitle("Edit", [
      shortcut("Undo", "undo"),
      { kind: "separator" },
      shortcut("Cut", "cut"),
      shortcut("Copy", "copy"),
      shortcut("Paste", "paste"),
      shortcut("Clear", "clear"),
      shortcut("Select All", "select_all"),
      { kind: "separator" },
      { kind: "item", label: "Show Clipboard", action: () => invoke("run_system_action", { action: "show_clipboard", confirmed: false }) }
    ]));
  }

  if (config.menus.show_view) {
    left.append(menuTitle("View", [
      { kind: "item", label: "Show Desktop", action: () => invoke("run_system_action", { action: "show_desktop", confirmed: false }) },
      { kind: "item", label: "Refresh", action: () => invoke("run_system_action", { action: "refresh", confirmed: false }) }
    ]));
  }

  if (config.menus.show_special) {
    left.append(menuTitle("Special", [
      { kind: "item", label: "Clean Up Window", action: () => invoke("run_system_action", { action: "clean_up_window", confirmed: false }) },
      { kind: "separator" },
      { kind: "item", label: "Sleep Display", action: () => invoke("run_system_action", { action: "sleep_display", confirmed: false }) },
      { kind: "item", label: "Restart", action: () => confirmedAction("restart", "Restart PiForma?") },
      { kind: "item", label: "Shut Down", action: () => confirmedAction("shut_down", "Shut down PiForma?") }
    ]));
  }

  const clock = document.createElement("div");
  clock.id = "clock";
  clock.className = "clock";
  right.append(clock);

  bar.append(left, right);
  appRoot.replaceChildren(bar);

  document.addEventListener("pointerdown", (event) => {
    const target = event.target;
    if (!(target instanceof Node)) {
      return;
    }
    if (openMenu && !openMenu.contains(target) && !openButton?.contains(target)) {
      closeMenu();
    }
  });
}

function menuTitle(label: string, items: MenuItem[]) {
  const button = makeMenuButton("menu-title");
  button.textContent = label;
  button.addEventListener("click", () => toggleMenu(button, items));
  return button;
}

function makeMenuButton(className: string) {
  const button = document.createElement("button");
  button.type = "button";
  button.className = className;
  return button;
}

function shortcut(label: string, action: string): MenuItem {
  return { kind: "item", label, action: () => invoke("send_shortcut", { action }) };
}

function appItems(apps: DesktopApp[]): MenuItem[] {
  if (apps.length === 0) {
    return [{ kind: "item", label: "No Items Found", enabled: false, action: () => undefined }];
  }

  const items: MenuItem[] = [];
  let lastGroup = "";
  for (const app of apps) {
    if (lastGroup && app.group !== lastGroup) {
      items.push({ kind: "separator" });
    }
    items.push({
      kind: "item",
      label: app.name,
      action: () => invoke("launch_app", { exec: app.exec, name: app.name })
    });
    lastGroup = app.group;
  }
  return items;
}

function toggleMenu(button: HTMLElement, items: MenuItem[]) {
  if (openButton === button) {
    closeMenu();
    return;
  }

  closeMenu();
  const menu = buildMenu(items);
  const rect = button.getBoundingClientRect();
  menu.style.left = `${Math.floor(rect.left)}px`;
  menu.style.top = `${config.bar.height}px`;
  document.body.append(menu);
  openMenu = menu;
  openButton = button;
  button.classList.add("is-open");
  resizeForOpenMenu(menu);
}

function buildMenu(items: MenuItem[]) {
  const menu = document.createElement("div");
  menu.className = "menu";
  menu.setAttribute("role", "menu");

  for (const item of items) {
    if (item.kind === "separator") {
      const separator = document.createElement("div");
      separator.className = "separator";
      menu.append(separator);
      continue;
    }

    if (item.kind === "submenu") {
      const row = document.createElement("div");
      row.className = "menu-item has-submenu";
      row.textContent = item.label;
      const submenu = buildMenu(item.items);
      submenu.classList.add("submenu");
      if (item.scrollable) {
        submenu.classList.add("scrollable");
      }
      row.append(submenu);
      menu.append(row);
      continue;
    }

    const row = document.createElement("button");
    row.type = "button";
    row.className = "menu-item";
    row.textContent = item.label;
    row.disabled = item.enabled === false;
    row.addEventListener("click", () => {
      closeMenu();
      void item.action();
    });
    menu.append(row);
  }

  return menu;
}

function closeMenu() {
  openMenu?.remove();
  openButton?.classList.remove("is-open");
  openMenu = null;
  openButton = null;
  void invoke("resize_panel_window", { menuHeight: null });
}

function resizeForOpenMenu(menu: HTMLElement) {
  const rect = menu.getBoundingClientRect();
  const menuHeight = Math.ceil(rect.height + 4);
  void invoke("resize_panel_window", { menuHeight });
}

function launchNamed(applications: DesktopApp[], match: string) {
  const app = applications.find((item) => item.name.toLowerCase().includes(match));
  if (app) {
    return invoke("launch_app", { exec: app.exec, name: app.name });
  }
  return invoke("launch_app", { exec: "xcalc", name: "Calculator" });
}

function confirmedAction(action: "restart" | "shut_down", message: string) {
  if (window.confirm(message)) {
    return invoke("run_system_action", { action, confirmed: true });
  }
  return undefined;
}

function placeholder(message: string) {
  window.alert(message);
}

function updateClock() {
  const clock = document.querySelector<HTMLDivElement>("#clock");
  if (!clock || !config?.clock.enabled) {
    return;
  }

  const now = new Date();
  const hour = now.getHours();
  const minute = now.getMinutes().toString().padStart(2, "0");
  const hour12 = hour % 12 || 12;
  const ampm = hour >= 12 ? "PM" : "AM";
  clock.textContent = config.clock.format
    .replace("%I", hour12.toString().padStart(2, "0"))
    .replace("%M", minute)
    .replace("%p", ampm);
}
