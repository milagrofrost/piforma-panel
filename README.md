# PiForma Panel

PiForma Panel is a Tauri 2 desktop panel that recreates a classic Macintosh-style menu bar for PiForma Linux desktops. It renders a compact transparent window at the top of the screen, loads menu behavior from the local system, and exposes common desktop actions from Apple, File, Edit, View, and Special menus.

## Features

- Fixed-position 656 x 20 pixel menu bar with classic monochrome menu styling.
- Optional Apple logo loaded from a local image path.
- Live clock with configurable `strftime` formatting.
- Application and Control Panel submenus generated from `.desktop` launchers.
- Desktop helpers for opening folders, launching a terminal, sending edit shortcuts, showing the desktop, refreshing, sleeping the display, restarting, and shutting down.
- Publishes an X11 dock window type and top-edge EWMH strut so Openbox keeps normal windows below the panel.
- User-editable YAML configuration created on first launch.

## Requirements

- Node.js 20 or newer.
- npm.
- Rust stable with Cargo.
- Linux desktop dependencies required by Tauri 2, GTK, WebKitGTK, and AppIndicator support.
- X11 with an EWMH-compatible window manager such as Openbox for desktop work-area reservation.
- `xprop` from `x11-utils` for publishing the panel strut after the native window is shown.

On Debian or Raspberry Pi OS style systems, the native dependencies are typically installed with:

```sh
sudo apt install build-essential curl libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev x11-utils
```

## Development

Install JavaScript dependencies:

```sh
npm install
```

Run the frontend only:

```sh
npm run dev
```

Run the Tauri app in development mode:

```sh
npm run tauri -- dev
```

Build the frontend:

```sh
npm run build
```

Run lightweight frontend checks:

```sh
npm run typecheck
npm test
npm run check
```

Run lightweight Rust checks:

```sh
cd src-tauri
cargo fmt --check
cargo test
cargo clippy --all-targets -- -D warnings
```

Build the Debian package configured in `src-tauri/tauri.conf.json`:

```sh
npm run tauri -- build
```

The generated package is written under `src-tauri/target/release/bundle/deb/`.

### Diagnostics

Normal startup prints concise build identity and startup failures. Verbose
frontend, popup, geometry, and packaged-asset diagnostics are disabled by
default. Enable them temporarily with either:

```yaml
diagnostics:
  verbose: true
```

or:

```sh
PIFORMA_PANEL_DEBUG=1 piforma-panel
```

Keep verbose diagnostics disabled for normal packaged sessions.

## Openbox work-area reservation

The panel marks its native GTK window as `_NET_WM_WINDOW_TYPE_DOCK` and publishes
both `_NET_WM_STRUT` and `_NET_WM_STRUT_PARTIAL`. The reserved top depth and
horizontal span are calculated from the effective panel geometry, including the
configured `x`, `y`, width, and height.

Openbox should then keep newly placed and maximized normal windows below the
panel. Fullscreen applications may still cover it, which is intentional. Do not
also configure a fixed top margin in Openbox, because combining a manual margin
with the panel strut can reserve the space twice.

The panel retries `xprop` briefly after showing the native window. If `xprop` is
missing or cannot find the window, the panel continues running and prints a
warning, but Openbox will not reserve the work area.

To inspect the properties while the panel is running:

```sh
xprop -name "PiForma Panel" _NET_WM_WINDOW_TYPE _NET_WM_STRUT _NET_WM_STRUT_PARTIAL
```

## Configuration

On first launch, PiForma Panel creates:

```text
~/.local/share/piforma-panel/config.yaml
```

Default configuration:

```yaml
# PiForma Panel config.
# Missing sections and fields use the defaults shown here.
bar:
  width: 658
  height: 20
  x: 76
  y: 0
  radius_top_left: 18
  radius_top_right: 18
  font_family: ChicagoFLF
  font_size: 13

apple:
  logo_path: ~/.local/share/piforma-panel/apple-color.png

clock:
  enabled: true
  format: "%I:%M %p"

applications:
  scan_dirs:
    - ~/.local/share/applications
    - /usr/local/share/applications
    - /usr/share/applications
  show_no_display: false
  group_by_categories: true
  show_category_labels: false
  max_menu_height: 420

menus:
  show_file: true
  show_edit: true
  show_view: true
  show_special: true

actions:
  clean_up_window_command: ""

diagnostics:
  verbose: false
```

Configuration is backward-compatible with partial files: missing sections and
fields use these defaults. User paths beginning with `~` are expanded at load
time. Unsafe numeric values are normalized before use: panel width and height,
font size, and maximum menu height are clamped to at least `1`; corner radii are
clamped to at least `0`; very large values are capped to protect window and menu
geometry.

After editing the configuration, restart the app to apply window size, position, menu visibility, and launcher scanning changes.

## Source Layout

- `src/main.ts` bootstraps the frontend and composes UI modules.
- `src/panelModel.ts` defines frontend-facing config, menu, popup, and launcher types.
- `src/panelApi.ts` wraps Tauri commands and events.
- `src/menuDefinitions.ts` owns static menu item definitions.
- `src/menuRenderer.ts` renders panel and popup menu DOM.
- `src/popupController.ts` owns popup state, coordination, and menu interactions.
- `src/clock.ts` owns clock formatting and update scheduling.
- `src/styles.css` contains the classic panel and menu styling.
- `src-tauri/src/main.rs` registers Tauri commands and composes backend modules.
- `src-tauri/src/config.rs` owns configuration defaults, loading, validation, and path handling.
- `src-tauri/src/panel_model.rs` defines effective panel geometry and reusable popup/strut coordinate helpers.
- `src-tauri/src/panel_actions.rs` defines typed panel actions and structured action results.
- `src-tauri/src/shell_identity.rs` applies stable PiForma shell window roles and titles.
- `src-tauri/src/desktop_entries.rs` discovers and parses application launchers.
- `src-tauri/src/launcher.rs` owns detached process launching helpers.
- `src-tauri/src/window_manager.rs` tracks and restores the previously active desktop window.
- `src-tauri/src/panel_window.rs` applies native geometry, dock identity, and EWMH work-area reservation for the main panel window.
- `src-tauri/src/popup_windows.rs` creates, sizes, and hides menu popup windows.
- `src-tauri/src/system_actions.rs` owns desktop/system menu actions and confirmations.
- `src-tauri/tauri.conf.json` defines window geometry and Debian bundle settings.
- `src-tauri/capabilities/default.json` defines the default Tauri permissions.
- `docs/shell-window-identity.md` documents the shared PiForma shell-window identity contract.

## Version Control

Commit source files, lockfiles, Tauri configuration, capabilities, and icons. Do not commit generated output such as `node_modules/`, `dist/`, or `src-tauri/target/`.
