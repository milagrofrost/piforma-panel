import { describe, expect, it } from "vitest";
import { BUILT_IN_APPLE_LOGO_DATA_URL, shouldUseBuiltInAppleLogo } from "./appleLogo";

describe("Apple logo selection", () => {
  it("uses the bundled logo for the legacy PiForma logo path", () => {
    expect(shouldUseBuiltInAppleLogo("/home/frost/.local/share/piforma-panel/apple-color.png")).toBe(true);
  });

  it("uses the bundled logo when no custom path is configured", () => {
    expect(shouldUseBuiltInAppleLogo("  ")).toBe(true);
  });

  it("preserves a genuinely custom logo path", () => {
    expect(shouldUseBuiltInAppleLogo("/home/frost/Pictures/custom-apple.png")).toBe(false);
  });

  it("embeds a PNG data URL", () => {
    expect(BUILT_IN_APPLE_LOGO_DATA_URL.startsWith("data:image/png;base64,")).toBe(true);
  });
});
