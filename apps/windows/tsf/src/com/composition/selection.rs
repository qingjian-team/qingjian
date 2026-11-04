//! 译文回填前复核同一文档、焦点、范围和原文；拒绝把迟到译文写到别处。

use windows::Win32::Foundation::HWND;
use windows::Win32::System::Threading::GetCurrentProcessId;
use windows::Win32::UI::Input::KeyboardAndMouse::GetFocus;
use windows::Win32::UI::TextServices::{ITfContext, ITfRange, TF_ANCHOR_END, TF_ANCHOR_START};
use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};

pub(crate) struct SelectionAnchor {
    context: ITfContext,

    range: ITfRange,

    text: String,

    focus: HWND,

    foreground: HWND,
}

impl SelectionAnchor {
    pub(crate) fn new(context: ITfContext, range: ITfRange, text: String) -> Self {
        Self {
            context,
            range,
            text,
            focus: unsafe { GetFocus() },
            foreground: unsafe { GetForegroundWindow() },
        }
    }

    pub(super) fn verified_range(self, context: &ITfContext, ec: u32) -> Option<ITfRange> {
        let mut process = 0;
        unsafe { GetWindowThreadProcessId(self.foreground, Some(&mut process)) };
        if self.context != *context
            || self.focus.is_invalid()
            || unsafe { GetFocus() } != self.focus
            || self.foreground.is_invalid()
            || unsafe { GetForegroundWindow() } != self.foreground
            || process != unsafe { GetCurrentProcessId() }
        {
            return None;
        }
        let current = super::super::edit::selected_range(context, ec)?;
        if unsafe { self.range.CompareStart(ec, &current, TF_ANCHOR_START) }.ok() != Some(0)
            || unsafe { self.range.CompareEnd(ec, &current, TF_ANCHOR_END) }.ok() != Some(0)
        {
            return None;
        }
        let mut buffer = [0u16; 501];
        let mut fetched = 0;
        unsafe { self.range.GetText(ec, 0, &mut buffer, &mut fetched) }.ok()?;
        let text = String::from_utf16(buffer.get(..fetched as usize)?).ok()?;
        (text == self.text).then_some(self.range)
    }
}
