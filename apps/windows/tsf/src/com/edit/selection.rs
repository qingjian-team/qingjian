//! 「翻译选中文字」的读选区会话：Server 收到快捷键后请 DLL 读当前选区，这里在异步只读会话里取文本与屏幕矩形回给 Server。
//! 读到非空选区才置 [`Shared::set_translating`]。

use std::rc::Rc;

use qingjian_platform::protocol::{KeyOutcome, ScreenRect};

use windows::Win32::UI::TextServices::{
    ITfContext, ITfEditSession, ITfEditSession_Impl, ITfRange, TF_ES_READ,
};
use windows::core::{Result, implement};

use super::anchor::{anchor_rect, mouse_screen_rect, selection_range};
use crate::com::composition::{SelectionAnchor, Shared};
use crate::com::log::log;
use crate::com::service::SharedClient;

/// 原文的 UTF-16 单元上限，避免整篇误触。
const MAX_TRANSLATE_UNITS: usize = 500;

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
        let (text, rect, anchor) = match read_selection(&self.context, ec) {
            Some((text, rect, anchor)) => (text, rect, Some(anchor)),
            None => (String::new(), mouse_screen_rect(), None),
        };
        log(&format!(
            "翻译选区：TSF 选区读取完成 units={} valid={}",
            text.encode_utf16().count(),
            anchor.is_some()
        ));
        if reply_selection(&self.engine, self.request, text, rect)
            && let Some(anchor) = anchor
        {
            self.shared.set_selection_anchor(Some(anchor));
            self.shared.set_translating(true);
        }
        Ok(())
    }
}

/// 请求 TSF 异步读选区。没有上下文或会话被拒绝时，以空选区结束请求。
pub(crate) fn request_selection(
    context: Option<&ITfContext>,
    client_id: u32,
    engine: SharedClient,
    shared: Rc<Shared>,
    request: u64,
) {
    shared.set_selection_anchor(None);
    shared.set_translating(false);
    if let Some(context) = context {
        let session = SelectionSession {
            context: context.clone(),
            engine: engine.clone(),
            shared,
            request,
        };
        match super::update::request(context, client_id, session.into(), TF_ES_READ) {
            Ok(()) => return,
            Err(error) => log(&format!("TSF selection edit session rejected: {error}")),
        }
    }
    reply_selection(&engine, request, String::new(), mouse_screen_rect());
}

fn reply_selection(engine: &SharedClient, request: u64, text: String, rect: ScreenRect) -> bool {
    let Ok(mut guard) = engine.try_borrow_mut() else {
        return false;
    };
    let Some(client) = guard.as_mut() else {
        return false;
    };
    match client.selection(request, text, rect) {
        Ok(response) => matches!(response.outcome, KeyOutcome::Consumed),
        Err(error) => {
            log(&format!("回选区给 Server 失败: {error}"));
            false
        }
    }
}

fn read_selection(context: &ITfContext, ec: u32) -> Option<(String, ScreenRect, SelectionAnchor)> {
    let range = selection_range(context, ec)?;
    if super::surrounding::private_input(context, ec, &range) {
        return None;
    }
    let text = range_text(&range, ec)?;
    let rect = anchor_rect(context, ec, &range);
    let anchor = SelectionAnchor::new(context.clone(), range, text.clone());
    Some((text, rect, anchor))
}

fn range_text(range: &ITfRange, ec: u32) -> Option<String> {
    let mut buf = [0u16; MAX_TRANSLATE_UNITS + 1];
    let mut fetched = 0u32;
    if let Err(error) = unsafe { range.GetText(ec, 0, &mut buf, &mut fetched) } {
        log(&format!("翻译选中文字：读取范围文字失败: {error}"));
        return None;
    }
    decode_selection(&buf, fetched as usize)
}

fn decode_selection(buffer: &[u16], fetched: usize) -> Option<String> {
    if fetched > MAX_TRANSLATE_UNITS {
        return None;
    }
    let text = String::from_utf16(buffer.get(..fetched)?).ok()?;
    (!text.trim().is_empty()).then_some(text)
}

#[cfg(test)]
mod tests {
    use super::decode_selection;

    fn decode(text: &str) -> Option<String> {
        let buffer: Vec<u16> = text.encode_utf16().collect();
        decode_selection(&buffer, buffer.len())
    }

    #[test]
    fn selection_limit_counts_utf16_units() {
        assert!(decode(&"中".repeat(500)).is_some());
        assert!(decode(&"中".repeat(501)).is_none());
        assert!(decode(&"😀".repeat(250)).is_some());
        assert!(decode(&"😀".repeat(251)).is_none());
    }

    #[test]
    fn only_nonblank_well_formed_selection_is_returned() {
        assert!(decode("").is_none());
        assert!(decode(" \t\n　").is_none());
        assert!(decode_selection(&[0xd800], 1).is_none());
        assert_eq!(decode(" 原文\n").as_deref(), Some(" 原文\n"));
    }
}
