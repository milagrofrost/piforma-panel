import { BUILT_IN_APPLE_LOGO_DATA_URL, shouldUseBuiltInAppleLogo } from "./appleLogo";
import { startClock } from "./clock";
import * as api from "./panelApi";
import { detectPopupMode } from "./panelModel";
import { PopupController } from "./popupController";
import "./styles.css";
import "./macOs9Theme.css";
import "./menuShadowFix.css";
import "./systemStatusMenu.css";
import "./systemStatusAssets";
import "./menuAutoClose";

const app = document.querySelector<HTMLDivElement>("#app");

if (!app) {
  throw new Error("missing #app");
}

const appRoot = app;

void init().catch((error) => {
  console.error("PiForma Panel frontend initialization failed", error);
  document.title = "PiForma Panel initialization failed";
  document.documentElement.dataset.debug = "frontend-init-failed";
});

async function init() {
  const popupMode = detectPopupMode(window.location.search);
  const panelState = await api.getPanelState();
  const { config, geometry } = panelState;
  api.setVerboseDiagnostics(config.diagnostics.verbose);
  const frontendLogAvailable = await api.frontendStatus("frontend init start");
  if (!frontendLogAvailable) {
    console.error("frontend init start failed to reach Rust frontend_log");
    document.title = "PiForma Panel frontend_log failed";
    document.documentElement.dataset.debug = "frontend-log-failed";
  }
  if (popupMode === "main") {
    document.documentElement.style.setProperty("--bar-width", `${geometry.width}px`);
    document.documentElement.style.setProperty("--bar-height", `${geometry.height}px`);
  }
  document.documentElement.style.setProperty("--radius-tl", `${config.bar.radius_top_left}px`);
  document.documentElement.style.setProperty("--radius-tr", `${config.bar.radius_top_right}px`);
  document.documentElement.style.setProperty("--panel-font", config.bar.font_family);
  document.documentElement.style.setProperty("--panel-font-size", `${config.bar.font_size}px`);
  document.documentElement.style.setProperty("--menu-max-height", `${config.applications.max_menu_height}px`);

  const popupController = new PopupController(appRoot, config, geometry, popupMode);
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
  const customLogo = shouldUseBuiltInAppleLogo(config.apple.logo_path)
    ? null
    : await api.getAppleLogoDataUrl();
  const logo = customLogo ?? BUILT_IN_APPLE_LOGO_DATA_URL;
  const [applications, controlPanels] = await Promise.all([api.listApplications(), api.listControlPanels()]);

  await popupController.initializeMainPanel(logo, applications, controlPanels);
  startClock(config);
}
