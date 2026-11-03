use std::sync::{OnceLock, Mutex};
use std::sync::atomic::{AtomicPtr, Ordering};
use crate::data::Phase;

// ── In-app update events ──────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum UpdateInfo {
    Checking,
    UpToDate,
    Available { version: String, url: String },
    Downloading,
    Ready { version: String },
    Failed(String),
}

static PENDING_UPDATE: Mutex<Option<UpdateInfo>> = Mutex::new(None);

fn push_update(info: UpdateInfo) {
    if let Ok(mut g) = PENDING_UPDATE.lock() { *g = Some(info); }
}

pub fn poll_update() -> Option<UpdateInfo> {
    PENDING_UPDATE.lock().ok()?.take()
}

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

    let menu      = tray_icon::menu::Menu::new();
    let quit_i    = tray_icon::menu::MenuItem::with_id("quit",         "Quit",             true, None);
    let update_i  = tray_icon::menu::MenuItem::with_id("check_update", "Check for update", true, None);
    let about_i   = tray_icon::menu::MenuItem::with_id("about",        "About focus",      true, None);
    let pause_i   = tray_icon::menu::MenuItem::with_id("pause_resume", "Resume timer",     true, None);
    let sep       = tray_icon::menu::PredefinedMenuItem::separator();
    let show_i    = tray_icon::menu::MenuItem::with_id("show",         "Show focus",       true, None);
    let tasks_i   = tray_icon::menu::MenuItem::with_id("show_tasks",   "Show tasks",       true, None);
    let _ = menu.append_items(&[&quit_i, &update_i, &about_i, &sep, &pause_i, &show_i, &tasks_i]);

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

// ── Window placement ──────────────────────────────────────────────────────
//
// Win32 reports physical pixels, but iced's resize/move_to take logical
// pixels, so everything here is divided by the window's DPI scale. The
// work area (monitor minus taskbar) of the monitor the window is on is used,
// so placement is correct on laptops with 125%/150% scaling and multi-monitor.

/// Work area of the window's monitor in logical pixels: (left, top, right, bottom).
fn work_area() -> (f32, f32, f32, f32) {
    unsafe {
        use windows_sys::Win32::Graphics::Gdi::{
            GetMonitorInfoW, MonitorFromWindow, MONITORINFO,
            MONITOR_DEFAULTTONEAREST, MONITOR_DEFAULTTOPRIMARY,
        };
        let hwnd = find_window();
        let flags = if hwnd != 0 { MONITOR_DEFAULTTONEAREST } else { MONITOR_DEFAULTTOPRIMARY };
        let monitor = MonitorFromWindow(hwnd, flags);
        let mut info: MONITORINFO = std::mem::zeroed();
        info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
        let s = scale();
        if GetMonitorInfoW(monitor, &mut info) == 0 {
            return (0.0, 0.0, 1280.0 / s, 720.0 / s);
        }
        let r = info.rcWork;
        (r.left as f32 / s, r.top as f32 / s, r.right as f32 / s, r.bottom as f32 / s)
    }
}

/// DPI scale factor of the window (1.0 = 100%, 1.5 = 150%).
fn scale() -> f32 {
    unsafe {
        use windows_sys::Win32::UI::HiDpi::{GetDpiForSystem, GetDpiForWindow};
        let hwnd = find_window();
        let dpi = if hwnd != 0 { GetDpiForWindow(hwnd) } else { GetDpiForSystem() };
        if dpi == 0 { 1.0 } else { dpi as f32 / 96.0 }
    }
}

const SCREEN_MARGIN: f32 = 10.0;

/// Bottom-right corner of the work area for the task dock. Returns the
/// logical (x, y, height); the height shrinks to fit short screens.
pub fn dock_rect(width: f32, height: f32) -> (f32, f32, f32) {
    let (left, top, right, bottom) = work_area();
    let h = height.min(bottom - top - SCREEN_MARGIN * 2.0).max(200.0);
    let x = (right - width - SCREEN_MARGIN).max(left);
    let y = (bottom - h - SCREEN_MARGIN).max(top);
    (x, y, h)
}

