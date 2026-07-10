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
  | { kind: "separator" };

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
let openButton: HTMLElement | null = null;
let globalMenuListenersInstalled = false;
let isPopupWindow = false;

const menuStorageKey = "piforma-panel-open-menu";

void init();

function serializeLogValue(value: unknown) {
  if (value instanceof Error) {
    return `${value.name}: ${value.message}`;
  }
  if (typeof value === "string") {
    return value;
  }
  try {
    return JSON.stringify(value);
  } catch {
    return String(value);
  }
}

async function frontendLog(message: string) {
  console.log(message);
  try {
    await invoke("frontend_log", { message });
    return true;
  } catch (error) {
    console.error("frontend_log failed", error);
    return false;
  }
}

async function init() {
  const params = new URLSearchParams(window.location.search);
  isPopupWindow = params.get("popup") === "menu";
  config = await invoke<PanelConfig>("get_config");
  const frontendLogAvailable = await frontendLog("frontend init start");
  if (!frontendLogAvailable) {
    console.error("frontend init start failed to reach Rust frontend_log");
    document.title = "PiForma Panel frontend_log failed";
    document.documentElement.dataset.debug = "frontend-log-failed";
  }
  document.documentElement.style.setProperty("--bar-width", `${config.bar.width}px`);
  document.documentElement.style.setProperty("--bar-height", `${config.bar.height}px`);
  document.documentElement.style.setProperty("--radius-tl", `${config.bar.radius_top_left}px`);
  document.documentElement.style.setProperty("--radius-tr", `${config.bar.radius_top_right}px`);
  document.documentElement.style.setProperty("--panel-font", config.bar.font_family);
  document.documentElement.style.setProperty("--panel-font-size", `${config.bar.font_size}px`);
  document.documentElement.style.setProperty("--menu-max-height", `${config.applications.max_menu_height}px`);

  installGlobalMenuListeners();

  if (isPopupWindow) {
    void frontendLog("popup mode init");
    document.body.classList.add("popup-window");
    renderPopupMenu();
    return;
  }

  void listen<{ label: string; action: MenuAction }>("menu-action-selected", (event) => {
    clearOpenMenuState();
    void runMenuAction(event.payload.action).catch(console.error);
  });
  void listen("menu-popup-closed", () => {
    clearOpenMenuState();
  });

  await invoke("initialize_main_window");
  const logo = await invoke<string | null>("get_apple_logo_data_url");
  const applications = await invoke<DesktopApp[]>("list_applications");

  renderPanel(logo, applications);
  updateClock();
  window.setInterval(updateClock, 1000);
}

type StoredMenu = {
  label: string;
  items: MenuItem[];
};

function renderPanel(logo: string | null, applications: DesktopApp[]) {
  void frontendLog("frontend renderPanel start");
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
    void frontendLog("Apple handler start before toggleMenu");
    void toggleMenu(appleButton, [
      { kind: "item", label: "About This PiForma", action: { kind: "placeholder", message: "About This PiForma" } },
      { kind: "separator" },
      { kind: "item", label: "Applications", enabled: false, action: { kind: "placeholder", message: "Applications" } },
      { kind: "item", label: "Control Panels", enabled: false, action: { kind: "placeholder", message: "Control Panels" } },
      { kind: "item", label: "Calculator", action: launchNamedAction(applications, "calculator") }
    ]).catch(console.error);
  });
  addButtonEventDiagnostics(appleButton, "Apple");
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
  logRenderedMenuDiagnostics();
}

