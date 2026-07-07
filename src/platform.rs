use std::sync::OnceLock;
use std::sync::atomic::{AtomicPtr, Ordering};
use crate::data::Phase;

static SHOW_HOTKEY_ID: OnceLock<u32> = OnceLock::new();
static TRAY_PTR: AtomicPtr<tray_icon::TrayIcon> = AtomicPtr::new(std::ptr::null_mut());
static PAUSE_ITEM_PTR: AtomicPtr<tray_icon::menu::MenuItem> = AtomicPtr::new(std::ptr::null_mut());

// ── Tray ──────────────────────────────────────────────────────────────────

fn make_icon_rgba(phase: Phase, running: bool) -> Vec<u8> {
    const S: u32 = 32;
    let n = S as usize;
    let mut rgba = vec![0u8; n * n * 4];
    let (ring_r, ring_g, ring_b): (u8, u8, u8) = if !running {
        (80, 80, 90)
    } else {
        match phase {
            Phase::Work => (74, 222, 128),
            _ => (96, 165, 250),
        }
    };
    let c = 15.5_f32;
    for y in 0..n {
        for x in 0..n {
            let dx = x as f32 - c;
            let dy = y as f32 - c;
            let r = (dx * dx + dy * dy).sqrt();
            let i = (y * n + x) * 4;
            if r > 14.5 { continue; }
            if r >= 11.0 {
                rgba[i] = ring_r; rgba[i+1] = ring_g; rgba[i+2] = ring_b; rgba[i+3] = 255;
            } else {
                rgba[i] = 15; rgba[i+1] = 17; rgba[i+2] = 23; rgba[i+3] = 255;
                let hand_up    = dx.abs() < 1.2 && dy < 0.0 && dy > -8.5;
                let hand_right = dy.abs() < 1.2 && dx > 0.0 && dx < 6.5;
                if hand_up || hand_right || r < 1.8 {
                    rgba[i] = 200; rgba[i+1] = 225; rgba[i+2] = 245; rgba[i+3] = 255;
                }
            }
        }
    }
    rgba
}

pub fn setup_tray() {
    let rgba = make_icon_rgba(Phase::Work, false);
    let icon = tray_icon::Icon::from_rgba(rgba, 32, 32).expect("tray icon rgba");

    let menu     = tray_icon::menu::Menu::new();
    let quit_i   = tray_icon::menu::MenuItem::with_id("quit",         "Quit",             true, None);
    let update_i = tray_icon::menu::MenuItem::with_id("check_update", "Check for update", true, None);
    let about_i  = tray_icon::menu::MenuItem::with_id("about",        "About focus",      true, None);
    let pause_i  = tray_icon::menu::MenuItem::with_id("pause_resume", "Resume timer",     true, None);
    let sep      = tray_icon::menu::PredefinedMenuItem::separator();
    let show_i   = tray_icon::menu::MenuItem::with_id("show",         "Show focus",       true, None);
    let _ = menu.append_items(&[&quit_i, &update_i, &about_i, &sep, &pause_i, &show_i]);

    let pause_leaked: &'static tray_icon::menu::MenuItem = Box::leak(Box::new(pause_i));
    PAUSE_ITEM_PTR.store(pause_leaked as *const _ as *mut _, Ordering::Relaxed);

    let tray = tray_icon::TrayIconBuilder::new()
        .with_tooltip("focus")
        .with_icon(icon)
        .with_menu(Box::new(menu))
        .build()
        .expect("tray icon build");

    let tray_leaked: &'static tray_icon::TrayIcon = Box::leak(Box::new(tray));
    TRAY_PTR.store(tray_leaked as *const _ as *mut _, Ordering::Relaxed);
}

