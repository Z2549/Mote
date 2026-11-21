//! Tray icon: minimising Mote keeps it one click away instead of the taskbar.

use std::mem::size_of;
use std::sync::OnceLock;

use eframe::egui;
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CreateBitmap, CreateDIBSection, DIB_RGB_COLORS,
    DeleteObject,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::GetCurrentProcessId;
use windows::Win32::UI::Shell::{
    NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NOTIFYICONDATAW, Shell_NotifyIconW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreateIconIndirect, CreatePopupMenu, CreateWindowExW, DefWindowProcW,
    DestroyMenu, DispatchMessageW, EnumWindows, GetClassNameW, GetCursorPos, GetMessageW,
    GetWindowTextW, GetWindowThreadProcessId, HICON, ICONINFO, IsIconic, MF_STRING, MSG,
    RegisterClassW, SW_RESTORE, SW_SHOW, SetForegroundWindow, ShowWindow, TPM_NONOTIFY,
    TPM_RETURNCMD, TPM_RIGHTBUTTON, TrackPopupMenu, TranslateMessage, WM_APP, WM_LBUTTONUP,
    WM_NULL, WM_RBUTTONUP, WNDCLASSW, WS_OVERLAPPED,
};
use windows::core::{BOOL, PCWSTR, w};

const ICON_MESSAGE: u32 = WM_APP + 1;
const SHOW: usize = 1;
const QUIT: usize = 2;
const CLASS: PCWSTR = w!("MoteTray");

static CONTEXT: OnceLock<egui::Context> = OnceLock::new();

pub fn spawn(ctx: egui::Context) {
    let _ = CONTEXT.set(ctx);
    std::thread::spawn(|| unsafe { run() });
}

