import { appleMenu, editMenu, fileMenu, specialMenu, viewMenu } from "./menuDefinitions";
import type { DesktopApp, MenuAction, MenuItem, PanelConfig, PopupGeometry } from "./panelModel";
import { SYSTEM_STATUS_ICON_DATA_URL } from "./systemStatusIcon";

export type PanelRenderHandlers = {
  makeMenuButton(className: string): HTMLButtonElement;
  menuTitle(label: string, items: MenuItem[]): HTMLButtonElement;
  installButtonEventDiagnostics(button: HTMLElement, label: string): void;
  frontendLog(message: string): void;
};

export type MenuRenderHandlers = {
  onItemAction?(label: string, action: MenuAction): void | Promise<void>;
  onPlainItemPointerEnter?(): void | Promise<void>;
  onSubmenuOpen?(row: HTMLElement, item: Extract<MenuItem, { kind: "submenu" }>): void | Promise<void>;
};

export function renderPanel(options: {
  root: HTMLElement;
  config: PanelConfig;
  logo: string | null;
  applications: DesktopApp[];
  controlPanels: DesktopApp[];
  handlers: PanelRenderHandlers;
}) {
  const { root, config, logo, applications, controlPanels, handlers } = options;
  handlers.frontendLog("frontend renderPanel start");
  const bar = document.createElement("div");
  bar.className = "menu-bar";

  const left = document.createElement("div");
  left.className = "menu-left";
  const right = document.createElement("div");
  right.className = "menu-right";

  const appleButton = handlers.makeMenuButton("apple-button");
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
    handlers.frontendLog("Apple handler start before toggleMenu");
    appleButton.dispatchEvent(
      new CustomEvent<MenuItem[]>("piforma:toggle-menu", {
        bubbles: true,
        detail: appleMenu(applications, controlPanels)
      })
    );
  });
  handlers.installButtonEventDiagnostics(appleButton, "Apple");
  left.append(appleButton);

  if (config.menus.show_file) {
    left.append(handlers.menuTitle("File", fileMenu()));
  }

  if (config.menus.show_edit) {
    left.append(handlers.menuTitle("Edit", editMenu()));
  }

  if (config.menus.show_view) {
    left.append(handlers.menuTitle("View", viewMenu()));
  }

  if (config.menus.show_special) {
    left.append(handlers.menuTitle("Special", specialMenu()));
  }

  const clock = document.createElement("div");
  clock.id = "clock";
  clock.className = "clock";

  const statusButton = handlers.makeMenuButton("system-status-button");
  statusButton.setAttribute("aria-label", "System status");
  statusButton.title = "Wireless and Sound";
  const statusImage = document.createElement("img");
  statusImage.src = SYSTEM_STATUS_ICON_DATA_URL;
  statusImage.alt = "";
  statusButton.append(statusImage);
  statusButton.addEventListener("click", () => {
    statusButton.dispatchEvent(
      new CustomEvent("piforma:toggle-system-status", {
        bubbles: true
      })
    );
  });
  handlers.installButtonEventDiagnostics(statusButton, "System status");

  right.append(clock, statusButton);
  bar.append(left, right);
  root.replaceChildren(bar);
  logRenderedMenuDiagnostics(handlers.frontendLog);
}

export function buildMenu(
  items: MenuItem[],
  options: {
    inertActions?: boolean;
    primaryPopup?: PopupGeometry;
    maxHeight?: number;
    handlers?: MenuRenderHandlers;
  } = {}
) {
  const menu = document.createElement("div");
  menu.className = "menu";
  menu.setAttribute("role", "menu");
  if (options.maxHeight) {
    menu.classList.add("scrollable");
  }

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

    if (item.kind === "submenu") {
      row.classList.add("has-submenu");
      row.textContent = item.label;
      const arrow = document.createElement("span");
      arrow.className = "submenu-arrow";
      arrow.setAttribute("aria-hidden", "true");
      row.append(arrow);
      if (!options.inertActions) {
        const openSubmenu = async () => {
          if (!options.primaryPopup) {
            return;
          }
          await options.handlers?.onSubmenuOpen?.(row, item);
        };
        row.addEventListener("pointerenter", () => {
          void openSubmenu().catch(console.error);
        });
        row.addEventListener("click", () => {
          void openSubmenu().catch(console.error);
        });
      }
      menu.append(row);
      continue;
    }

    row.textContent = item.label;
    row.disabled = item.enabled === false;
    if (!options.inertActions) {
      row.addEventListener("pointerenter", () => {
        void options.handlers?.onPlainItemPointerEnter?.();
      });
      row.addEventListener("click", () => {
        void options.handlers?.onItemAction?.(item.label, item.action);
      });
    }
    menu.append(row);
  }

  return menu;
}

export function measureMenu(items: MenuItem[], options: { maxHeight?: number } = {}) {
  const menu = buildMenu(items, { inertActions: true, maxHeight: options.maxHeight });
  menu.classList.add("measure-menu");
  document.body.append(menu);
  const rect = menu.getBoundingClientRect();
  menu.remove();

  return {
    width: Math.max(1, Math.ceil(rect.width)),
    height: Math.max(1, Math.ceil(Math.min(rect.height, options.maxHeight ?? rect.height)))
  };
}

function logRenderedMenuDiagnostics(frontendLog: (message: string) => void) {
  const menuTitles = document.querySelectorAll<HTMLElement>(".menu-title");
  const appleButton = document.querySelector<HTMLElement>(".apple-button");
  frontendLog(`rendered menu-title count=${menuTitles.length}`);
  frontendLog(`rendered apple button exists=${appleButton !== null}`);

  const buttons = document.querySelectorAll<HTMLElement>(".apple-button, .menu-title, .system-status-button");
  buttons.forEach((button, index) => {
    const label = button.getAttribute("aria-label") ?? button.textContent?.trim() ?? "";
    const text = button.textContent?.trim().replace(/\s+/g, " ") ?? "";
    const style = window.getComputedStyle(button);
    const rect = button.getBoundingClientRect();
    frontendLog(`menu button ${index}: label=${label}, text="${text}"`);
    frontendLog(
      `menu button ${index}: pointer-events=${style.pointerEvents}, rect=x:${Math.round(rect.x)}, y:${Math.round(rect.y)}, width:${Math.round(rect.width)}, height:${Math.round(rect.height)}`
    );
  });
}
