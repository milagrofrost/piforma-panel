# PiForma Shell Window Identity

This contract defines stable identifiers for PiForma shell-owned windows. It is
intended for PiForma Panel, Control Strip, At Ease, Clippy, and future window
management helpers that need to distinguish shell UI from normal applications.

## Common Properties

Prefer stable properties in this order:

1. `_PIFORMA_ROLE` if a component implements it in the future.
2. Standard `WM_WINDOW_ROLE` / GTK window role.
3. Application ID or WM class/instance.
4. Stable Tauri window label for Tauri components.
5. Owning PID and executable name.
6. Window title as a last-resort fallback only.

PiForma Panel currently sets stable Tauri labels, stable titles, and GTK window
roles via safe GTK APIs. It does not set `_PIFORMA_ROLE`: doing that reliably
would require direct X11 atom/property handling that should be introduced only
with a small, tested X11 helper.

## Roles

| Role | Component | Stable role ID | Tauri label | Title | App ID / WM class |
| --- | --- | --- | --- | --- | --- |
| Main panel | PiForma Panel | `piforma-panel.main-panel` | `main` | `PiForma Panel` | `org.piforma.panel` |
| Menu popup | PiForma Panel | `piforma-panel.menu-popup` | `menu-popup` | `PiForma Menu` | `org.piforma.panel` |
| Flyout popup | PiForma Panel | `piforma-panel.menu-flyout` | `menu-flyout` | `PiForma Menu Flyout` | `org.piforma.panel` |
| Control Strip | Control Strip | `piforma-control-strip.main` | component-defined | `Control Strip` | component-defined |
| At Ease | At Ease | `piforma-at-ease.main` | component-defined | `At Ease` | component-defined |
| Clippy | Clippy | `piforma-clippy.main` | component-defined | `Clippy` | component-defined |
| App-owned dialog | Any PiForma app | `piforma.<app>.dialog` | component-defined | dialog-specific | owning app |
| Window overview | Future shell | `piforma-shell.window-overview` | component-defined | `PiForma Window Overview` | component-defined |
| Normal application | Non-shell apps | none | none | app-defined | app-defined |

## Matching Guidance

Shell-aware code should treat PiForma shell roles as reserved UI and avoid
including them in normal application lists, focus restoration targets, Show
Desktop actions, or future Show All Windows views.

Title matching exists only for compatibility with older builds and window
managers that do not expose role/class data. Do not make title the primary
identifier for new PiForma components.
