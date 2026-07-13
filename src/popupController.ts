import * as api from "./panelApi";
import { renderPanel, buildMenu, measureMenu } from "./menuRenderer";
import type { DesktopApp, MenuAction, MenuItem, PanelConfig, PanelGeometry, PopupGeometry, PopupMode, RenderMenuPopupPayload, SubmenuKind, SystemStatus } from "./panelModel";
import { serializeLogValue } from "./panelModel";

const DEBUG_EVENT_DIAGNOSTICS = false;
const SYSTEM_STATUS_LABEL = "System Status";
const SYSTEM_STATUS_POPUP_WIDTH = 222;
const SYSTEM_STATUS_POPUP_HEIGHT = 58;

export class PopupController {
  private openButton: HTMLElement | null = null;
  private globalMenuListenersInstalled = false;
  private popupBlurCloseTimer: number | null = null;
  private activePrimaryPopup: PopupGeometry | null = null;
  private activeFlyoutSubmenu: SubmenuKind | null = null;
  private activeFlyoutRow: HTMLElement | null = null;
  private flyoutOpen = false;

  constructor(
    private readonly root: HTMLElement,
    private readonly config: PanelConfig,
    private readonly geometry: PanelGeometry,
    private readonly popupMode: PopupMode
  ) {}

  installGlobalMenuListeners() {
    if (this.globalMenuListenersInstalled) {
      return;
    }

    this.installEventPathDiagnostics();
    document.addEventListener("pointerdown", (event) => this.handleGlobalPointerDown(event), true);
    document.addEventListener("keydown", (event) => this.handleGlobalKeyDown(event), true);
    window.addEventListener("blur", () => {
      if (this.popupMode === "menu" || this.popupMode === "flyout") {
        void api.frontendLog("popup blur");
        this.cancelPopupBlurClose();
        this.popupBlurCloseTimer = window.setTimeout(() => {
          void this.closeMenu().catch(console.error);
        }, this.flyoutOpen ? 220 : 160);
      }
    });
    this.globalMenuListenersInstalled = true;
  }

  async initializeMainPanel(logo: string | null, applications: DesktopApp[], controlPanels: DesktopApp[]) {
    await api.onMenuActionSelected((payload) => {
      this.clearOpenMenuState();
      void this.runMenuAction(payload.action).catch(console.error);
    });
    await api.onMenuPopupClosed(() => {
      this.clearOpenMenuState();
    });

    renderPanel({
      root: this.root,
      config: this.config,
      logo,
      applications,
      controlPanels,
      handlers: {
        makeMenuButton: (className) => this.makeMenuButton(className),
        menuTitle: (label, items) => this.menuTitle(label, items),
        installButtonEventDiagnostics: (button, label) => this.installButtonEventDiagnostics(button, label),
        frontendLog: (message) => {
          void api.frontendLog(message);
        }
      }
    });

    this.root.addEventListener("piforma:toggle-menu", (event) => {
      if (!(event instanceof CustomEvent) || !Array.isArray(event.detail)) {
        return;
      }
      const target = event.target;
      if (target instanceof HTMLElement) {
        void this.toggleMenu(target, event.detail as MenuItem[]).catch(console.error);
      }
    });

    this.root.addEventListener("piforma:toggle-system-status", (event) => {
      const target = event.target;
      if (target instanceof HTMLElement) {
        void this.toggleSystemStatus(target).catch(console.error);
      }
    });
  }

  async initializePrimaryPopupWindow() {
    this.root.replaceChildren();
    await api.onRenderMenuPopup(async (payload) => {
      this.cancelPopupBlurClose();
      this.flyoutOpen = false;
      this.activeFlyoutSubmenu = null;
      if (payload.label === SYSTEM_STATUS_LABEL) {
        await this.renderSystemStatusPopup(payload);
      } else {
        this.renderPopupMenu(payload);
      }
      await api.menuPopupRendered({
        label: payload.label,
        width: payload.width,
        height: payload.height
      });
    });
    await api.onMenuFlyoutEntered(() => {
      this.cancelPopupBlurClose();
    });
    await api.onMenuFlyoutRendered(() => {
      this.cancelPopupBlurClose();
      this.flyoutOpen = true;
    });
    void api.frontendLog("popup waiting for render-menu-popup");
  }

  async initializeFlyoutWindow() {
    this.root.replaceChildren();
    await api.onRenderMenuFlyout(async (payload) => {
      this.cancelPopupBlurClose();
      this.renderFlyoutMenu(payload);
      await api.menuFlyoutRendered({
        label: payload.label,
        width: payload.width,
        height: payload.height
      });
    });
    void api.frontendLog("flyout waiting for render-menu-flyout");
  }

