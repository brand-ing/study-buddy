# focus

A minimal study timer for Windows. Pomodoro timer, daily to-do list, and activity heatmap in a small always-on-top window. Themes shift automatically with the time of day.

## Installing

1. Download **focus-setup-0.1.0.exe** from the [Releases](../../releases) page
2. Double-click it and follow the wizard (no admin rights required)
3. Launch **focus** from the Start Menu

To uninstall, go to **Settings → Apps** (or **Add/Remove Programs**) and remove **focus**.

## Building from source

You will need [Rust](https://rustup.rs) installed.

```
cargo build --release
```

The binary is at `target\release\focus.exe`. Run it directly or build the installer:

```
cargo build --release
iscc focus.iss
```

Requires [Inno Setup 6](https://jrsoftware.org/isinfo.php) for the second step. The installer is output to `installer\output\focus-setup-0.1.0.exe`.