function menuTitle(label: string, items: MenuItem[]) {
  void frontendLog(`menuTitle created: ${label}`);
  const button = makeMenuButton("menu-title");
  button.textContent = label;
  button.addEventListener("click", () => {
    void frontendLog(`menuTitle click handler start: ${label}`);
    void toggleMenu(button, items).catch(console.error);
  });
  addButtonEventDiagnostics(button, label);
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

async function toggleMenu(button: HTMLElement, items: MenuItem[]) {
  console.log("toggleMenu start");
  await frontendLog("toggleMenu start");
  if (openButton === button) {
    await closeMenu();
    return;
  }

  await closeMenu();
  const rect = button.getBoundingClientRect();
  const menu = measureMenu(items);
  const label = button.getAttribute("aria-label") ?? button.textContent ?? "Menu";
  const x = config.bar.x + Math.floor(rect.left);
  const y = config.bar.y + config.bar.height;
  await frontendLog(`menu label=${label}`);
  await frontendLog(`menu item count=${items.length}`);
  await frontendLog(`measured menu width=${menu.width}, height=${menu.height}`);
  await frontendLog(`computed popup x=${x}, y=${y}`);
  localStorage.setItem(menuStorageKey, JSON.stringify({ label, items } satisfies StoredMenu));
  openButton = button;
  button.classList.add("is-open");
  try {
    await frontendLog("before invoking open_menu_popup");
    await invoke("open_menu_popup", {
      label,
      x,
      y,
      width: menu.width,
      height: menu.height
    });
    await frontendLog("after successful open_menu_popup");
  } catch (error) {
    await frontendLog(`open_menu_popup error: ${serializeLogValue(error)}`);
    openButton = null;
    button.classList.remove("is-open");
  }
}

function renderPopupMenu() {
  const stored = readStoredMenu();
  if (!stored) {
    void closeMenu().catch(console.error);
    return;
  }

  const menu = buildMenu(stored.items);
  appRoot.replaceChildren(menu);
  void frontendLog(`popup rendered item count=${stored.items.length}`);
}

function readStoredMenu() {
  const raw = localStorage.getItem(menuStorageKey);
  if (!raw) {
    void frontendLog("popup readStoredMenu failure: missing stored menu");
    return null;
  }

  try {
    const stored = JSON.parse(raw) as StoredMenu;
    void frontendLog(`popup readStoredMenu success: label=${stored.label}, items=${stored.items.length}`);
    return stored;
  } catch (error) {
    console.error(error);
    void frontendLog(`popup readStoredMenu failure: ${serializeLogValue(error)}`);
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
  const rect = menu.getBoundingClientRect();
  menu.remove();

  return {
    width: Math.max(1, Math.ceil(rect.width)),
    height: Math.max(1, Math.ceil(rect.height))
  };
}

function installGlobalMenuListeners() {
  if (globalMenuListenersInstalled) {
    return;
  }

  installEventPathDiagnostics();
  document.addEventListener("pointerdown", handleGlobalPointerDown, true);
  document.addEventListener("keydown", handleGlobalKeyDown, true);
  window.addEventListener("blur", () => {
    if (isPopupWindow) {
      void frontendLog("popup blur");
      void closeMenu().catch(console.error);
    }
  });
  globalMenuListenersInstalled = true;
}

function installEventPathDiagnostics() {
  for (const eventType of ["pointerdown", "mousedown", "mouseup", "click"]) {
    document.addEventListener(
      eventType,
      (event) => {
        if (!(event instanceof MouseEvent)) {
          return;
        }
        void frontendLog(`document event: ${formatMouseEvent(event)}`);
      },
      true
    );
  }
}

function addButtonEventDiagnostics(button: HTMLElement, label: string) {
  for (const eventType of ["pointerdown", "mousedown", "mouseup", "click"]) {
    button.addEventListener(eventType, () => {
      void frontendLog(`button event: ${label} ${eventType}`);
    });
  }
}

function formatMouseEvent(event: MouseEvent) {
  const target = event.target;
  const element = target instanceof Element ? target : null;
  const tagName = element?.tagName ?? "unknown";
  const className = element ? String(element.className) : "";
  const text = element?.textContent?.trim().replace(/\s+/g, " ").slice(0, 40) ?? "";
  return `${event.type} target=${tagName} class=${className} text="${text}" x=${event.clientX} y=${event.clientY}`;
}

function logRenderedMenuDiagnostics() {
  const menuTitles = document.querySelectorAll<HTMLElement>(".menu-title");
  const appleButton = document.querySelector<HTMLElement>(".apple-button");
  void frontendLog(`rendered menu-title count=${menuTitles.length}`);
  void frontendLog(`rendered apple button exists=${appleButton !== null}`);

  const buttons = document.querySelectorAll<HTMLElement>(".apple-button, .menu-title");
  buttons.forEach((button, index) => {
    const label = button.getAttribute("aria-label") ?? button.textContent?.trim() ?? "";
    const text = button.textContent?.trim().replace(/\s+/g, " ") ?? "";
    const style = window.getComputedStyle(button);
    const rect = button.getBoundingClientRect();
    void frontendLog(`menu button ${index}: label=${label}, text="${text}"`);
    void frontendLog(
      `menu button ${index}: pointer-events=${style.pointerEvents}, rect=x:${Math.round(rect.x)}, y:${Math.round(rect.y)}, width:${Math.round(rect.width)}, height:${Math.round(rect.height)}`
    );
  });
}

function handleGlobalPointerDown(event: PointerEvent) {
  const target = event.target;
  if (!(target instanceof Node)) {
    return;
  }
  if (!isPopupWindow && openButton && !openButton.contains(target)) {
    console.log("outside-click close: main window pointerdown outside active menu button");
    void closeMenu({ pointerEvent: event }).catch(console.error);
  }
  if (isPopupWindow && target instanceof Element && !target.closest(".menu")) {
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
  await frontendLog("closeMenu start");
  document.querySelectorAll<HTMLElement>("body > .menu").forEach((menu) => menu.remove());
  clearOpenMenuState();
  clearInteractionState(options.pointerEvent);
  await invoke("close_menu_popup");
  await frontendLog("closeMenu end");
}

function clearOpenMenuState() {
  openButton?.classList.remove("is-open");
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
