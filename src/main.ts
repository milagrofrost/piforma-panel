import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
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
  | { kind: "item"; label: string; action: MenuAction; enabled?: boolean }
  | { kind: "separator" }
  | { kind: "submenu"; label: string; items: MenuItem[]; scrollable?: boolean };

type MenuAction =
  | { kind: "placeholder"; message: string }
  | { kind: "launch_app"; exec: string; name: string }
  | { kind: "open_folder"; folder: "applications" | "home" | "desktop" }
  | { kind: "new_terminal_window" }
  | { kind: "send_shortcut"; action: string }
  | { kind: "run_system_action"; action: string; confirmed: boolean }
  | { kind: "confirmed_system_action"; action: "restart" | "shut_down"; message: string };

const app = document.querySelector<HTMLDivElement>("#app");

if (!app) {
  throw new Error("missing #app");
}

const appRoot = app;

let config: PanelConfig;
let openMenu: HTMLElement | null = null;
let openButton: HTMLElement | null = null;
let globalMenuListenersInstalled = false;
let openMenuToken = 0;
let isPopupWindow = false;

const menuStorageKey = "piforma-panel-open-menu";

void init();

async function init() {
  isPopupWindow = new URLSearchParams(window.location.search).get("popup") === "menu";
  config = await invoke<PanelConfig>("get_config");
  installGlobalMenuListeners();

  if (isPopupWindow) {
    renderPopupMenu();
    return;
  }

  void listen("menu-popup-closed", () => {
    clearOpenMenuState();
  });
  void listen<{ label: string; action: MenuAction }>("menu-action-selected", (event) => {
    void runMenuAction(event.payload.action).catch(console.error);
  });

  await invoke("initialize_main_window");
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

type StoredMenu = {
  token: number;
  items: MenuItem[];
};

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
    void toggleMenu(appleButton, [
      { kind: "item", label: "About This PiForma", action: { kind: "placeholder", message: "About This PiForma" } },
      { kind: "separator" },
      { kind: "submenu", label: "Applications", items: appItems(applications), scrollable: true },
      { kind: "submenu", label: "Control Panels", items: appItems(controlPanels), scrollable: true },
      { kind: "item", label: "Calculator", action: launchNamedAction(applications, "calculator") }
    ]).catch(console.error);
  });
  left.append(appleButton);

  if (config.menus.show_file) {
    left.append(menuTitle("File", [
      { kind: "item", label: "Open Applications Folder", action: { kind: "open_folder", folder: "applications" } },
      { kind: "item", label: "Open Home Folder", action: { kind: "open_folder", folder: "home" } },
      { kind: "item", label: "Open Desktop", action: { kind: "open_folder", folder: "desktop" } },
      { kind: "separator" },
      { kind: "item", label: "New Terminal Window", action: { kind: "new_terminal_window" } }
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
      { kind: "item", label: "Show Clipboard", action: { kind: "run_system_action", action: "show_clipboard", confirmed: false } }
    ]));
  }

  if (config.menus.show_view) {
    left.append(menuTitle("View", [
      { kind: "item", label: "Show Desktop", action: { kind: "run_system_action", action: "show_desktop", confirmed: false } },
      { kind: "item", label: "Refresh", action: { kind: "run_system_action", action: "refresh", confirmed: false } }
    ]));
  }

  if (config.menus.show_special) {
    left.append(menuTitle("Special", [
      { kind: "item", label: "Clean Up Window", action: { kind: "run_system_action", action: "clean_up_window", confirmed: false } },
      { kind: "separator" },
      { kind: "item", label: "Sleep Display", action: { kind: "run_system_action", action: "sleep_display", confirmed: false } },
      { kind: "item", label: "Restart", action: { kind: "confirmed_system_action", action: "restart", message: "Restart PiForma?" } },
      { kind: "item", label: "Shut Down", action: { kind: "confirmed_system_action", action: "shut_down", message: "Shut down PiForma?" } }
    ]));
  }

  const clock = document.createElement("div");
  clock.id = "clock";
  clock.className = "clock";
  right.append(clock);

  bar.append(left, right);
  appRoot.replaceChildren(bar);
}

function menuTitle(label: string, items: MenuItem[]) {
  const button = makeMenuButton("menu-title");
  button.textContent = label;
  button.addEventListener("click", () => {
    void toggleMenu(button, items).catch(console.error);
  });
  return button;
}

function makeMenuButton(className: string) {
  const button = document.createElement("button");
  button.type = "button";
  button.className = className;
  return button;
}

function shortcut(label: string, action: string): MenuItem {
  return { kind: "item", label, action: { kind: "send_shortcut", action } };
}

function appItems(apps: DesktopApp[]): MenuItem[] {
  if (apps.length === 0) {
    return [{ kind: "item", label: "No Items Found", enabled: false, action: { kind: "placeholder", message: "No Items Found" } }];
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
      action: { kind: "launch_app", exec: app.exec, name: app.name }
    });
    lastGroup = app.group;
  }
  return items;
}

