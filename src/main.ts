import { startClock } from "./clock";
import * as api from "./panelApi";
import { detectPopupMode } from "./panelModel";
import { PopupController } from "./popupController";
import "./styles.css";

const STATIC_FRONTEND_MARKER_ID = "static-frontend-marker";

const app = document.querySelector<HTMLDivElement>("#app");

if (!app) {
  throw new Error("missing #app");
}

const appRoot = app;

markFrontendLoaded();
void init();

function markFrontendLoaded() {
  const marker = document.getElementById(STATIC_FRONTEND_MARKER_ID);
  if (marker) {
    marker.textContent = "FRONTEND_JS_STARTED";
    marker.remove();
  }
  void api.frontendLog("frontend top-level loaded");
}

async function init() {
  const popupMode = detectPopupMode(window.location.search);
  const config = await api.getConfig();
  const frontendLogAvailable = await api.frontendLog("frontend init start");
  if (!frontendLogAvailable) {
    console.error("frontend init start failed to reach Rust frontend_log");
    document.title = "PiForma Panel frontend_log failed";
    document.documentElement.dataset.debug = "frontend-log-failed";
  }
  if (popupMode === "main") {
    document.documentElement.style.setProperty("--bar-width", `${config.bar.width}px`);
    document.documentElement.style.setProperty("--bar-height", `${config.bar.height}px`);
  }
  document.documentElement.style.setProperty("--radius-tl", `${config.bar.radius_top_left}px`);
  document.documentElement.style.setProperty("--radius-tr", `${config.bar.radius_top_right}px`);
  document.documentElement.style.setProperty("--panel-font", config.bar.font_family);
  document.documentElement.style.setProperty("--panel-font-size", `${config.bar.font_size}px`);
  document.documentElement.style.setProperty("--menu-max-height", `${config.applications.max_menu_height}px`);

  const popupController = new PopupController(appRoot, config, popupMode);
  popupController.installGlobalMenuListeners();

  if (popupMode === "menu") {
    void api.frontendLog("primary popup mode init");
    document.body.classList.add("popup-window");
    await popupController.initializePrimaryPopupWindow();
    return;
  }

  if (popupMode === "flyout") {
    void api.frontendLog("flyout popup mode init");
    document.body.classList.add("popup-window", "flyout-window");
    await popupController.initializeFlyoutWindow();
    return;
  }

  await api.initializeMainWindow();
  const logo = await api.getAppleLogoDataUrl();
  const [applications, controlPanels] = await Promise.all([api.listApplications(), api.listControlPanels()]);

  await popupController.initializeMainPanel(logo, applications, controlPanels);
  startClock(config);
}
