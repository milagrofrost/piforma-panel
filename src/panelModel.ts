export type PanelConfig = {
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
  apple: {
    logo_path: string;
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
  diagnostics: {
    verbose: boolean;
  };
};

export type PanelGeometry = {
  monitor_id?: string | null;
  monitor_origin_x: number;
  monitor_origin_y: number;
  monitor_width?: number | null;
  monitor_height?: number | null;
  x: number;
  y: number;
  width: number;
  height: number;
  scale_factor: number;
  coordinate_space: string;
};

export type PanelState = {
  config: PanelConfig;
  geometry: PanelGeometry;
};

export type DesktopApp = {
  id: string;
  name: string;
  exec: string;
  icon?: string;
  categories: string[];
  group: string;
  is_control_panel: boolean;
};

export type MenuItem =
  | { kind: "item"; label: string; action: MenuAction; enabled?: boolean }
  | { kind: "submenu"; label: string; submenu: SubmenuKind; items: MenuItem[] }
  | { kind: "separator" };

export type MenuAction =
  | { kind: "placeholder"; message: string }
  | { kind: "show_about" }
  | { kind: "launch_app"; exec: string; name: string }
  | { kind: "launch_calculator" }
  | { kind: "open_folder"; folder: "applications" | "home" | "desktop" }
  | { kind: "new_terminal_window" }
  | { kind: "send_shortcut"; action: ShortcutAction }
  | { kind: "run_system_action"; action: SystemActionId; confirmed: boolean }
  | { kind: "confirmed_system_action"; action: "restart" | "shut_down"; message: string };

export type BackendPanelAction = Exclude<MenuAction, { kind: "placeholder" } | { kind: "confirmed_system_action" }>;

export type ShortcutAction = "undo" | "cut" | "copy" | "paste" | "clear" | "select_all";

export type SystemActionId =
  | "sleep_display"
  | "show_desktop"
  | "refresh"
  | "clean_up_window"
  | "restart"
  | "shut_down"
  | "show_clipboard";

export type ActionResult = {
  success: boolean;
  message?: string;
  error_kind?: "unsupported" | "cancelled" | "authorization_failed" | "command_failed" | "target_unavailable" | "invalid_request";
};

export type RenderMenuPopupPayload = {
  label: string;
  submenu?: SubmenuKind;
  items: MenuItem[];
  width: number;
  height: number;
  x: number;
  y: number;
};

export type PopupMode = "main" | "menu" | "flyout";
export type SubmenuKind = "applications" | "control_panels";

export type PopupGeometry = {
  x: number;
  y: number;
  width: number;
  height: number;
};

export function detectPopupMode(search: string): PopupMode {
  const params = new URLSearchParams(search);
  const popupParam = params.get("popup");
  return popupParam === "menu" || popupParam === "flyout" ? popupParam : "main";
}

export function serializeLogValue(value: unknown) {
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
