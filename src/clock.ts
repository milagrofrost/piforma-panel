import type { PanelConfig } from "./panelModel";

export function startClock(config: PanelConfig) {
  updateClock(config);
  window.setInterval(() => updateClock(config), 1000);
}

function updateClock(config: PanelConfig) {
  const clock = document.querySelector<HTMLDivElement>("#clock");
  if (!clock || !config.clock.enabled) {
    return;
  }

  clock.textContent = formatClockTime(new Date(), config.clock.format);
}

export function formatClockTime(now: Date, format: string) {
  const hour = now.getHours();
  const minute = now.getMinutes().toString().padStart(2, "0");
  const hour12 = hour % 12 || 12;
  const ampm = hour >= 12 ? "PM" : "AM";
  return format
    .replace("%I", hour12.toString().padStart(2, "0"))
    .replace("%M", minute)
    .replace("%p", ampm);
}
