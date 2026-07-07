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
    let pause_i  = tray_icon::menu::MenuItem::with_id("pause_resume", "Resume timer",     true, None);
    let sep      = tray_icon::menu::PredefinedMenuItem::separator();
    let show_i   = tray_icon::menu::MenuItem::with_id("show",         "Show focus",       true, None);
    let _ = menu.append_items(&[&quit_i, &update_i, &sep, &pause_i, &show_i]);

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

pub fn notify_work_done() {
    std::thread::spawn(|| {
        let _ = winrt_notification::Toast::new(winrt_notification::Toast::POWERSHELL_APP_ID)
            .title("focus  ·  session complete")
            .text1("time for a break")
            .duration(winrt_notification::Duration::Short)
            .show();
    });
}

// ── Autostart ─────────────────────────────────────────────────────────────

pub fn get_autostart() -> bool {
    let hkcu = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER);
    hkcu.open_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Run")
        .and_then(|k| k.get_value::<String, _>("focus"))
        .is_ok()
}

// ── Update check ──────────────────────────────────────────────────────────

pub fn check_for_update(current_version: &str) {
    let current = current_version.to_owned();
    std::thread::spawn(move || {
        const API: &str =
            "https://api.github.com/repos/brand-ing/study-buddy/releases/latest";

        let result = ureq::get(API)
            .set("User-Agent", "focus-app")
            .call();

        let toast = |title: &str, body: &str, long: bool| {
            let dur = if long {
                winrt_notification::Duration::Long
            } else {
                winrt_notification::Duration::Short
            };
            let _ = winrt_notification::Toast::new(winrt_notification::Toast::POWERSHELL_APP_ID)
                .title(title)
                .text1(body)
                .duration(dur)
                .show();
        };

        match result {
            Ok(resp) => {
                let body = resp.into_string().unwrap_or_default();
                let json: serde_json::Value =
                    serde_json::from_str(&body).unwrap_or(serde_json::Value::Null);
                let latest = json["tag_name"]
                    .as_str()
                    .unwrap_or("")
                    .trim_start_matches('v');

                if latest.is_empty() {
                    toast("focus  ·  update check failed", "no release found on github", false);
                } else if latest == current.as_str() {
                    toast(
                        "focus  ·  you're up to date",
                        &format!("version {} is the latest", current),
                        false,
                    );
                } else {
                    toast(
                        &format!("focus  ·  {} available", latest),
                        "visit github.com/brand-ing/study-buddy to download",
                        true,
                    );
                }
            }
            Err(_) => {
                toast(
                    "focus  ·  update check failed",
                    "couldn't reach github — check your connection",
                    false,
                );
            }
        }
    });
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
