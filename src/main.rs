use image::ImageReader;
use std::sync::atomic::{AtomicBool, Ordering};
use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};
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
const WM_TRAY_UPDATE: u32 = 0x8001;

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
        WM_TRAY_UPDATE => 0,
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
    let prevent = prevents_sleep();

    unsafe {
        // unsafe, since this is a call into the Windows API
        if prevent {
            SetThreadExecutionState(ES_CONTINUOUS | ES_SYSTEM_REQUIRED | ES_DISPLAY_REQUIRED);
        } else {
            SetThreadExecutionState(ES_CONTINUOUS);
        }
    }
}

fn prevents_sleep() -> bool {
    IS_ACTIVE.load(Ordering::SeqCst)
        && !IS_LOCKED.load(Ordering::SeqCst)
        && !IS_SUSPENDED.load(Ordering::SeqCst)
}

fn load_icon(path: &str) -> Icon {
    let image = ImageReader::open(path)
        .expect("Failed to open tray icon")
        .decode()
        .expect("Failed to decode tray icon")
        .into_rgba8();
    let (width, height) = image.dimensions();

    Icon::from_rgba(image.into_raw(), width, height).expect("Invalid tray icon")
}

fn tray_tooltip(preventing_sleep: bool) -> &'static str {
    if preventing_sleep {
        "Espresso - Preventing Sleep"
    } else {
        "Espresso - Sleep Prevention Paused"
    }
}

fn update_tray_icon(
    tray_icon: &TrayIcon,
    filled_icon: &Icon,
    empty_icon: &Icon,
    preventing_sleep: bool,
) {
    let icon = if preventing_sleep {
        filled_icon
    } else {
        empty_icon
    };

    tray_icon
        .set_icon(Some(icon.clone()))
        .expect("Failed to update tray icon");
    tray_icon
        .set_tooltip(Some(tray_tooltip(preventing_sleep)))
        .expect("Failed to update tray tooltip");
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

    let filled_icon = load_icon("assets/espresso_filled_32.png");
    let empty_icon = load_icon("assets/espresso_empty_32.png");
    let mut rendered_preventing_sleep = prevents_sleep();
    let initial_icon = if rendered_preventing_sleep {
        filled_icon.clone()
    } else {
        empty_icon.clone()
    };
    let tray_icon = TrayIconBuilder::new()
        .with_menu(Box::new(tray_menu))
        .with_tooltip(tray_tooltip(rendered_preventing_sleep))
        .with_icon(initial_icon)
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
                    unsafe {
                        // unsafe, since this is a call into the Windows API
                        PostMessageW(target_hwnd, WM_TRAY_UPDATE, 0, 0);
                    }
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

            let preventing_sleep = prevents_sleep();
            if preventing_sleep != rendered_preventing_sleep {
                update_tray_icon(&tray_icon, &filled_icon, &empty_icon, preventing_sleep);
                rendered_preventing_sleep = preventing_sleep;
            }
        }

        DestroyWindow(hwnd);
    }
}
