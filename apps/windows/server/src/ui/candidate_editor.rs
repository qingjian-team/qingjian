//! 候选译文修改的系统原生小窗口。

use std::cell::RefCell;

use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::SetFocus;
use windows::Win32::UI::WindowsAndMessaging::{
    BS_DEFPUSHBUTTON, BS_PUSHBUTTON, CREATESTRUCTW, CS_HREDRAW, CS_VREDRAW, CreateWindowExW,
    DefWindowProcW, DestroyWindow, ES_AUTOHSCROLL, GetWindowTextW, HMENU, IDC_ARROW, LoadCursorW,
    SW_SHOW, SetForegroundWindow, ShowWindow, WINDOW_STYLE, WM_CLOSE, WM_COMMAND, WM_CREATE,
    WM_DESTROY, WNDCLASSEXW, WS_CAPTION, WS_CHILD, WS_EX_TOOLWINDOW, WS_OVERLAPPED, WS_SYSMENU,
    WS_TABSTOP, WS_VISIBLE,
};
use windows::core::{PCWSTR, Result, w};

use super::candidates::click;
use super::window_class::WindowClass;
use crate::dispatch::CandidateMouse;

const CLASS_NAME: PCWSTR = w!("QingjianCandidateEditor");
const EDIT_ID: i32 = 100;
const OK_ID: i32 = 1;
const CANCEL_ID: i32 = 2;
static CLASS: WindowClass = WindowClass::new();

struct Editor {
    hwnd: HWND,

    edit: HWND,

    offset: usize,
}

thread_local! {
    static EDITOR: RefCell<Option<Editor>> = const { RefCell::new(None) };
}

/// 打开编辑框；同一时刻只保留一个。
pub(super) fn open(offset: usize, current: &str) {
    close();
    if let Err(error) = ensure_class() {
        tracing::warn!(%error, "候选编辑框窗口类注册失败");
        return;
    }
    let current = wide(current);
    let hwnd = unsafe {
        CreateWindowExW(
            WS_EX_TOOLWINDOW,
            CLASS_NAME,
            w!("修改候选译文"),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU,
            0x80000000u32 as i32,
            0x80000000u32 as i32,
            420,
            150,
            None,
            None,
            Some(super::module_handle()),
            Some(&offset as *const usize as *const _),
        )
    };
    let Ok(hwnd) = hwnd else {
        return;
    };
    let edit = unsafe {
        CreateWindowExW(
            Default::default(),
            w!("EDIT"),
            PCWSTR(current.as_ptr()),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | WINDOW_STYLE(ES_AUTOHSCROLL as u32),
            20,
            22,
            365,
            28,
            Some(hwnd),
            Some(HMENU(EDIT_ID as *mut _)),
            Some(super::module_handle()),
            None,
        )
    };
    let Ok(edit) = edit else {
        let _ = unsafe { DestroyWindow(hwnd) };
        return;
    };
    let _ = unsafe {
        CreateWindowExW(
            Default::default(),
            w!("BUTTON"),
            w!("保存译文"),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | WINDOW_STYLE(BS_DEFPUSHBUTTON as u32),
            205,
            65,
            100,
            30,
            Some(hwnd),
            Some(HMENU(OK_ID as *mut _)),
            Some(super::module_handle()),
            None,
        )
    };
    let _ = unsafe {
        CreateWindowExW(
            Default::default(),
            w!("BUTTON"),
            w!("取消"),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | WINDOW_STYLE(BS_PUSHBUTTON as u32),
            315,
            65,
            70,
            30,
            Some(hwnd),
            Some(HMENU(CANCEL_ID as *mut _)),
            Some(super::module_handle()),
            None,
        )
    };
    EDITOR.with(|state| *state.borrow_mut() = Some(Editor { hwnd, edit, offset }));
    let _ = unsafe { ShowWindow(hwnd, SW_SHOW) };
    let _ = unsafe { SetForegroundWindow(hwnd) };
    let _ = unsafe { SetFocus(Some(edit)) };
}

fn ensure_class() -> Result<()> {
    CLASS.ensure(|| WNDCLASSEXW {
        lpfnWndProc: Some(wndproc),
        hInstance: super::module_handle(),
        hCursor: unsafe { LoadCursorW(None, IDC_ARROW) }.unwrap_or_default(),
        lpszClassName: CLASS_NAME,
        style: CS_HREDRAW | CS_VREDRAW,
        ..Default::default()
    })
}

fn submit() {
    let value = EDITOR.with(|state| {
        let state = state.borrow();
        let editor = state.as_ref()?;
        let mut buffer = vec![0u16; 1024];
        let len = unsafe { GetWindowTextW(editor.edit, &mut buffer) } as usize;
        let text = String::from_utf16_lossy(&buffer[..len]).trim().to_owned();
        (!text.is_empty()).then_some((editor.offset, text))
    });
    if let Some((offset, text)) = value {
        click::send(CandidateMouse::TranslationEdited { offset, text });
    }
    close();
}

fn close() {
    let hwnd = EDITOR.with(|state| state.borrow().as_ref().map(|editor| editor.hwnd));
    if let Some(hwnd) = hwnd {
        let _ = unsafe { DestroyWindow(hwnd) };
    }
}

unsafe extern "system" fn wndproc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        WM_CREATE => {
            let create = unsafe { &*(lparam.0 as *const CREATESTRUCTW) };
            let offset = unsafe { *(create.lpCreateParams as *const usize) };
            EDITOR.with(|state| {
                *state.borrow_mut() = Some(Editor {
                    hwnd,
                    edit: HWND::default(),
                    offset,
                });
            });
            LRESULT(0)
        }
        WM_COMMAND => {
            match (wparam.0 & 0xffff) as i32 {
                OK_ID => submit(),
                CANCEL_ID => close(),
                _ => {}
            }
            LRESULT(0)
        }
        WM_CLOSE => {
            close();
            LRESULT(0)
        }
        WM_DESTROY => {
            EDITOR.with(|state| state.borrow_mut().take());
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
    }
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(Some(0)).collect()
}
