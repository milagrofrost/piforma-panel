import { emit, listen, type UnlistenFn } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";

const MENU_AUTO_CLOSE_DELAY_MS = 2000;
const MENU_TERRITORY_SELECTOR = ".menu, .apple-button, .menu-title, .system-status-button";
const POINTER_ENTERED_EVENT = "piforma-menu-pointer-entered";
const POINTER_LEFT_EVENT = "piforma-menu-pointer-left";

let closeTimer: number | null = null;
let unlistenEntered: UnlistenFn | null = null;
let unlistenLeft: UnlistenFn | null = null;

function cancelCloseTimer() {
  if (closeTimer === null) {
    return;
  }
  window.clearTimeout(closeTimer);
  closeTimer = null;
}

function scheduleCloseTimer() {
  cancelCloseTimer();
  closeTimer = window.setTimeout(() => {
    closeTimer = null;
    void invoke("close_menu_popup").catch((error) => {
      console.error("menu auto-close failed", error);
    });
  }, MENU_AUTO_CLOSE_DELAY_MS);
}

function closestMenuTerritory(target: EventTarget | null) {
  return target instanceof Element ? target.closest(MENU_TERRITORY_SELECTOR) : null;
}

function pointerStayedInsideTerritory(event: PointerEvent) {
  const from = closestMenuTerritory(event.target);
  const to = closestMenuTerritory(event.relatedTarget);
  return from !== null && to !== null;
}

async function installCrossWindowListeners() {
  unlistenEntered = await listen(POINTER_ENTERED_EVENT, cancelCloseTimer);
  unlistenLeft = await listen(POINTER_LEFT_EVENT, scheduleCloseTimer);
}

function installLocalPointerListeners() {
  document.addEventListener(
    "pointerover",
    (event) => {
      if (!closestMenuTerritory(event.target)) {
        return;
      }
      cancelCloseTimer();
      void emit(POINTER_ENTERED_EVENT);
    },
    true
  );

  document.addEventListener(
    "pointerout",
    (event) => {
      if (!closestMenuTerritory(event.target) || pointerStayedInsideTerritory(event)) {
        return;
      }
      scheduleCloseTimer();
      void emit(POINTER_LEFT_EVENT);
    },
    true
  );
}

function cleanup() {
  cancelCloseTimer();
  unlistenEntered?.();
  unlistenEntered = null;
  unlistenLeft?.();
  unlistenLeft = null;
}

installLocalPointerListeners();
void installCrossWindowListeners().catch((error) => {
  console.error("failed to install menu auto-close listeners", error);
});
window.addEventListener("beforeunload", cleanup, { once: true });