  private makeMenuButton(className: string) {
    const button = document.createElement("button");
    button.type = "button";
    button.className = className;
    button.addEventListener("pointerdown", () => {
      void this.rememberActiveWindow();
    });
    return button;
  }

  private menuTitle(label: string, items: MenuItem[]) {
    void api.frontendLog(`menuTitle created: ${label}`);
    const button = this.makeMenuButton("menu-title");
    button.textContent = label;
    button.addEventListener("click", () => {
      void api.frontendLog(`menuTitle click handler start: ${label}`);
      void this.toggleMenu(button, items).catch(console.error);
    });
    this.installButtonEventDiagnostics(button, label);
    return button;
  }

  private async toggleMenu(button: HTMLElement, items: MenuItem[]) {
    await api.frontendLog("toggleMenu start");
    if (this.openButton === button) {
      await this.closeMenu();
      return;
    }

    if (this.openButton) {
      this.openButton.classList.remove("is-open");
    }
    const rect = button.getBoundingClientRect();
    const menu = measureMenu(items);
    const label = button.getAttribute("aria-label") ?? button.textContent ?? "Menu";
    const x = this.geometry.x + Math.floor(rect.left);
    const y = this.geometry.y + this.geometry.height;
    await api.frontendLog(`menu label=${label}`);
    await api.frontendLog(`menu item count=${items.length}`);
    await api.frontendLog(`measured menu width=${menu.width}, height=${menu.height}`);
    await api.frontendLog(`computed popup x=${x}, y=${y}`);
    this.openButton = button;
    button.classList.add("is-open");
    try {
      await this.rememberActiveWindow();
      await api.frontendLog("before invoking open_menu_popup");
      await api.openMenuPopup({
        label,
        x,
        y,
        width: menu.width,
        height: menu.height,
        items
      });
      await api.frontendLog("after successful open_menu_popup");
    } catch (error) {
      await api.frontendLog(`open_menu_popup error: ${serializeLogValue(error)}`);
      this.openButton = null;
      button.classList.remove("is-open");
    }
  }

  private async toggleSystemStatus(button: HTMLElement) {
    await api.frontendLog("toggleSystemStatus start");
    if (this.openButton === button) {
      await this.closeMenu();
      return;
    }

    this.openButton?.classList.remove("is-open");
    const rect = button.getBoundingClientRect();
    const popupRight = this.geometry.x + Math.floor(rect.right);
    const x = Math.max(this.geometry.monitor_origin_x, popupRight - SYSTEM_STATUS_POPUP_WIDTH);
    const y = this.geometry.y + this.geometry.height;

    this.openButton = button;
    button.classList.add("is-open");
    try {
      await this.rememberActiveWindow();
      await api.openMenuPopup({
        label: SYSTEM_STATUS_LABEL,
        x,
        y,
        width: SYSTEM_STATUS_POPUP_WIDTH,
        height: SYSTEM_STATUS_POPUP_HEIGHT,
        items: []
      });
    } catch (error) {
      await api.frontendLog(`system status popup error: ${serializeLogValue(error)}`);
      this.openButton = null;
      button.classList.remove("is-open");
    }
  }

  private async rememberActiveWindow() {
    if (this.popupMode !== "main") {
      return;
    }
    try {
      await api.rememberActiveWindow();
    } catch (error) {
      await api.frontendLog(`remember_active_window failed: ${serializeLogValue(error)}`);
    }
  }

  private renderPopupMenu(payload: RenderMenuPopupPayload) {
    this.activePrimaryPopup = { x: payload.x, y: payload.y, width: payload.width, height: payload.height };
    const menu = buildMenu(payload.items, {
      primaryPopup: this.activePrimaryPopup,
      handlers: this.menuHandlers()
    });
    this.root.replaceChildren(menu);
    void api.frontendLog(
      `popup rendered label=${payload.label}, item count=${payload.items.length}, width=${payload.width}, height=${payload.height}`
    );
  }

