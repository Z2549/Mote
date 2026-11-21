//! Chrome window lookup and key injection.

use std::thread;
use std::time::Duration;

use mote_core::Command;
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::System::Threading::AttachThreadInput;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_KEYBOARD, KEYBD_EVENT_FLAGS, KEYBDINPUT, KEYEVENTF_EXTENDEDKEY,
    KEYEVENTF_KEYUP, MAPVK_VK_TO_VSC, MapVirtualKeyW, SendInput, VIRTUAL_KEY, VK_DOWN, VK_F, VK_R,
    VK_RIGHT, VK_UP, VK_X,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetClassNameW, GetForegroundWindow, GetWindowTextW, GetWindowThreadProcessId,
    IsIconic, IsWindowVisible, PostMessageW, SW_RESTORE, SetForegroundWindow, ShowWindow,
    WM_KEYDOWN, WM_KEYUP,
};
use windows::core::BOOL;

pub fn press(command: Command) {
    let Some(chrome) = find_chrome() else {
        return;
    };
    let key = key_for(command);
    let focused = focus(chrome);

    if !focused || send_key(key) == 0 {
        post_key(chrome, key);
    }
}

fn key_for(command: Command) -> VIRTUAL_KEY {
    match command {
        Command::Prev => VK_UP,
        Command::Next => VK_DOWN,
        Command::Forward => VK_RIGHT,
        Command::Author => VK_F,
        Command::Dislike => VK_R,
        Command::X => VK_X,
    }
}

fn send_key(key: VIRTUAL_KEY) -> u32 {
    // The scan code is what gives Chrome a non-empty `event.code`; Douyin's
    // shortcuts check it, so a bare virtual key is silently ignored there.
    // Arrow keys are extended keys: without the flag `→` arrives as Numpad6.
    let scan = unsafe { MapVirtualKeyW(key.0 as u32, MAPVK_VK_TO_VSC) } as u16;
    let extended = if is_extended(key) {
        KEYEVENTF_EXTENDEDKEY
    } else {
        KEYBD_EVENT_FLAGS::default()
    };
    let down = INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: key,
                wScan: scan,
                dwFlags: extended,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };
    let up = INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: key,
                wScan: scan,
                dwFlags: extended | KEYEVENTF_KEYUP,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };
    unsafe { SendInput(&[down, up], std::mem::size_of::<INPUT>() as i32) }
}

fn is_extended(key: VIRTUAL_KEY) -> bool {
    matches!(key, VK_UP | VK_DOWN | VK_RIGHT)
}

/// Last resort when Windows refuses to hand over the foreground.
fn post_key(hwnd: HWND, key: VIRTUAL_KEY) {
    let scan = unsafe { MapVirtualKeyW(key.0 as u32, MAPVK_VK_TO_VSC) } as isize;
    let extended = if is_extended(key) { 1 << 24 } else { 0 };
    let lparam = 1 | (scan << 16) | extended;

    unsafe {
        let _ = PostMessageW(
            Some(hwnd),
            WM_KEYDOWN,
            WPARAM(key.0 as usize),
            LPARAM(lparam),
        );
        let _ = PostMessageW(
            Some(hwnd),
            WM_KEYUP,
            WPARAM(key.0 as usize),
            LPARAM(lparam | (1 << 30) | (1 << 31)),
        );
    }
}

fn focus(hwnd: HWND) -> bool {
    unsafe {
        if IsIconic(hwnd).as_bool() {
            let _ = ShowWindow(hwnd, SW_RESTORE);
        }
        if GetForegroundWindow() == hwnd {
            return true;
        }

        let foreground = GetForegroundWindow();
        let from = GetWindowThreadProcessId(foreground, None);
        let to = GetWindowThreadProcessId(hwnd, None);
        let attached = AttachThreadInput(from, to, true).as_bool();
        let _ = SetForegroundWindow(hwnd);
        if attached {
            let _ = AttachThreadInput(from, to, false);
        }

        thread::sleep(Duration::from_millis(16));
        GetForegroundWindow() == hwnd
    }
}

fn find_chrome() -> Option<HWND> {
    let foreground = unsafe { GetForegroundWindow() };
    let mut finder = Finder::default();
    unsafe {
        let _ = EnumWindows(Some(visit), LPARAM(&mut finder as *mut Finder as isize));
    }

    finder
        .named
        .or_else(|| is_chrome(foreground).then_some(foreground))
        .or(finder.any)
}

#[derive(Default)]
struct Finder {
    named: Option<HWND>,
    any: Option<HWND>,
}

unsafe extern "system" fn visit(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let finder = unsafe { &mut *(lparam.0 as *mut Finder) };
    if !unsafe { IsWindowVisible(hwnd) }.as_bool() || !is_chrome(hwnd) {
        return true.into();
    }

    if finder.any.is_none() {
        finder.any = Some(hwnd);
    }

    let title = title_of(hwnd);
    if title.contains("抖音") || title.to_lowercase().contains("douyin") {
        finder.named = Some(hwnd);
        return false.into();
    }
    true.into()
}

fn is_chrome(hwnd: HWND) -> bool {
    if hwnd.0.is_null() {
        return false;
    }
    let mut class = [0u16; 64];
    let len = unsafe { GetClassNameW(hwnd, &mut class) };
    len > 0 && String::from_utf16_lossy(&class[..len as usize]) == "Chrome_WidgetWin_1"
}

fn title_of(hwnd: HWND) -> String {
    let mut title = [0u16; 512];
    let len = unsafe { GetWindowTextW(hwnd, &mut title) };
    String::from_utf16_lossy(&title[..len.max(0) as usize])
}