unsafe fn run() {
    let instance = unsafe { GetModuleHandleW(None) }.unwrap_or_default();
    let window_class = WNDCLASSW {
        lpfnWndProc: Some(wndproc),
        hInstance: HINSTANCE(instance.0),
        lpszClassName: CLASS,
        ..Default::default()
    };
    if unsafe { RegisterClassW(&window_class) } == 0 {
        return;
    }

    let Ok(hwnd) = (unsafe {
        CreateWindowExW(
            Default::default(),
            CLASS,
            w!("Mote"),
            WS_OVERLAPPED,
            0,
            0,
            0,
            0,
            None,
            None,
            Some(HINSTANCE(instance.0)),
            None,
        )
    }) else {
        return;
    };

    let icon = unsafe { icon() }.unwrap_or_default();
    let mut data = NOTIFYICONDATAW {
        cbSize: size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: 1,
        uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP,
        uCallbackMessage: ICON_MESSAGE,
        hIcon: icon,
        ..Default::default()
    };
    for (slot, unit) in data.szTip.iter_mut().zip("Mote".encode_utf16()) {
        *slot = unit;
    }
    if !unsafe { Shell_NotifyIconW(NIM_ADD, &data) }.as_bool() {
        return;
    }

    let mut message = MSG::default();
    while unsafe { GetMessageW(&mut message, None, 0, 0) }.0 > 0 {
        unsafe {
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
}

unsafe extern "system" fn wndproc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if message == ICON_MESSAGE {
        match lparam.0 as u32 {
            WM_LBUTTONUP => show(),
            WM_RBUTTONUP => unsafe { menu(hwnd) },
            _ => {}
        }
        return LRESULT(0);
    }
    unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
}

fn show() {
    if let Some(hwnd) = main_window() {
        unsafe {
            if IsIconic(hwnd).as_bool() {
                let _ = ShowWindow(hwnd, SW_RESTORE);
            } else {
                let _ = ShowWindow(hwnd, SW_SHOW);
            }
            let _ = SetForegroundWindow(hwnd);
        }
    }
    if let Some(ctx) = CONTEXT.get() {
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        ctx.request_repaint();
    }
}

/// The eframe window is the only window this process owns that is not the tray.
fn main_window() -> Option<HWND> {
    let mut finder = Finder {
        pid: unsafe { GetCurrentProcessId() },
        found: None,
    };
    unsafe {
        let _ = EnumWindows(
            Some(visit_main),
            LPARAM(&mut finder as *mut Finder as isize),
        );
    }
    finder.found
}

struct Finder {
    pid: u32,
    found: Option<HWND>,
}

unsafe extern "system" fn visit_main(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let finder = unsafe { &mut *(lparam.0 as *mut Finder) };
    let mut pid = 0;
    unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
    if pid != finder.pid || unsafe { class_name(hwnd) } == "MoteTray" {
        return true.into();
    }
    if unsafe { title(hwnd) } != "Mote" {
        return true.into();
    }
    finder.found = Some(hwnd);
    false.into()
}

unsafe fn class_name(hwnd: HWND) -> String {
    let mut buffer = [0u16; 64];
    let len = unsafe { GetClassNameW(hwnd, &mut buffer) };
    String::from_utf16_lossy(&buffer[..len.max(0) as usize])
}

unsafe fn title(hwnd: HWND) -> String {
    let mut buffer = [0u16; 128];
    let len = unsafe { GetWindowTextW(hwnd, &mut buffer) };
    String::from_utf16_lossy(&buffer[..len.max(0) as usize])
}

unsafe fn menu(hwnd: HWND) {
    let Ok(menu) = (unsafe { CreatePopupMenu() }) else {
        return;
    };
    unsafe {
        let _ = AppendMenuW(menu, MF_STRING, SHOW, w!("显示"));
        let _ = AppendMenuW(menu, MF_STRING, QUIT, w!("退出"));
    }

    let mut point = POINT::default();
    let _ = unsafe { GetCursorPos(&mut point) };
    unsafe {
        let _ = SetForegroundWindow(hwnd);
    }
    let chosen = unsafe {
        TrackPopupMenu(
            menu,
            TPM_RETURNCMD | TPM_NONOTIFY | TPM_RIGHTBUTTON,
            point.x,
            point.y,
            None,
            hwnd,
            None,
        )
    };
    unsafe {
        let _ = DestroyMenu(menu);
        let _ = windows::Win32::UI::WindowsAndMessaging::PostMessageW(
            Some(hwnd),
            WM_NULL,
            WPARAM(0),
            LPARAM(0),
        );
    }

    match chosen.0 as usize {
        SHOW => show(),
        QUIT => {
            if let Some(ctx) = CONTEXT.get() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
        _ => {}
    }
}

/// Turns the PNG that is already embedded in the window into a tray HICON.
unsafe fn icon() -> Option<HICON> {
    let data = eframe::icon_data::from_png_bytes(include_bytes!("../assets/icon.png")).ok()?;
    let width = i32::try_from(data.width).ok()?;
    let height = i32::try_from(data.height).ok()?;

    let info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width,
            biHeight: -height,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut bits = std::ptr::null_mut();
    let color = unsafe { CreateDIBSection(None, &info, DIB_RGB_COLORS, &mut bits, None, 0) }.ok()?;
    if bits.is_null() {
        unsafe {
            let _ = DeleteObject(color.into());
        }
        return None;
    }

    let pixels =
        unsafe { std::slice::from_raw_parts_mut(bits as *mut u8, data.rgba.len()) };
    for (rgba, bgra) in data.rgba.chunks_exact(4).zip(pixels.chunks_exact_mut(4)) {
        bgra[0] = rgba[2];
        bgra[1] = rgba[1];
        bgra[2] = rgba[0];
        bgra[3] = rgba[3];
    }

    let mask = unsafe { CreateBitmap(width, height, 1, 1, None) };
    let made = unsafe {
        CreateIconIndirect(&ICONINFO {
            fIcon: BOOL(1),
            xHotspot: 0,
            yHotspot: 0,
            hbmMask: mask,
            hbmColor: color,
        })
    };
    unsafe {
        let _ = DeleteObject(color.into());
        let _ = DeleteObject(mask.into());
    }
    made.ok()
}