  private async renderSystemStatusPopup(payload: RenderMenuPopupPayload) {
    this.activePrimaryPopup = { x: payload.x, y: payload.y, width: payload.width, height: payload.height };
    let status: SystemStatus;
    try {
      status = await api.getSystemStatus();
    } catch (error) {
      await api.frontendLog(`get_system_status failed: ${serializeLogValue(error)}`);
      status = {
        ssid: null,
        internet_available: false,
        volume: 0,
        audio_available: false
      };
    }

    const menu = document.createElement("div");
    menu.className = "menu system-status-menu";
    menu.setAttribute("role", "menu");

    const networkRow = document.createElement("button");
    networkRow.type = "button";
    networkRow.className = "system-status-network-row";
    networkRow.setAttribute("role", "menuitem");
    networkRow.title = "Open network settings";

    const networkName = document.createElement("span");
    networkName.className = "system-status-network-name";
    networkName.textContent = status.ssid ?? "Not Connected";

    const networkDot = document.createElement("span");
    const connectivityClass = status.ssid
      ? status.internet_available
        ? "connected"
        : "limited"
      : "disconnected";
    networkDot.className = `system-status-dot ${connectivityClass}`;
    networkDot.setAttribute("aria-hidden", "true");
    networkRow.append(networkName, networkDot);
    networkRow.addEventListener("click", () => {
      void (async () => {
        await this.closeMenu();
        await api.openNetworkSettings();
      })().catch(async (error) => {
        await api.frontendLog(`open_network_settings failed: ${serializeLogValue(error)}`);
      });
    });

    const separator = document.createElement("div");
    separator.className = "system-status-separator";

    const volumeRow = document.createElement("div");
    volumeRow.className = "system-status-volume-row";

    const speaker = document.createElement("span");
    speaker.className = "system-status-speaker";
    speaker.setAttribute("aria-hidden", "true");

    const slider = document.createElement("input");
    slider.className = "system-status-volume";
    slider.type = "range";
    slider.min = "0";
    slider.max = "100";
    slider.step = "1";
    slider.value = String(Math.max(0, Math.min(100, status.volume)));
    slider.disabled = !status.audio_available;
    slider.setAttribute("aria-label", "System volume");
    slider.title = status.audio_available ? `Volume: ${slider.value}%` : "System audio unavailable";

    let volumeTimer: number | null = null;
    slider.addEventListener("input", () => {
      slider.title = `Volume: ${slider.value}%`;
      if (volumeTimer !== null) {
        window.clearTimeout(volumeTimer);
      }
      volumeTimer = window.setTimeout(() => {
        volumeTimer = null;
        void api.setSystemVolume(Number(slider.value)).catch(async (error) => {
          await api.frontendLog(`set_system_volume failed: ${serializeLogValue(error)}`);
        });
      }, 45);
    });

    volumeRow.append(speaker, slider);
    menu.append(networkRow, separator, volumeRow);
    this.root.replaceChildren(menu);
    void api.frontendLog(
      `system status popup rendered ssid=${status.ssid ?? "none"}, internet=${status.internet_available}, volume=${status.volume}, audio=${status.audio_available}`
    );
  }

  private renderFlyoutMenu(payload: RenderMenuPopupPayload) {
    const menu = buildMenu(payload.items, { maxHeight: payload.height, handlers: this.menuHandlers() });
    menu.addEventListener("pointerenter", () => {
      this.cancelPopupBlurClose();
      void api.menuFlyoutPointerEntered().catch(console.error);
    });
    this.root.replaceChildren(menu);
    void api.frontendLog(
      `flyout rendered label=${payload.label}, item count=${payload.items.length}, width=${payload.width}, height=${payload.height}`
    );
  }

  private menuHandlers() {
    return {
      onItemAction: async (label: string, action: MenuAction) => {
        if (this.popupMode === "menu" || this.popupMode === "flyout") {
          await api.selectMenuAction(label, action);
          return;
        }

        await this.closeMenu();
        await this.runMenuAction(action);
      },
      onPlainItemPointerEnter: async () => {
        if (this.popupMode === "menu") {
          await this.closeFlyout();
        }
      },
      onSubmenuOpen: async (row: HTMLElement, item: Extract<MenuItem, { kind: "submenu" }>) => {
        this.cancelPopupBlurClose();
        if (!this.activePrimaryPopup) {
          return;
        }
        this.setActiveFlyoutRow(row);
        if (this.activeFlyoutSubmenu === item.submenu && this.flyoutOpen) {
          return;
        }
        if (this.activeFlyoutSubmenu !== null && this.activeFlyoutSubmenu !== item.submenu) {
          void api.frontendLog(`flyout submenu switched: ${this.activeFlyoutSubmenu} -> ${item.submenu}`);
        }
        this.activeFlyoutSubmenu = item.submenu;
        await this.openFlyout(row, item, this.activePrimaryPopup);
      }
    };
  }

  private async openFlyout(
    row: HTMLElement,
    item: Extract<MenuItem, { kind: "submenu" }>,
    primaryPopup: PopupGeometry
  ) {
    const menu = measureMenu(item.items, { maxHeight: this.config.applications.max_menu_height });
    const rowRect = row.getBoundingClientRect();
    const rightX = primaryPopup.x + primaryPopup.width - 1;
    const screenLeft = (window.screen as Screen & { availLeft?: number }).availLeft ?? 0;
    const screenRight = screenLeft + window.screen.availWidth;
    const x = rightX + menu.width <= screenRight ? rightX : primaryPopup.x - menu.width + 1;
    const y = primaryPopup.y + Math.floor(rowRect.top);
    this.flyoutOpen = true;
    await api.frontendLog(
      `flyout open request label=${item.label}, x=${x}, y=${y}, width=${menu.width}, height=${menu.height}, item count=${item.items.length}`
    );
    await api.openMenuFlyout({
      label: item.label,
      submenu: item.submenu,
      x,
      y,
      width: menu.width,
      height: menu.height,
      items: item.items
    });
  }

