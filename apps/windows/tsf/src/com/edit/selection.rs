//! 「翻译选中文字」的读选区会话：Server 收到快捷键后请 DLL 读当前选区，这里在异步只读会话里取文本与屏幕矩形回给 Server。
//! 读到非空选区才置 [`Shared::set_translating`]。

use std::rc::Rc;

use windows::Win32::UI::TextServices::{
    ITfContext, ITfEditSession, ITfEditSession_Impl, ITfRange, TF_ES_READ,
};
use windows::core::{Result, implement};

use super::anchor::{anchor_rect, mouse_screen_rect, selection_range};
use crate::com::composition::Shared;
use crate::com::log::log;
use crate::com::service::SharedClient;

/// 原文字符上限（与 macOS 壳一致），超过就不译，避免整篇误触。
const MAX_TRANSLATE_CHARS: usize = 500;

#[implement(ITfEditSession)]
pub(crate) struct SelectionSession {
    /// 目标文档上下文。
    context: ITfContext,

    /// 引擎层：把选区文本发给 Server。
    engine: SharedClient,

    /// 读到非空选区就置「翻译评审进行中」。
    shared: Rc<Shared>,

    /// 请求标识，回给 Server 对上是哪一次 `RequestSelection`。
    request: u64,
}

impl ITfEditSession_Impl for SelectionSession_Impl {
    fn DoEditSession(&self, ec: u32) -> Result<()> {
        use crate::com::composition::SelectionAnchor;
        use windows::Win32::System::Threading::GetCurrentProcessId;
        use windows::Win32::UI::WindowsAndMessaging::{
            GetForegroundWindow, GetWindowThreadProcessId,
        };
        self.shared.set_selection_anchor(None);
        let range = selection_range(&self.context, ec);
        let rect = range.as_ref().map_or_else(mouse_screen_rect, |range| {
            anchor_rect(&self.context, ec, range)
        });
        let private = range
            .as_ref()
            .is_some_and(|range| super::surrounding::private_input(&self.context, ec, range));
        let text = if private {
            String::new()
        } else {
            range
                .as_ref()
                .map_or_else(String::new, |range| range_text(range, ec))
        };
        crate::com::log::log(&format!(
            "翻译选区：TSF 选区读取完成 units={} private={private}",
            text.encode_utf16().count()
        ));
        if let Ok(mut guard) = self.engine.try_borrow_mut()
            && let Some(client) = guard.as_mut()
        {
            if !text.trim().is_empty() {
                let anchor = range
                    .map(|range| SelectionAnchor::new(self.context.clone(), range, text.clone()));
                match client.selection(self.request, text, rect) {
                    Ok(_) => {
                        self.shared.set_selection_anchor(anchor);
                        self.shared.set_translating(true);
                    }
                    Err(error) => log(&format!("回选区给 Server 失败: {error}")),
                }
            } else {
                let window = unsafe { GetForegroundWindow() };
                let process = unsafe { GetCurrentProcessId() };
                let mut foreground_process = 0;
                unsafe { GetWindowThreadProcessId(window, Some(&mut foreground_process)) };
                if !private && !window.is_invalid() && foreground_process == process {
                    let fallback = super::scintilla::selected_text().unwrap_or_default();
                    crate::com::log::log("翻译选区：TSF 无选区，向 Server 请求备用读取");
                    match client.fallback_selection(
                        self.request,
                        process,
                        window.0 as usize as u64,
                        fallback,
                        rect,
                    ) {
                        Ok(active) => self.shared.set_translating(active),
                        Err(error) => {
                            crate::com::log::log(&format!("翻译选区：备用请求失败: {error}"))
                        }
                    }
                } else {
                    let _ = client.selection(self.request, String::new(), rect);
                }
            }
        }
        Ok(())
    }
}

/// 请求一个异步只读会话读选区。`Ok` 只说明已受理。
pub(crate) fn request_selection(
    context: &ITfContext,
    client_id: u32,
    engine: SharedClient,
    shared: Rc<Shared>,
    request: u64,
) -> Result<()> {
    let session = SelectionSession {
        context: context.clone(),
        engine,
        shared,
        request,
    };
    super::update::request(context, client_id, session.into(), TF_ES_READ)
}

fn range_text(range: &ITfRange, ec: u32) -> String {
    let mut buf = [0u16; MAX_TRANSLATE_CHARS + 1];
    let mut fetched = 0u32;
    if let Err(error) = unsafe { range.GetText(ec, 0, &mut buf, &mut fetched) } {
        log(&format!("翻译选中文字：读取范围文字失败: {error}"));
        return String::new();
    }
    let count = fetched as usize;
    if count == 0 || count > MAX_TRANSLATE_CHARS {
        return String::new();
    }
    String::from_utf16(&buf[..count]).unwrap_or_default()
}
