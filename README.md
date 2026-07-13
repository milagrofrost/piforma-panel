# PiForma Panel

PiForma Panel is a Tauri 2 desktop panel that recreates a classic Macintosh-style menu bar for PiForma Linux desktops. It renders a compact transparent window at the top of the screen, loads menu behavior from the local system, and exposes common desktop actions from Apple, File, Edit, View, and Special menus.

## Features

- Fixed-position 656 x 20 pixel menu bar with classic monochrome menu styling.
- Optional Apple logo loaded from a local image path.
- Live clock with configurable `strftime` formatting.
- Application and Control Panel submenus generated from `.desktop` launchers.
- Desktop helpers for opening folders, launching a terminal, sending edit shortcuts, showing the desktop, refreshing, sleeping the display, restarting, and shutting down.
- User-editable YAML configuration created on first launch.

## Requirements

- Node.js 20 or newer.
- npm.
- Rust stable with Cargo.
- Linux desktop dependencies required by Tauri 2, GTK, WebKitGTK, and AppIndicator support.

On Debian or Raspberry Pi OS style systems, the native dependencies are typically installed with:

```sh
sudo apt install build-essential curl libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev
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

Build the Debian package configured in `src-tauri/tauri.conf.json`:

```sh
npm run tauri -- build
```

The generated package is written under `src-tauri/target/release/bundle/deb/`.

### Packaged frontend diagnostics

Release builds are configured to bundle the Vite output from `../dist` through
`build.frontendDist` in `src-tauri/tauri.conf.json`. This debug branch also
sets `app.security.csp` to `null` so CSP should not block the bundled frontend.

Before packaging, confirm the built frontend contains both the static HTML
marker and the JavaScript diagnostics:

```sh
grep -R FRONTEND_STATIC_MARKER dist
grep -R "frontend init start" dist
```

After installing a package, confirm the installed binary was built from this
frontend/config state:

```sh
strings /usr/bin/piforma-panel | grep FRONTEND
```

At runtime, the app should print `frontend top-level loaded` as soon as the
frontend module starts and `frontend init start` when initialization reaches
Rust. If the static `FRONTEND_STATIC_MARKER` remains visible in the panel, the
HTML loaded but the JavaScript bundle did not execute.

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
- `src-tauri/src/panel_window.rs` applies native geometry for the main panel window.
- `src-tauri/src/popup_windows.rs` creates, sizes, and hides menu popup windows.
- `src-tauri/src/system_actions.rs` owns desktop/system menu actions and confirmations.
- `src-tauri/tauri.conf.json` defines window geometry and Debian bundle settings.
- `src-tauri/capabilities/default.json` defines the default Tauri permissions.
- `docs/shell-window-identity.md` documents the shared PiForma shell-window identity contract.

## Version Control

Commit source files, lockfiles, Tauri configuration, capabilities, and icons. Do not commit generated output such as `node_modules/`, `dist/`, or `src-tauri/target/`.
