use crate::launcher::{command_exists, spawn_detached_shell};

const SHELL_WINDOW_PATTERN: &str =
    "PiForma Panel|PiForma Menu|Control Strip|At Ease|At-Ease|Clippy|PiForma Window Overview";

pub fn show_all_windows() -> Result<(), String> {
    require_x11_tools(&["wmctrl"])?;
    if !command_exists("zenity") && !command_exists("yad") {
        return Err("Show All Windows requires zenity or yad".to_string());
    }

    spawn_detached_shell(&format!(
        r#"
set -eu
list="${{TMPDIR:-/tmp}}/piforma-window-overview-$$.tsv"
trap 'rm -f "$list"' EXIT

wmctrl -lx | while read -r id desktop class host title; do
  [ -n "${{id:-}}" ] || continue
  props="$(xprop -id "$id" _NET_WM_WINDOW_TYPE 2>/dev/null || true)"
  case "$props" in
    *DOCK*|*DESKTOP*|*SPLASH*|*TOOLTIP*|*NOTIFICATION*) continue ;;
  esac
  identity="$class $title"
  printf '%s' "$identity" | grep -Eiq '{shell_pattern}' && continue
  [ -n "${{title:-}}" ] || title="(Untitled Window)"
  printf '%s\t%s\t%s\n' "$id" "$title" "$class" >> "$list"
done

[ -s "$list" ] || {{
  if command -v zenity >/dev/null 2>&1; then
    zenity --info --title='Show All Windows' --text='No application windows are open.'
  else
    yad --info --title='Show All Windows' --text='No application windows are open.'
  fi
  exit 0
}}

(
  i=0
  while [ "$i" -lt 30 ]; do
    wmctrl -r 'PiForma Window Overview' -b add,above,sticky 2>/dev/null && exit 0
    i=$((i + 1))
    sleep 0.05
  done
) &

if command -v zenity >/dev/null 2>&1; then
  selected="$(zenity --list --title='PiForma Window Overview' --text='Choose a window:' \
    --column='Window ID' --column='Window' --column='Application' \
    --hide-column=1 --print-column=1 --width=640 --height=420 < "$list" || true)"
else
  selected="$(yad --list --title='PiForma Window Overview' --text='Choose a window:' \
    --column='Window ID' --column='Window' --column='Application' \
    --hide-column=1 --print-column=1 --width=640 --height=420 < "$list" || true)"
fi

[ -n "$selected" ] || exit 0
wmctrl -i -r "$selected" -b remove,hidden 2>/dev/null || true
wmctrl -i -a "$selected" 2>/dev/null || {{
  xdotool windowmap "$selected" 2>/dev/null || true
  xdotool windowactivate --sync "$selected"
}}
"#,
        shell_pattern = SHELL_WINDOW_PATTERN
    ))
}

pub fn toggle_show_desktop() -> Result<(), String> {
    require_x11_tools(&["wmctrl", "xdotool"])?;

    spawn_detached_shell(&format!(
        r#"
set -eu
state="${{XDG_RUNTIME_DIR:-/tmp}}/piforma-show-desktop-${{UID:-$(id -u)}}.windows"

if [ -s "$state" ]; then
  last=""
  while IFS= read -r id; do
    [ -n "$id" ] || continue
    wmctrl -i -r "$id" -b remove,hidden 2>/dev/null || xdotool windowmap "$id" 2>/dev/null || true
    last="$id"
  done < "$state"
  rm -f "$state"
  [ -n "$last" ] && wmctrl -i -a "$last" 2>/dev/null || true
  exit 0
fi

: > "$state"
wmctrl -lx | while read -r id desktop class host title; do
  [ -n "${{id:-}}" ] || continue
  props="$(xprop -id "$id" _NET_WM_WINDOW_TYPE 2>/dev/null || true)"
  case "$props" in
    *DOCK*|*DESKTOP*|*SPLASH*|*TOOLTIP*|*NOTIFICATION*) continue ;;
  esac
  identity="$class $title"
  printf '%s' "$identity" | grep -Eiq '{shell_pattern}' && continue
  printf '%s\n' "$id" >> "$state"
done

if [ ! -s "$state" ]; then
  rm -f "$state"
  exit 0
fi

while IFS= read -r id; do
  xdotool windowminimize "$id" 2>/dev/null || wmctrl -i -r "$id" -b add,hidden 2>/dev/null || true
done < "$state"
"#,
        shell_pattern = SHELL_WINDOW_PATTERN
    ))
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
    fn shell_pattern_covers_core_piforma_components() {
        for name in ["PiForma Panel", "Control Strip", "At Ease", "Clippy"] {
            assert!(SHELL_WINDOW_PATTERN.contains(name));
        }
    }
}
