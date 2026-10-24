# focus

A minimal study timer for Windows. Pomodoro timer, daily to-do list, and activity heatmap in a compact always-on-top window. Themes shift automatically with the time of day.

## Install

1. Go to the [**Releases**](../../releases/latest) page
2. Download **focus.exe**
3. Run it — no installer, no admin rights needed

The app lives in your system tray. To keep it around permanently, move `focus.exe` to a folder like `%LOCALAPPDATA%\focus\` and enable **Launch at login** in Settings (⚙).

**To uninstall:** close the app, delete the file, and optionally remove app data at `%LOCALAPPDATA%\study-buddy\`.

## Features

- **Pomodoro timer** — Classic (25/5/15), Deep Work (50/10), Balanced (45/15), or custom
- **To-do list** — add, check off, reorder by drag, double-click to edit
- **Task dock** — a taller scrollable panel that pops up from the tray (`Shift+2` to open, `Shift+1` for the main window)
- **Activity heatmap** — 16-week GitHub-style graph, one square per day
- **Six time-of-day themes** — Sunrise · Morning · Afternoon · Evening · Night · Late Night
- **System tray** — live countdown tooltip, pause/resume, show tasks without opening the main window
- **Auto-update** — checks GitHub Releases, downloads and stages the new binary silently; installs on next close
- **Global hotkey** — `Ctrl+Shift+F` shows or hides the window from anywhere

## Keyboard shortcuts

| Key | Action |
|---|---|
| `Space` | Start / pause timer |
| `R` | Reset timer |
| `S` | Skip phase |
| `← →` | Switch tabs |
| `Shift+1` | Main window |
| `Shift+2` | Task dock |
| `Ctrl+Shift+F` | Show / hide (global) |
| `?` | Shortcuts panel |
| `Esc` | Close panel |

## Building from source

Requires [Rust](https://rustup.rs).

```
cargo build --release
```

The binary is at `target\release\focus.exe`.
