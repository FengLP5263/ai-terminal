# Settings, windows and floating bar

[简体中文](settings.md) | English

These commands require manager 0.14.0-preview.2 or newer. Run `instance capabilities` and check `capabilities.native_controls=true`. A newer CLI returns unsupported_command for older managers; failed settings changes are no reason to recreate sessions.

```powershell
ai-terminal-cli settings show
ai-terminal-cli settings fonts
ai-terminal-cli floating set --enabled true --transparency 50 --border true
ai-terminal-cli floating set --scale 142
ai-terminal-cli floating center
ai-terminal-cli sidebar hide
ai-terminal-cli sidebar show
ai-terminal-cli sidebar set --width 220
ai-terminal-cli settings set --language en --font jetbrains_mono --scale 114
```

Percentages are human percentages: transparency 50 is half-transparent; 0 is opaque, 100 makes only the background fully transparent. Avatars remain opaque. Main and floating zoom are independent, each 50–250%. Sidebar width is 190–420 egui points. `floating show/hide/status` enables, disables or queries the bar. `floating set --x X --y Y` uses physical screen pixels; supply both coordinates, which are constrained to available screens.

`settings set` also accepts `--sidebar-collapsed true|false`, `--sidebar-width`, `--floating-enabled`, `--floating-border`, `--floating-scale`, and `--floating-transparency`. Specify boolean values explicitly; --force is not a boolean value. Language IDs: system, zh_cn, en. Font IDs come from fonts; unavailable system fonts fail explicitly.

Use `settings set --file patch.json` for a UTF-8/UTF-8 BOM patch up to 16 KiB. Command-line fields override matching file fields. Only supplied fields change; unknown fields fail. Example:

```json
{"floating_enabled":true,"floating_transparency_percent":50,"floating_border":true,"ui_scale_percent":114,"terminal_font":"jetbrains_mono","language":"en"}
```

Other fields: sidebar_collapsed, sidebar_width, floating_scale_percent, floating_position_pixels (two-element array), floating_center. The last two are mutually exclusive. The complete patch validates and saves before immediate application, preserving sessions/processes. `settings reset` restores all UI defaults, only when requested; it neither deletes sessions nor recreates desktop shortcuts.

```powershell
ai-terminal-cli window show
ai-terminal-cli window focus
ai-terminal-cli window minimize
ai-terminal-cli window maximize
ai-terminal-cli window restore
ai-terminal-cli window set --width 1100 --height 720
ai-terminal-cli window set --x 100 --y 100
ai-terminal-cli panel open files
ai-terminal-cli panel open history
ai-terminal-cli panel close git
ai-terminal-cli panel open settings
```

Window dimensions/positions use egui points; dimensions and coordinates each require a pair. Commands return requested; read window show for the actual OS result. Panels: files, git, history, changes, settings. Files and Git sidebars are mutually exclusive; closing a sidebar does not close the central document. Commands also execute while minimized, without mouse input.

`window capture --output <absolute-path.png>` captures only this manager's main window, returning pending. Query `window capture-status` until complete/failed. The parent directory must exist; existing files are never overwritten. Minimized windows may time out; restore and retry. A capture is an explicitly requested artifact and can contain visible terminal text; do not capture user workspaces by default. `app quit --force` closes the target manager and its owned processes; never close an unrequested instance.