pub fn update_tray(phase: Phase, running: bool, remaining: u32) {
    let tray_ptr = TRAY_PTR.load(Ordering::Relaxed);
    if tray_ptr.is_null() { return; }
    let tray = unsafe { &*tray_ptr };

    let rgba = make_icon_rgba(phase, running);
    if let Ok(icon) = tray_icon::Icon::from_rgba(rgba, 32, 32) {
        let _ = tray.set_icon(Some(icon));
    }

    let tooltip = if !running {
        "focus · idle".to_string()
    } else if phase == Phase::OpenBreak {
        "focus · open break".to_string()
    } else {
        let label = if phase == Phase::Work { "work" } else { "break" };
        format!("focus · {:02}:{:02} · {}", remaining / 60, remaining % 60, label)
    };
    let _ = tray.set_tooltip(Some(&tooltip));

    let pause_ptr = PAUSE_ITEM_PTR.load(Ordering::Relaxed);
    if !pause_ptr.is_null() {
        let item = unsafe { &*pause_ptr };
        item.set_text(if running { "Pause timer" } else { "Resume timer" });
    }
}

// ── Global hotkey ─────────────────────────────────────────────────────────

pub fn setup_hotkey() {
    use global_hotkey::{GlobalHotKeyManager, hotkey::{HotKey, Modifiers, Code}};
    let manager = GlobalHotKeyManager::new().expect("hotkey manager");
    let hotkey  = HotKey::new(
        Some(Modifiers::CONTROL | Modifiers::SHIFT),
        Code::KeyF,
    );
    SHOW_HOTKEY_ID.set(hotkey.id()).ok();
    manager.register(hotkey).ok();
    Box::leak(Box::new(manager));
}

pub fn show_hotkey_id() -> Option<u32> {
    SHOW_HOTKEY_ID.get().copied()
}

// ── Window visibility ─────────────────────────────────────────────────────

pub fn hide_window() {
    unsafe {
        use windows_sys::Win32::UI::WindowsAndMessaging::{FindWindowW, ShowWindow, SW_HIDE};
        let title: Vec<u16> = "focus\0".encode_utf16().collect();
        let hwnd = FindWindowW(std::ptr::null(), title.as_ptr());
        if hwnd != 0 { ShowWindow(hwnd, SW_HIDE); }
    }
}

pub fn show_window() {
    unsafe {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            FindWindowW, ShowWindow, SetForegroundWindow, SW_RESTORE,
        };
        let title: Vec<u16> = "focus\0".encode_utf16().collect();
        let hwnd = FindWindowW(std::ptr::null(), title.as_ptr());
        if hwnd != 0 {
            ShowWindow(hwnd, SW_RESTORE);
            SetForegroundWindow(hwnd);
        }
    }
}

// ── Notifications ─────────────────────────────────────────────────────────

fn do_toast(title: &str, body: &str, long: bool) {
    let dur = if long { winrt_notification::Duration::Long } else { winrt_notification::Duration::Short };
    let _ = winrt_notification::Toast::new(winrt_notification::Toast::POWERSHELL_APP_ID)
        .title(title)
        .text1(body)
        .duration(dur)
        .show();
}

pub fn open_url(url: &str) {
    let url = url.to_owned();
    std::thread::spawn(move || {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        let _ = std::process::Command::new("cmd")
            .args(["/C", "start", "", &url])
            .creation_flags(CREATE_NO_WINDOW)
            .spawn();
    });
}

pub fn notify_work_done() {
    std::thread::spawn(|| do_toast("focus  ·  session complete", "time for a break", false));
}

// ── Autostart ─────────────────────────────────────────────────────────────

pub fn get_autostart() -> bool {
    let hkcu = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER);
    hkcu.open_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Run")
        .and_then(|k| k.get_value::<String, _>("focus"))
        .is_ok()
}

pub fn set_autostart(enable: bool) {
    let Ok(exe) = std::env::current_exe() else { return };
    let exe = exe.to_string_lossy().into_owned();
    let hkcu = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER);
    let Ok(run) = hkcu.open_subkey_with_flags(
        "Software\\Microsoft\\Windows\\CurrentVersion\\Run",
        winreg::enums::KEY_WRITE,
    ) else { return };
    if enable { let _ = run.set_value("focus", &exe); }
    else      { let _ = run.delete_value("focus"); }
}

