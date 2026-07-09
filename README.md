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

## Configuration

On first launch, PiForma Panel creates:

```text
~/.local/share/piforma-panel/config.yaml
```

Default configuration:

```yaml
bar:
  width: 656
  height: 20
  x: 77
  y: 0
  radius_top_left: 18
  radius_top_right: 18
  font_family: ChicagoFLF
  font_size: 13

apple:
  logo_path: /home/frost/.local/share/piforma-panel/apple-color.png

clock:
  enabled: true
  format: "%I:%M %p"

applications:
  scan_dirs:
    - /home/frost/.local/share/applications
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
```

After editing the configuration, restart the app to apply window size, position, menu visibility, and launcher scanning changes.

## Source Layout

- `src/main.ts` builds the menu bar UI and calls Tauri commands.
- `src/styles.css` contains the classic panel and menu styling.
- `src-tauri/src/main.rs` owns configuration, desktop launcher scanning, and system actions.
- `src-tauri/tauri.conf.json` defines window geometry and Debian bundle settings.
- `src-tauri/capabilities/default.json` defines the default Tauri permissions.

## Version Control

Commit source files, lockfiles, Tauri configuration, capabilities, and icons. Do not commit generated output such as `node_modules/`, `dist/`, or `src-tauri/target/`.
