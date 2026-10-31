//! Which app the user is typing into, for per-app writing styles.
//!
//! The id is what a style rule matches on: the bundle identifier on macOS, the
//! executable name on Windows and the window class on X11. Wayland offers no
//! way to ask, so Linux sessions there fall back to the default style.

use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Type)]
pub struct ActiveApp {
    pub id: String,
    pub name: String,
}

/// The app that has keyboard focus right now.
pub fn frontmost() -> Option<ActiveApp> {
    imp::frontmost()
}

/// Apps with a visible window, for the "add an app" picker. Sorted by name.
pub fn running() -> Vec<ActiveApp> {
    let mut apps = imp::running();
    apps.sort_by_key(|app| app.name.to_lowercase());
    apps.dedup_by(|a, b| a.id == b.id);
    apps
}

#[cfg(target_os = "macos")]
mod imp {
    use super::ActiveApp;
    use objc2_app_kit::{NSApplicationActivationPolicy, NSRunningApplication, NSWorkspace};

    fn describe(app: &NSRunningApplication) -> Option<ActiveApp> {
        let id = app.bundleIdentifier()?.to_string();
        let name = app
            .localizedName()
            .map(|n| n.to_string())
            .unwrap_or_else(|| id.clone());
        Some(ActiveApp { id, name })
    }

    pub fn frontmost() -> Option<ActiveApp> {
        let workspace = NSWorkspace::sharedWorkspace();
        let app = workspace.frontmostApplication()?;
        describe(&app)
    }

    pub fn running() -> Vec<ActiveApp> {
        let workspace = NSWorkspace::sharedWorkspace();
        workspace
            .runningApplications()
            .iter()
            .filter(|app| app.activationPolicy() == NSApplicationActivationPolicy::Regular)
            .filter_map(|app| describe(&app))
            .collect()
    }
}

#[cfg(target_os = "windows")]
mod imp {
    use super::ActiveApp;
    use windows::core::PWSTR;
    use windows::Win32::Foundation::{CloseHandle, HWND, LPARAM, TRUE};
    use windows::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_FORMAT,
        PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetForegroundWindow, GetWindowTextLengthW, GetWindowThreadProcessId,
        IsWindowVisible,
    };

    fn exe_name(hwnd: HWND) -> Option<String> {
        let mut pid = 0u32;
        unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
        if pid == 0 {
            return None;
        }
        let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }.ok()?;
        let mut buf = [0u16; 1024];
        let mut len = buf.len() as u32;
        let result = unsafe {
            QueryFullProcessImageNameW(
                handle,
                PROCESS_NAME_FORMAT(0),
                PWSTR(buf.as_mut_ptr()),
                &mut len,
            )
        };
        let _ = unsafe { CloseHandle(handle) };
        result.ok()?;
        let path = String::from_utf16_lossy(&buf[..len as usize]);
        path.rsplit(['\\', '/']).next().map(str::to_string)
    }

    fn describe(exe: String) -> ActiveApp {
        let name = exe
            .strip_suffix(".exe")
            .or_else(|| exe.strip_suffix(".EXE"))
            .unwrap_or(&exe)
            .to_string();
        ActiveApp {
            id: exe.to_lowercase(),
            name,
        }
    }

    pub fn frontmost() -> Option<ActiveApp> {
        let hwnd = unsafe { GetForegroundWindow() };
        exe_name(hwnd).map(describe)
    }

    unsafe extern "system" fn collect(hwnd: HWND, lparam: LPARAM) -> windows::core::BOOL {
        let apps = &mut *(lparam.0 as *mut Vec<ActiveApp>);
        if IsWindowVisible(hwnd).as_bool() && GetWindowTextLengthW(hwnd) > 0 {
            if let Some(exe) = exe_name(hwnd) {
                apps.push(describe(exe));
            }
        }
        TRUE
    }

    pub fn running() -> Vec<ActiveApp> {
        let mut apps: Vec<ActiveApp> = Vec::new();
        let _ = unsafe { EnumWindows(Some(collect), LPARAM(&mut apps as *mut _ as isize)) };
        apps
    }
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
mod imp {
    use super::ActiveApp;
    use std::process::Command;

    fn xprop(args: &[&str]) -> Option<String> {
        let out = Command::new("xprop").args(args).output().ok()?;
        out.status
            .success()
            .then(|| String::from_utf8_lossy(&out.stdout).into_owned())
    }

    /// X11 only: `xprop` reads the active window's WM_CLASS.
    pub fn frontmost() -> Option<ActiveApp> {
        let root = xprop(&["-root", "_NET_ACTIVE_WINDOW"])?;
        let window = root.split_whitespace().last()?.to_string();
        if !window.starts_with("0x") || window == "0x0" {
            return None;
        }
        let class = xprop(&["-id", &window, "WM_CLASS"])?;
        // WM_CLASS(STRING) = "instance", "Class"
        let name = class
            .split('"')
            .nth(3)
            .or_else(|| class.split('"').nth(1))?;
        Some(ActiveApp {
            id: name.to_lowercase(),
            name: name.to_string(),
        })
    }

    pub fn running() -> Vec<ActiveApp> {
        Vec::new()
    }
}