async function toggleMenu(button: HTMLElement, items: MenuItem[]) {
  if (openButton === button) {
    await closeMenu();
    return;
  }

  await closeMenu();
  const rect = button.getBoundingClientRect();
  const size = measureMenu(items);
  const token = openMenuToken + 1;
  openMenuToken = token;
  localStorage.setItem(menuStorageKey, JSON.stringify({ token, items } satisfies StoredMenu));
  openButton = button;
  button.classList.add("is-open");
  await invoke("open_menu_popup", {
    x: config.bar.x + Math.floor(rect.left),
    y: config.bar.y + config.bar.height,
    width: size.width,
    height: size.height
  });
}

function renderPopupMenu() {
  const stored = readStoredMenu();
  if (!stored) {
    void closeMenu().catch(console.error);
    return;
  }

  const menu = buildMenu(stored.items);
  menu.classList.add("popup-menu");
  appRoot.replaceChildren(menu);
}

function readStoredMenu() {
  const raw = localStorage.getItem(menuStorageKey);
  if (!raw) {
    return null;
  }

  try {
    return JSON.parse(raw) as StoredMenu;
  } catch (error) {
    console.error(error);
    return null;
  }
}

function buildMenu(items: MenuItem[], options: { inertActions?: boolean } = {}) {
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
    if (!options.inertActions) {
      row.addEventListener("click", async () => {
        if (isPopupWindow) {
          await invoke("select_menu_action", { label: item.label, action: item.action });
          return;
        }

        await closeMenu();
        await runMenuAction(item.action);
      });
    }
    menu.append(row);
  }

  return menu;
}

function measureMenu(items: MenuItem[]) {
  const menu = buildMenu(items, { inertActions: true });
  menu.classList.add("measure-menu");
  document.body.append(menu);

  const topRect = menu.getBoundingClientRect();
  let width = Math.ceil(topRect.width + 4);
  let height = Math.ceil(topRect.height + 4);

  for (const submenu of menu.querySelectorAll<HTMLElement>(".submenu")) {
    submenu.classList.add("measure-submenu");
    const submenuRect = submenu.getBoundingClientRect();
    width = Math.max(width, Math.ceil(topRect.width + submenuRect.width + 8));
    height = Math.max(height, Math.ceil(submenuRect.height + 4));
    submenu.classList.remove("measure-submenu");
  }

  menu.remove();

  return {
    width: Math.max(1, width),
    height: Math.max(1, Math.min(height, config.applications.max_menu_height + 12))
  };
}

function installGlobalMenuListeners() {
  if (globalMenuListenersInstalled) {
    return;
  }

  document.addEventListener("pointerdown", handleGlobalPointerDown, true);
  document.addEventListener("keydown", handleGlobalKeyDown, true);
  window.addEventListener("blur", () => {
    if (isPopupWindow) {
      void closeMenu().catch(console.error);
    }
  });
  globalMenuListenersInstalled = true;
}

function handleGlobalPointerDown(event: PointerEvent) {
  const target = event.target;
  if (!(target instanceof Node)) {
    return;
  }
  if (!isPopupWindow && openMenu && !openMenu.contains(target) && !openButton?.contains(target)) {
    void closeMenu({ pointerEvent: event }).catch(console.error);
  }
}

function handleGlobalKeyDown(event: KeyboardEvent) {
  if (event.key === "Escape" && (isPopupWindow || openButton)) {
    event.preventDefault();
    void closeMenu().catch(console.error);
  }
}

async function closeMenu(options: { pointerEvent?: PointerEvent } = {}) {
  document.querySelectorAll<HTMLElement>("body > .menu").forEach((menu) => menu.remove());
  clearOpenMenuState();
  clearInteractionState(options.pointerEvent);
  await invoke("close_menu_popup");
}

function clearOpenMenuState() {
  openButton?.classList.remove("is-open");
  openMenu = null;
  openButton = null;
}

function clearInteractionState(pointerEvent?: PointerEvent) {
  const pointerTarget = pointerEvent?.target;
  if (pointerEvent && pointerTarget instanceof Element && pointerTarget.hasPointerCapture(pointerEvent.pointerId)) {
    try {
      pointerTarget.releasePointerCapture(pointerEvent.pointerId);
    } catch (error) {
      console.error(error);
    }
  }

  if (document.activeElement instanceof HTMLElement) {
    document.activeElement.blur();
  }
  window.getSelection()?.removeAllRanges();
}

function launchNamedAction(applications: DesktopApp[], match: string): MenuAction {
  const app = applications.find((item) => item.name.toLowerCase().includes(match));
  if (app) {
    return { kind: "launch_app", exec: app.exec, name: app.name };
  }
  return { kind: "launch_app", exec: "xcalc", name: "Calculator" };
}

async function runMenuAction(action: MenuAction) {
  switch (action.kind) {
    case "placeholder":
      window.alert(action.message);
      return;
    case "launch_app":
      await invoke("launch_app", { exec: action.exec, name: action.name });
      return;
    case "open_folder":
      await invoke("open_folder", { folder: action.folder });
      return;
    case "new_terminal_window":
      await invoke("new_terminal_window");
      return;
    case "send_shortcut":
      await invoke("send_shortcut", { action: action.action });
      return;
    case "run_system_action":
      await invoke("run_system_action", { action: action.action, confirmed: action.confirmed });
      return;
    case "confirmed_system_action":
      if (window.confirm(action.message)) {
        await invoke("run_system_action", { action: action.action, confirmed: true });
      }
  }
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