// ── Update check & auto-install ───────────────────────────────────────────

fn parse_version(v: &str) -> (u32, u32, u32) {
    let mut p = v.splitn(3, '.');
    let major = p.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    let minor = p.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    let patch = p.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    (major, minor, patch)
}

pub fn check_for_update(current_version: &str) {
    let current = current_version.to_owned();
    std::thread::spawn(move || {
        const API: &str =
            "https://api.github.com/repos/brand-ing/study-buddy/releases/latest";

        match ureq::get(API).set("User-Agent", "focus-app").call() {
            Ok(resp) => {
                let body = resp.into_string().unwrap_or_default();
                let json: serde_json::Value =
                    serde_json::from_str(&body).unwrap_or(serde_json::Value::Null);
                let latest = json["tag_name"]
                    .as_str()
                    .unwrap_or("")
                    .trim_start_matches('v');

                if latest.is_empty() {
                    do_toast("focus  ·  update check failed", "no release found on github", false);
                } else if parse_version(latest) <= parse_version(&current) {
                    do_toast(
                        "focus  ·  you're up to date",
                        &format!("version {} is the latest", current),
                        false,
                    );
                } else {
                    let url = format!(
                        "https://github.com/brand-ing/study-buddy/releases/download/v{}/focus.exe",
                        latest
                    );
                    download_and_stage_update(url, latest.to_owned());
                }
            }
            Err(_) => {
                do_toast(
                    "focus  ·  update check failed",
                    "couldn't reach github — check your connection",
                    false,
                );
            }
        }
    });
}

fn download_and_stage_update(url: String, version: String) {
    do_toast(
        &format!("focus  ·  {} available", version),
        "downloading in the background…",
        false,
    );

    let temp_new = std::env::temp_dir().join("focus_update.exe");

    let resp = match ureq::get(&url).set("User-Agent", "focus-app").call() {
        Ok(r) => r,
        Err(_) => {
            do_toast("focus  ·  update failed", "download failed — check your connection", false);
            return;
        }
    };

    let mut out = match std::fs::File::create(&temp_new) {
        Ok(f) => f,
        Err(_) => {
            do_toast("focus  ·  update failed", "could not write to temp directory", false);
            return;
        }
    };

    if std::io::copy(&mut resp.into_reader(), &mut out).is_err() {
        do_toast("focus  ·  update failed", "download incomplete", false);
        return;
    }
    drop(out);

    let Ok(current_exe) = std::env::current_exe() else {
        do_toast("focus  ·  update failed", "could not locate executable", false);
        return;
    };

    // Write a batch script that waits for this process to exit, swaps the
    // binary in place, and relaunches. Windows won't let us overwrite a
    // running exe, so the wait loop is necessary.
    let bat = std::env::temp_dir().join("focus_update.bat");
    let pid = std::process::id();
    let bat_content = format!(
        "@echo off\r\n\
         :wait\r\n\
         tasklist /FI \"PID eq {pid}\" 2>nul | find /I \"focus.exe\" >nul\r\n\
         if not errorlevel 1 (timeout /t 1 /nobreak >nul & goto :wait)\r\n\
         move /Y \"{src}\" \"{dst}\"\r\n\
         start \"\" \"{dst}\"\r\n\
         del \"%~f0\"\r\n",
        pid = pid,
        src = temp_new.display(),
        dst = current_exe.display(),
    );

    if std::fs::write(&bat, bat_content).is_err() {
        do_toast("focus  ·  update failed", "could not create updater script", false);
        return;
    }

    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    if std::process::Command::new("cmd")
        .arg("/C")
        .arg(&bat)
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .is_err()
    {
        do_toast("focus  ·  update failed", "could not launch updater", false);
        return;
    }

    do_toast(
        &format!("focus  ·  {} ready", version),
        "update will install when you close the app",
        true,
    );
}