/// Where to put the window after it changes to `width` x `height`: keeps it
/// horizontally centred on where it is now and clamps it inside the work area.
pub fn fit_on_screen(width: f32, height: f32) -> (f32, f32) {
    let (left, top, right, bottom) = work_area();
    let (mut x, mut y) = (left + SCREEN_MARGIN, top + SCREEN_MARGIN);
    unsafe {
        use windows_sys::Win32::Foundation::RECT;
        use windows_sys::Win32::UI::WindowsAndMessaging::GetWindowRect;
        let hwnd = find_window();
        let mut r: RECT = std::mem::zeroed();
        if hwnd != 0 && GetWindowRect(hwnd, &mut r) != 0 {
            let s = scale();
            let cur_cx = (r.left + r.right) as f32 / 2.0 / s;
            x = cur_cx - width / 2.0;
            y = r.top as f32 / s;
        }
    }
    let x = x.min(right - width - SCREEN_MARGIN).max(left);
    let y = y.min(bottom - height - SCREEN_MARGIN).max(top);
    (x, y)
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

fn find_window() -> windows_sys::Win32::Foundation::HWND {
    unsafe {
        use windows_sys::Win32::UI::WindowsAndMessaging::FindWindowW;
        let title: Vec<u16> = "focus\0".encode_utf16().collect();
        FindWindowW(std::ptr::null(), title.as_ptr())
    }
}

pub fn hide_window() {
    unsafe {
        use windows_sys::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_HIDE};
        let hwnd = find_window();
        if hwnd != 0 { ShowWindow(hwnd, SW_HIDE); }
    }
}

pub fn show_window() {
    unsafe {
        use windows_sys::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            BringWindowToTop, GetForegroundWindow, GetWindowThreadProcessId,
            SetForegroundWindow, ShowWindow, SW_RESTORE,
        };
        let hwnd = find_window();
        if hwnd == 0 { return; }
        // Windows refuses SetForegroundWindow from a background app unless
        // its input is attached to the current foreground thread, which is
        // why showing only "often" worked before.
        let fg_thread = GetWindowThreadProcessId(GetForegroundWindow(), std::ptr::null_mut());
        let me = GetCurrentThreadId();
        let attached = fg_thread != 0 && fg_thread != me && AttachThreadInput(me, fg_thread, 1) != 0;
        ShowWindow(hwnd, SW_RESTORE);
        BringWindowToTop(hwnd);
        SetForegroundWindow(hwnd);
        if attached { AttachThreadInput(me, fg_thread, 0); }
    }
}

/// Visible on screen (not hidden to tray or minimised).
pub fn is_window_shown() -> bool {
    unsafe {
        use windows_sys::Win32::UI::WindowsAndMessaging::{IsIconic, IsWindowVisible};
        let hwnd = find_window();
        hwnd != 0 && IsWindowVisible(hwnd) != 0 && IsIconic(hwnd) == 0
    }
}

pub fn is_foreground() -> bool {
    unsafe {
        use windows_sys::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
        let hwnd = find_window();
        hwnd != 0 && GetForegroundWindow() == hwnd
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

// ── Update check & install ────────────────────────────────────────────────

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
                    push_update(UpdateInfo::Failed("no release found on GitHub".into()));
                } else if parse_version(latest) <= parse_version(&current) {
                    push_update(UpdateInfo::UpToDate);
                } else {
                    let url = format!(
                        "https://github.com/brand-ing/study-buddy/releases/download/v{}/focus.exe",
                        latest
                    );
                    push_update(UpdateInfo::Available { version: latest.to_owned(), url });
                }
            }
            Err(_) => push_update(UpdateInfo::Failed(
                "couldn't reach GitHub — check your connection".into(),
            )),
        }
    });
}

// Called when the user confirms they want to download the update.
pub fn trigger_download(url: String, version: String) {
    push_update(UpdateInfo::Downloading);
    std::thread::spawn(move || {
        let temp_new = std::env::temp_dir().join("focus_update.exe");

        let resp = match ureq::get(&url).set("User-Agent", "focus-app").call() {
            Ok(r) => r,
            Err(_) => {
                push_update(UpdateInfo::Failed("download failed — check your connection".into()));
                return;
            }
        };

        let mut out = match std::fs::File::create(&temp_new) {
            Ok(f) => f,
            Err(_) => {
                push_update(UpdateInfo::Failed("could not write to temp directory".into()));
                return;
            }
        };

        if std::io::copy(&mut resp.into_reader(), &mut out).is_err() {
            push_update(UpdateInfo::Failed("download incomplete".into()));
            return;
        }
        drop(out);

        let Ok(current_exe) = std::env::current_exe() else {
            push_update(UpdateInfo::Failed("could not locate executable".into()));
            return;
        };

        // Batch script waits for this process to exit, swaps the binary, relaunches.
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
            push_update(UpdateInfo::Failed("could not create updater script".into()));
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
            push_update(UpdateInfo::Failed("could not launch updater".into()));
            return;
        }

        push_update(UpdateInfo::Ready { version });
    });
}
