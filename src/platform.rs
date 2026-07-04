use std::sync::OnceLock;

static SHOW_HOTKEY_ID: OnceLock<u32> = OnceLock::new();

// ── Tray ──────────────────────────────────────────────────────────────────

pub fn setup_tray() {
    let icon = {
        const S: u32 = 32;
        let n = S as usize;
        let mut rgba = vec![0u8; n * n * 4];
        let c = 15.5_f32;
        for y in 0..n {
            for x in 0..n {
                let dx = x as f32 - c;
                let dy = y as f32 - c;
                let r = (dx * dx + dy * dy).sqrt();
                let i = (y * n + x) * 4;
                if r > 14.5 { continue; }
                if r >= 11.0 {
                    rgba[i] = 96; rgba[i+1] = 165; rgba[i+2] = 250; rgba[i+3] = 255;
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
        tray_icon::Icon::from_rgba(rgba, S, S).expect("tray icon rgba")
    };

    let menu = tray_icon::menu::Menu::new();
    let show_i = tray_icon::menu::MenuItem::with_id("show", "Show focus", true, None);
    let quit_i = tray_icon::menu::MenuItem::with_id("quit", "Quit", true, None);
    let sep    = tray_icon::menu::PredefinedMenuItem::separator();
    let _ = menu.append_items(&[&show_i, &sep, &quit_i]);

    let tray = tray_icon::TrayIconBuilder::new()
        .with_tooltip("focus")
        .with_icon(icon)
        .with_menu(Box::new(menu))
        .build()
        .expect("tray icon build");

    Box::leak(Box::new(tray));
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
