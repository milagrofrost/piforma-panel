import { describe, expect, it } from "vitest";

import { detectPopupMode, serializeLogValue } from "./panelModel";

describe("panel model helpers", () => {
  it("detects supported popup modes from the query string", () => {
    expect(detectPopupMode("?popup=menu")).toBe("menu");
    expect(detectPopupMode("?popup=flyout")).toBe("flyout");
    expect(detectPopupMode("?popup=unknown")).toBe("main");
    expect(detectPopupMode("")).toBe("main");
  });

  it("serializes errors and arbitrary values for diagnostics", () => {
    expect(serializeLogValue(new Error("broken"))).toBe("Error: broken");
    expect(serializeLogValue({ ok: true })).toBe("{\"ok\":true}");
  });
});
