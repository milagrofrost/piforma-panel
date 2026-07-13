use crate::{
    config::config_path,
    launcher::{command_exists, spawn_detached_shell},
};
use serde::Deserialize;
use std::fs;

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct ShowDesktopSettings {
    show_desktop_apps: Vec<String>,
}

pub fn show_all_windows() -> Result<(), String> {
    require_x11_tools(&["wmctrl", "xdotool"])?;

    spawn_detached_shell(
        r#"
set -eu
state="${XDG_RUNTIME_DIR:-/tmp}/piforma-show-desktop-${UID:-$(id -u)}.windows"

wmctrl -lx | while read -r id desktop class host title; do
  [ -n "${id:-}" ] || continue
  props="$(xprop -id "$id" _NET_WM_WINDOW_TYPE 2>/dev/null || true)"
  case "$props" in
    *DOCK*|*DESKTOP*|*SPLASH*|*TOOLTIP*|*NOTIFICATION*) continue ;;
  esac

  wmctrl -i -r "$id" -b remove,hidden 2>/dev/null || true
  xdotool windowmap "$id" 2>/dev/null || true
  xdotool windowraise "$id" 2>/dev/null || true
done

rm -f "$state"
"#,
    )
}

pub fn show_desktop() -> Result<(), String> {
    require_x11_tools(&["wmctrl", "xdotool"])?;
    let preserved_apps = load_show_desktop_apps()?;
    let preserve_setup = preserved_apps
        .iter()
        .filter(|name| !name.trim().is_empty())
        .map(|name| format!("printf '%s\\n' {} >> \"$preserve\"", shell_quote(name.trim())))
        .collect::<Vec<_>>()
        .join("\n");

    spawn_detached_shell(&format!(
        r#"
set -eu
state="${{XDG_RUNTIME_DIR:-/tmp}}/piforma-show-desktop-${{UID:-$(id -u)}}.windows"
preserve="${{XDG_RUNTIME_DIR:-/tmp}}/piforma-show-desktop-${{UID:-$(id -u)}}.preserve"
rm -f "$preserve"
: > "$preserve"
{preserve_setup}

: > "$state"
wmctrl -lx | while read -r id desktop class host title; do
  [ -n "${{id:-}}" ] || continue
  props="$(xprop -id "$id" _NET_WM_WINDOW_TYPE 2>/dev/null || true)"
  case "$props" in
    *DOCK*|*DESKTOP*|*SPLASH*|*TOOLTIP*|*NOTIFICATION*) continue ;;
  esac
  identity="$class $title"
  if [ -s "$preserve" ] && printf '%s\n' "$identity" | grep -Fqi -f "$preserve"; then
    continue
  fi
  printf '%s\n' "$id" >> "$state"
done

rm -f "$preserve"
if [ ! -s "$state" ]; then
  rm -f "$state"
  exit 0
fi

while IFS= read -r id; do
  xdotool windowminimize "$id" 2>/dev/null || wmctrl -i -r "$id" -b add,hidden 2>/dev/null || true
done < "$state"
"#,
    ))
}

fn load_show_desktop_apps() -> Result<Vec<String>, String> {
    let path = config_path()?;
    let contents = fs::read_to_string(&path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    let settings: ShowDesktopSettings = serde_yaml::from_str(&contents)
        .map_err(|err| format!("invalid config.yaml: {err}"))?;
    Ok(settings.show_desktop_apps)
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn require_x11_tools(tools: &[&str]) -> Result<(), String> {
    let missing = tools
        .iter()
        .copied()
        .filter(|tool| !command_exists(tool))
        .collect::<Vec<_>>();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(format!("window management requires: {}", missing.join(", ")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_show_desktop_config_preserves_nothing() {
        let settings: ShowDesktopSettings = serde_yaml::from_str("bar: {}\n").unwrap();
        assert!(settings.show_desktop_apps.is_empty());
    }

    #[test]
    fn show_desktop_apps_are_loaded_from_top_level_list() {
        let settings: ShowDesktopSettings = serde_yaml::from_str(
            "show_desktop_apps:\n  - PiForma Panel\n  - Control Strip\n  - At Ease\n  - Clippy\n",
        )
        .unwrap();
        assert_eq!(settings.show_desktop_apps.len(), 4);
    }
}
