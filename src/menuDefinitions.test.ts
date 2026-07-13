import { describe, expect, it } from "vitest";

import { appleMenu, editMenu, specialMenu } from "./menuDefinitions";
import type { DesktopApp } from "./panelModel";

function app(overrides: Partial<DesktopApp>): DesktopApp {
  return {
    id: "test.desktop",
    name: "Test App",
    exec: "test-app %U",
    categories: [],
    group: "Other",
    is_control_panel: false,
    ...overrides
  };
}

describe("menu definitions", () => {
  it("creates disabled placeholder items for empty application submenus", () => {
    const menu = appleMenu([], []);
    const applications = menu.find((item) => item.kind === "submenu" && item.submenu === "applications");

    expect(applications).toMatchObject({
      kind: "submenu",
      items: [
        {
          kind: "item",
          label: "(No Applications)",
          enabled: false,
          action: { kind: "placeholder", message: "(No Applications)" }
        }
      ]
    });
  });

  it("sorts application items by display name", () => {
    const menu = appleMenu([app({ name: "Zebra" }), app({ name: "Calculator" })], []);
    const applications = menu.find((item) => item.kind === "submenu" && item.submenu === "applications");

    expect(applications?.kind).toBe("submenu");
    if (applications?.kind !== "submenu") {
      return;
    }
    expect(applications.items.map((item) => (item.kind === "item" ? item.label : ""))).toEqual(["Calculator", "Zebra"]);
  });

  it("uses typed shortcut and system action identifiers", () => {
    expect(editMenu()).toContainEqual({ kind: "item", label: "Copy", action: { kind: "send_shortcut", action: "copy" } });
    expect(specialMenu()).toContainEqual({
      kind: "item",
      label: "Sleep Display",
      action: { kind: "run_system_action", action: "sleep_display", confirmed: false }
    });
  });
});
