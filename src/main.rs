use std::sync::atomic::{AtomicBool, Ordering};
use tray_icon::TrayIconBuilder;
use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Power::{
    ES_CONTINUOUS, ES_DISPLAY_REQUIRED, ES_SYSTEM_REQUIRED, SetThreadExecutionState,
};
use windows_sys::Win32::System::StationsAndDesktops::{CloseDesktop, OpenInputDesktop};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW, HWND_MESSAGE,
    KillTimer, MSG, PBT_APMRESUMEAUTOMATIC, PBT_APMSUSPEND, PostMessageW, PostQuitMessage,
    RegisterClassExW, SetTimer, TranslateMessage, WM_DESTROY, WM_POWERBROADCAST,
    WM_QUERYENDSESSION, WM_TIMER, WNDCLASSEXW,
};

static IS_ACTIVE: AtomicBool = AtomicBool::new(true);
static IS_LOCKED: AtomicBool = AtomicBool::new(false);
static IS_SUSPENDED: AtomicBool = AtomicBool::new(false);

static mut TOGGLE_ITEM: Option<MenuItem> = None;
static mut QUIT_ITEM: Option<MenuItem> = None;

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_TIMER => {
            if wparam == 1 {
                let locked = check_screen_lock();
                let prev_locked = IS_LOCKED.load(Ordering::SeqCst);
                if locked != prev_locked {
                    IS_LOCKED.store(locked, Ordering::SeqCst);
                    update_execution_state();
                }

                let menu_channel = MenuEvent::receiver();
                if let Ok(event) = menu_channel.try_recv() {
                    unsafe {
                        if let Some(ref toggle_item) = TOGGLE_ITEM
                            && event.id == toggle_item.id()
                        {
                            let current = IS_ACTIVE.load(Ordering::SeqCst);
                            let new_state = !current;
                            IS_ACTIVE.store(new_state, Ordering::SeqCst);
                            update_execution_state();

                            if new_state {
                                toggle_item.set_text("Pause Espresso");
                            } else {
                                toggle_item.set_text("Resume Espresso");
                            }
                        }
                        if let Some(ref quit_item) = QUIT_ITEM
                            && event.id == quit_item.id()
                        {
                            PostMessageW(hwnd, WM_DESTROY, 0, 0);
                        }
                    }
                }
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
                SetThreadExecutionState(ES_CONTINUOUS);
            }
            1
        }
        WM_DESTROY => {
            unsafe {
                KillTimer(hwnd, 1);
                PostQuitMessage(0);
            }
            0
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

fn update_execution_state() {
    let active = IS_ACTIVE.load(Ordering::SeqCst);
    let locked = IS_LOCKED.load(Ordering::SeqCst);
    let suspended = IS_SUSPENDED.load(Ordering::SeqCst);

    unsafe {
        if active && !locked && !suspended {
            SetThreadExecutionState(ES_CONTINUOUS | ES_SYSTEM_REQUIRED | ES_DISPLAY_REQUIRED);
        } else {
            SetThreadExecutionState(ES_CONTINUOUS);
        }
    }
}

fn check_screen_lock() -> bool {
    unsafe {
        let hdesk = OpenInputDesktop(0, 0, 0x0001);
        if hdesk.is_null() {
            CloseDesktop(hdesk);
            false
        } else {
            true
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

    let hinstance = unsafe { GetModuleHandleW(std::ptr::null()) };

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
        RegisterClassExW(&window_class);
    }

    let hwnd = unsafe {
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

    update_execution_state();

    let tray_menu = Menu::new();
    let toggle_item = MenuItem::new("Pause Espresso", true, None);
    let quit_item = MenuItem::new("Quit", true, None);

    tray_menu.append(&toggle_item).unwrap();
    tray_menu.append(&PredefinedMenuItem::separator()).unwrap();
    tray_menu.append(&quit_item).unwrap();

    unsafe {
        TOGGLE_ITEM = Some(toggle_item);
        QUIT_ITEM = Some(quit_item);
    }

    let icon = create_tray_icon_image();

    let _tray_icon = TrayIconBuilder::new()
        .with_menu(Box::new(tray_menu))
        .with_tooltip("Espresso - Preventing Sleep")
        .with_icon(icon)
        .build()
        .unwrap();

    unsafe {
        SetTimer(hwnd, 1, 500, None);
    }

    let mut msg = MSG {
        hwnd: std::ptr::null_mut(),
        message: 0,
        wParam: 0,
        lParam: 0,
        time: 0,
        pt: windows_sys::Win32::Foundation::POINT { x: 0, y: 0 },
    };

    unsafe {
        while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        DestroyWindow(hwnd);
    }
}