  private async closeFlyout() {
    if (!this.flyoutOpen && this.activeFlyoutSubmenu === null) {
      return;
    }
    this.flyoutOpen = false;
    this.activeFlyoutSubmenu = null;
    this.clearActiveFlyoutRow();
    await api.closeMenuFlyout();
  }

  private setActiveFlyoutRow(row: HTMLElement) {
    this.activeFlyoutRow?.classList.remove("submenu-open");
    this.activeFlyoutRow = row;
    this.activeFlyoutRow.classList.add("submenu-open");
  }

  private clearActiveFlyoutRow() {
    this.activeFlyoutRow?.classList.remove("submenu-open");
    this.activeFlyoutRow = null;
  }

  private cancelPopupBlurClose() {
    if (this.popupBlurCloseTimer === null) {
      return;
    }
    window.clearTimeout(this.popupBlurCloseTimer);
    this.popupBlurCloseTimer = null;
  }

  private installEventPathDiagnostics() {
    if (!DEBUG_EVENT_DIAGNOSTICS) {
      return;
    }

    for (const eventType of ["pointerdown", "mousedown", "mouseup", "click"]) {
      document.addEventListener(
        eventType,
        (event) => {
          if (!(event instanceof MouseEvent)) {
            return;
          }
          void api.frontendLog(`document event: ${this.formatMouseEvent(event)}`);
        },
        true
      );
    }
  }

  private installButtonEventDiagnostics(button: HTMLElement, label: string) {
    if (!DEBUG_EVENT_DIAGNOSTICS) {
      return;
    }

    for (const eventType of ["pointerdown", "mousedown", "mouseup", "click"]) {
      button.addEventListener(eventType, () => {
        void api.frontendLog(`button event: ${label} ${eventType}`);
      });
    }
  }

  private formatMouseEvent(event: MouseEvent) {
    const target = event.target;
    const element = target instanceof Element ? target : null;
    const tagName = element?.tagName ?? "unknown";
    const className = element ? String(element.className) : "";
    const text = element?.textContent?.trim().replace(/\s+/g, " ").slice(0, 40) ?? "";
    return `${event.type} target=${tagName} class=${className} text="${text}" x=${event.clientX} y=${event.clientY}`;
  }

  private handleGlobalPointerDown(event: PointerEvent) {
    const target = event.target;
    if (!(target instanceof Node)) {
      return;
    }
    if (this.popupMode === "main" && this.openButton && !this.openButton.contains(target) && !this.isPanelMenuButton(target)) {
      void api.frontendLog("outside-click close: main window pointerdown outside active menu button");
      void this.closeMenu({ pointerEvent: event }).catch(console.error);
    }
    if ((this.popupMode === "menu" || this.popupMode === "flyout") && target instanceof Element && !target.closest(".menu")) {
      void this.closeMenu({ pointerEvent: event }).catch(console.error);
    }
  }

  private isPanelMenuButton(target: Node) {
    return target instanceof Element && target.closest(".apple-button, .menu-title, .system-status-button");
  }

  private handleGlobalKeyDown(event: KeyboardEvent) {
    if (event.key === "Escape" && (this.popupMode === "menu" || this.popupMode === "flyout" || this.openButton)) {
      event.preventDefault();
      void this.closeMenu().catch(console.error);
    }
  }

  private async closeMenu(options: { pointerEvent?: PointerEvent } = {}) {
    await api.frontendLog("closeMenu start");
    document.querySelectorAll<HTMLElement>("body > .menu").forEach((menu) => menu.remove());
    this.flyoutOpen = false;
    this.activeFlyoutSubmenu = null;
    this.clearActiveFlyoutRow();
    this.clearOpenMenuState();
    this.clearInteractionState(options.pointerEvent);
    await api.closeMenuPopup();
    await api.frontendLog("closeMenu end");
  }

  private clearOpenMenuState() {
    this.openButton?.classList.remove("is-open");
    this.openButton = null;
  }

  private clearInteractionState(pointerEvent?: PointerEvent) {
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

  private async runMenuAction(action: MenuAction) {
    try {
      const result = await api.runMenuAction(action);
      if (!result.success) {
        const kind = result.error_kind ?? "unknown";
        const message = result.message ?? "panel action failed";
        await api.frontendLog(`menu action failed: ${kind}: ${message}`);
      }
    } catch (error) {
      await api.frontendLog(`menu action failed: ${serializeLogValue(error)}`);
    }
  }
}
