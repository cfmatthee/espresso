use std::sync::atomic::{AtomicBool, Ordering};
use tray_icon::TrayIconBuilder;
use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Power::{
    ES_CONTINUOUS, ES_DISPLAY_REQUIRED, ES_SYSTEM_REQUIRED, SetThreadExecutionState,
};
use windows_sys::Win32::System::RemoteDesktop::{
    NOTIFY_FOR_THIS_SESSION, WTSRegisterSessionNotification, WTSUnRegisterSessionNotification,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW, HWND_MESSAGE,
    MSG, PBT_APMRESUMEAUTOMATIC, PBT_APMSUSPEND, PostMessageW, PostQuitMessage, RegisterClassExW,
    TranslateMessage, WM_DESTROY, WM_POWERBROADCAST, WM_QUERYENDSESSION, WM_WTSSESSION_CHANGE,
    WNDCLASSEXW,
};

const WTS_SESSION_LOCK: u32 = 0x7;
const WTS_SESSION_UNLOCK: u32 = 0x8;

static IS_ACTIVE: AtomicBool = AtomicBool::new(true);
static IS_LOCKED: AtomicBool = AtomicBool::new(false);
static IS_SUSPENDED: AtomicBool = AtomicBool::new(false);

// unsafe due to:
//   FFI Callback: This function is passed to the Windows operating system
//   Unsafe Operation Inside: This function calls Windows API functions
unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_WTSSESSION_CHANGE => {
            let event = wparam as u32;
            if event == WTS_SESSION_LOCK {
                IS_LOCKED.store(true, Ordering::SeqCst);
                update_execution_state();
            } else if event == WTS_SESSION_UNLOCK {
                IS_LOCKED.store(false, Ordering::SeqCst);
                update_execution_state();
            }
            0
        }
        WM_POWERBROADCAST => {
            let event = wparam as u32;
            if event == PBT_APMSUSPEND {
                IS_SUSPENDED.store(true, Ordering::SeqCst);
                update_execution_state();
            } else if event == PBT_APMRESUMEAUTOMATIC {
                IS_SUSPENDED.store(false, Ordering::SeqCst);
                update_execution_state();
            }
            1
        }
        WM_QUERYENDSESSION => {
            unsafe {
                // unsafe, since this is a call into the Windows API
                SetThreadExecutionState(ES_CONTINUOUS);
            }
            1
        }
        WM_DESTROY => {
            unsafe {
                // unsafe, since this is a call into the Windows API
                WTSUnRegisterSessionNotification(hwnd);
                PostQuitMessage(0);
            }
            0
        }
        _ => unsafe {
            // unsafe, since this is a call into the Windows API
            DefWindowProcW(hwnd, msg, wparam, lparam)
        },
    }
}

fn update_execution_state() {
    let active = IS_ACTIVE.load(Ordering::SeqCst);
    let locked = IS_LOCKED.load(Ordering::SeqCst);
    let suspended = IS_SUSPENDED.load(Ordering::SeqCst);

    unsafe {
        if active && !locked && !suspended {
            // unsafe, since this is a call into the Windows API
            SetThreadExecutionState(ES_CONTINUOUS | ES_SYSTEM_REQUIRED | ES_DISPLAY_REQUIRED);
        } else {
            // unsafe, since this is a call into the Windows API
            SetThreadExecutionState(ES_CONTINUOUS);
        }
    }
}

fn create_tray_icon_image() -> tray_icon::Icon {
    let width = 16;
    let height = 16;
    let mut rgba = vec![0u8; width * height * 4];
    for y in 0..height {
        for x in 0..width {
            let idx = (y * width + x) * 4;
            if (2..14).contains(&x) && (2..14).contains(&y) {
                rgba[idx] = 34;
                rgba[idx + 1] = 139;
                rgba[idx + 2] = 34;
                rgba[idx + 3] = 255;
            } else {
                rgba[idx + 3] = 0;
            }
        }
    }
    tray_icon::Icon::from_rgba(rgba, width as u32, height as u32).expect("Failed to create icon")
}

fn main() {
    let class_name: Vec<u16> = "EspressoMessageWindowClass\0".encode_utf16().collect();
    let window_name: Vec<u16> = "Espresso Helper\0".encode_utf16().collect();

    let hinstance = unsafe {
        // unsafe, since this is a call into the Windows API
        GetModuleHandleW(std::ptr::null())
    };

    let window_class = WNDCLASSEXW {
        cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
        style: 0,
        lpfnWndProc: Some(window_proc),
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: hinstance,
        hIcon: std::ptr::null_mut(),
        hCursor: std::ptr::null_mut(),
        hbrBackground: std::ptr::null_mut(),
        lpszMenuName: std::ptr::null(),
        lpszClassName: class_name.as_ptr(),
        hIconSm: std::ptr::null_mut(),
    };

    unsafe {
        // unsafe, since this is a call into the Windows API
        RegisterClassExW(&window_class);
    }

    let hwnd = unsafe {
        // unsafe, since this is a call into the Windows API
        CreateWindowExW(
            0,
            class_name.as_ptr(),
            window_name.as_ptr(),
            0,
            0,
            0,
            0,
            0,
            HWND_MESSAGE,
            std::ptr::null_mut(),
            hinstance,
            std::ptr::null(),
        )
    };

    if hwnd.is_null() {
        panic!("Failed to create message window");
    }

    // Register for session notifications (lock/unlock events)
    unsafe {
        // unsafe, since this is a call into the Windows API
        WTSRegisterSessionNotification(hwnd, NOTIFY_FOR_THIS_SESSION);
    }

    update_execution_state();

    let tray_menu = Menu::new();
    let toggle_item = MenuItem::new("Pause Espresso", true, None);
    let quit_item = MenuItem::new("Quit", true, None);

    tray_menu.append(&toggle_item).unwrap();
    tray_menu.append(&PredefinedMenuItem::separator()).unwrap();
    tray_menu.append(&quit_item).unwrap();

    let toggle_id = toggle_item.id().clone();
    let quit_id = quit_item.id().clone();

    let icon = create_tray_icon_image();
    let _tray_icon = TrayIconBuilder::new()
        .with_menu(Box::new(tray_menu))
        .with_tooltip("Espresso - Preventing Sleep")
        .with_icon(icon)
        .build()
        .unwrap();

    let hwnd_addr = hwnd as usize;
    std::thread::spawn(move || {
        let menu_channel = MenuEvent::receiver();
        let target_hwnd = hwnd_addr as HWND;
        loop {
            if let Ok(event) = menu_channel.recv() {
                if event.id == quit_id {
                    unsafe {
                        // unsafe, since this is a call into the Windows API
                        PostMessageW(target_hwnd, WM_DESTROY, 0, 0);
                    }
                    break;
                } else if event.id == toggle_id {
                    let current = IS_ACTIVE.load(Ordering::SeqCst);
                    IS_ACTIVE.store(!current, Ordering::SeqCst);
                    update_execution_state();
                }
            }
        }
    });

    let mut msg = MSG {
        hwnd: std::ptr::null_mut(),
        message: 0,
        wParam: 0,
        lParam: 0,
        time: 0,
        pt: windows_sys::Win32::Foundation::POINT { x: 0, y: 0 },
    };

    unsafe {
        // unsafe, since this is a call into the Windows API
        while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        DestroyWindow(hwnd);
    }
}
