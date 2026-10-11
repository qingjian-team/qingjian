//! 「翻译选中文字」的读选区会话：Server 收到快捷键后请 DLL 读当前选区，这里在异步只读会话里取文本与屏幕矩形回给 Server。
//! 读到非空选区才置 [`Shared::set_translating`]。

use std::rc::Rc;

use windows::Win32::UI::TextServices::{
    ITfContext, ITfEditSession, ITfEditSession_Impl, ITfRange, TF_ES_READ,
};
use windows::core::{Result, implement};

use qingjian_platform::protocol::ScreenRect;

use super::anchor::{anchor_rect, mouse_screen_rect, selection_range};
use crate::com::composition::Shared;
use crate::com::log::log;
use crate::com::service::SharedClient;

/// Windows 长选区上限；按 Unicode 字符计数，UTF-16 缓冲允许每字两个单元。
const MAX_TRANSLATE_CHARS: usize = 20_000;

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
        let (text, rect) = read_selection(&self.context, ec);
        let has_text = !text.trim().is_empty();
        let anchor = selection_range(&self.context, ec).map(|range| (range, text.clone()));
        // 发成功再置本地评审态，免得卡在无窗口的评审里。
        if let Ok(mut guard) = self.engine.try_borrow_mut()
            && let Some(client) = guard.as_mut()
        {
            match client.selection(self.request, text, rect) {
                Ok(_) if has_text => {
                    self.shared.set_translation_anchor(anchor);
                    self.shared.set_translating(true);
                }
                Ok(_) => {}
                Err(error) => log(&format!("回选区给 Server 失败: {error}")),
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

/// 没有选区 / 读不到 / 过长时文本为空串（矩形退到鼠标处）。
fn read_selection(context: &ITfContext, ec: u32) -> (String, ScreenRect) {
    let Some(range) = selection_range(context, ec) else {
        return (String::new(), mouse_screen_rect());
    };
    let rect = anchor_rect(context, ec, &range);
    (range_text(&range, ec), rect)
}

pub(super) fn range_text(range: &ITfRange, ec: u32) -> String {
    let mut buf = vec![0u16; MAX_TRANSLATE_CHARS * 2 + 1];
    let mut fetched = 0u32;
    if unsafe { range.GetText(ec, 0, &mut buf, &mut fetched) }.is_err() {
        return String::new();
    }
    let count = fetched as usize;
    if count == 0 || count > MAX_TRANSLATE_CHARS * 2 {
        return String::new();
    }
    let Ok(text) = String::from_utf16(&buf[..count]) else {
        return String::new();
    };
    if text.chars().count() > MAX_TRANSLATE_CHARS {
        return String::new();
    }
    text
}
