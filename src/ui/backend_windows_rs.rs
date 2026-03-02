use crate::app::Entry;
use std::ffi::OsStr;
use std::ffi::c_void;
use std::os::windows::ffi::OsStrExt;
use std::ptr::null_mut;

use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::WINDOW_LONG_PTR_INDEX;
use windows::Win32::UI::WindowsAndMessaging::{
    CW_USEDEFAULT, CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, HWND_MESSAGE,
    MSG, PostQuitMessage, RegisterClassW, SW_SHOW, ShowWindow, TranslateMessage, WM_COMMAND,
    WM_CREATE, WM_DESTROY, WNDCLASSW, WS_OVERLAPPEDWINDOW, WS_VISIBLE,
};
use windows::core::PCWSTR;

fn to_wide(s: &str) -> Vec<u16> {
    OsStr::new(s)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

const ID_OPEN: i32 = 1001;
const ID_CLOSE: i32 = 1002;

extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
        match msg {
            WM_CREATE => {
                // lparam is pointer to CREATESTRUCTW; lpCreateParams holds our boxed pointer
                // Retrieve and store as GWLP_USERDATA via SetWindowLongPtr if needed, but here we rely on lpCreateParams later
                DefWindowProcW(hwnd, msg, wparam, lparam)
            }
            WM_COMMAND => {
                let id = (wparam.0 & 0xffff) as i32;
                if id == ID_OPEN {
                    // Open first bookmark: retrieve from window's userdata via GetProp? Simpler: use GetWindowLongPtrW userdata
                    let data = windows::Win32::UI::WindowsAndMessaging::GetWindowLongPtrW(
                        hwnd,
                        WINDOW_LONG_PTR_INDEX::GWLP_USERDATA,
                    );
                    if data != 0 {
                        let vec_ptr = data as *mut Vec<Entry>;
                        if !vec_ptr.is_null() {
                            if let Some(entry) = (*vec_ptr).get(0) {
                                match entry {
                                    Entry::Bookmark { url, .. } => {
                                        let verb = to_wide("open");
                                        let urlw = to_wide(url);
                                        ShellExecuteW(
                                            hwnd,
                                            PCWSTR(verb.as_ptr()),
                                            PCWSTR(urlw.as_ptr()),
                                            PCWSTR(std::ptr::null()),
                                            PCWSTR(std::ptr::null()),
                                            SW_SHOW,
                                        );
                                    }
                                    Entry::App { command, args, .. } => {
                                        let mut cmd = std::process::Command::new(command);
                                        if !args.is_empty() {
                                            cmd.args(args);
                                        }
                                        let _ = cmd.spawn();
                                    }
                                }
                            }
                        }
                    }
                } else if id == ID_CLOSE {
                    PostQuitMessage(0);
                }
                LRESULT(0)
            }
            WM_DESTROY => {
                PostQuitMessage(0);
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}

pub fn run_app(bookmarks: Vec<Entry>) -> Result<(), Box<dyn std::error::Error>> {
    unsafe {
        let hinstance = GetModuleHandleW(PCWSTR(std::ptr::null()));

        let class_name = to_wide("bookmark_launcher_class");
        let wnd_class = WNDCLASSW {
            lpfnWndProc: Some(wndproc),
            hInstance: hinstance,
            lpszClassName: PCWSTR(class_name.as_ptr()),
            ..std::mem::zeroed()
        };

        RegisterClassW(&wnd_class);

        // Box bookmarks and pass pointer via lpParam
        let boxed: *mut Vec<Entry> = Box::into_raw(Box::new(bookmarks));

        let hwnd = CreateWindowExW(
            Default::default(),
            PCWSTR(class_name.as_ptr()),
            PCWSTR(to_wide("Bookmark Launcher").as_ptr()),
            WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            400,
            200,
            HWND(0),
            None,
            hinstance,
            Some(boxed as *mut c_void),
        );

        if hwnd.0 == 0 {
            // leak box on error
            let _ = Box::from_raw(boxed);
            return Err("Failed to create window".into());
        }

        // store boxed pointer in GWLP_USERDATA
        windows::Win32::UI::WindowsAndMessaging::SetWindowLongPtrW(
            hwnd,
            WINDOW_LONG_PTR_INDEX::GWLP_USERDATA,
            boxed as isize,
        );

        ShowWindow(hwnd, SW_SHOW);

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, HWND(0), 0, 0).into() {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        // cleanup boxed pointer
        let data = windows::Win32::UI::WindowsAndMessaging::GetWindowLongPtrW(
            hwnd,
            WINDOW_LONG_PTR_INDEX::GWLP_USERDATA,
        );
        if data != 0 {
            let vec_ptr = data as *mut Vec<Entry>;
            if !vec_ptr.is_null() {
                let _ = Box::from_raw(vec_ptr);
            }
        }

        Ok(())
    }
}
