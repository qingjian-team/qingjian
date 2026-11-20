//! 对 IMK 客户端对象（实现 `IMKTextInput` 协议的代理）的薄封装。
//!
//! objc2-input-method-kit 没有为 IMKTextInput 生成绑定，这里用 `msg_send!` 直接发消息。

use std::cell::Cell;
use std::time::Instant;

use objc2::msg_send;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_foundation::{NSAttributedString, NSDictionary, NSNotFound, NSRange, NSRect, NSString};
use qingjian_core::SurroundingText;

/// `{NSNotFound, 0}`：不替换任何已有文本，插到当前位置。
const NO_REPLACEMENT: NSRange = NSRange::new(NSNotFound as usize, 0);

thread_local! {
    /// 上次 [`take_inserted`] 以来交给应用的字数，组句结束的日志用（见 `host::CompositionTrace`）。
    static INSERTED: Cell<usize> = const { Cell::new(0) };
}

/// 取走并清零上屏字数。
pub fn take_inserted() -> usize {
    INSERTED.with(|inserted| inserted.replace(0))
}

#[derive(Clone, Copy)]
pub struct TextClient<'a> {
    /// IMK 传进来的 `sender`。
    object: &'a AnyObject,
}

impl<'a> TextClient<'a> {
    pub fn new(object: &'a AnyObject) -> Self {
        Self { object }
    }

    /// 设置 marked text（带下划线的未上屏文本），光标放在第 `cursor` 个字符处。空串等于清除。
    pub fn set_marked_text(&self, text: &str, cursor: usize) {
        let string = NSString::from_str(text);
        let cursor = NSRange::new(cursor.min(text.chars().count()), 0);
        unsafe {
            let _: () = msg_send![
                self.object,
                setMarkedText: &*string,
                selectionRange: cursor,
                replacementRange: NO_REPLACEMENT
            ];
        }
    }

    /// 上屏。
    pub fn insert_text(&self, text: &str) {
        INSERTED.with(|inserted| inserted.set(inserted.get() + text.chars().count()));
        let string = NSString::from_str(text);
        unsafe {
            let _: () =
                msg_send![self.object, insertText: &*string, replacementRange: NO_REPLACEMENT];
        }
    }

    /// 应用里当前选中的文字与它的范围（翻译用）。没有选区、应用不支持读文本、超过 `max_chars` 个字符都返回 `None`。
    pub fn selected_text(&self, max_chars: usize) -> Option<(String, NSRange)> {
        let selected: NSRange = unsafe { msg_send![self.object, selectedRange] };
        if selected.location == NSNotFound as usize
            || selected.length == 0
            || selected.length > max_chars
        {
            return None;
        }
        let text: Option<Retained<NSAttributedString>> =
            unsafe { msg_send![self.object, attributedSubstringFromRange: selected] };
        let text = text?.string().to_string();
        (!text.trim().is_empty()).then_some((text, selected))
    }

    /// 用 `text` 替换应用里 `range` 那段文字（翻译结果替换选区）。
    pub fn replace_range(&self, text: &str, range: NSRange) {
        let string = NSString::from_str(text);
        unsafe {
            let _: () = msg_send![self.object, insertText: &*string, replacementRange: range];
        }
    }

    /// 读光标附近的文本给联想当上下文：marked text 之前 `before` 个字符、之后 `after` 个字符。
    /// 读取失败返回 `None`；debug 只记录范围、长度与失败步骤，不记录正文。
    pub fn surrounding_text(
        &self,
        before: usize,
        after: usize,
        phase: &'static str,
    ) -> Option<SurroundingText> {
        let started = tracing::enabled!(tracing::Level::DEBUG).then(Instant::now);
        let (length, selected, marked): (usize, NSRange, NSRange) = unsafe {
            (
                msg_send![self.object, length],
                msg_send![self.object, selectedRange],
                msg_send![self.object, markedRange],
            )
        };
        let mut before_range = None;
        let mut after_range = None;
        let mut before_status = "not_attempted";
        let mut after_status = "not_attempted";
        let mut before_utf16 = None;
        let mut after_utf16 = None;
        let mut outcome = "ok";
        // 保留原来的早退与短路顺序：诊断不修正范围，也不因前文失败而追加后文读取。
        let result = (|| {
            if selected.location == NSNotFound as usize {
                outcome = "selection_unavailable";
                return None;
            }
            if length == 0 {
                outcome = "zero_document_length";
                return None;
            }
            // 组句中光标在 marked text 里；上下文以 marked text 为界。
            let (start, end) = if marked.location == NSNotFound as usize {
                (selected.location, selected.location + selected.length)
            } else {
                (marked.location, marked.location + marked.length)
            };
            let start = start.min(length);
            let end = end.min(length);
            let preceding = NSRange::new(
                start.saturating_sub(before),
                start - start.saturating_sub(before),
            );
            let following = NSRange::new(end, after.min(length - end));
            before_range = Some(preceding);
            after_range = Some(following);
            let read = |range: NSRange, status: &mut &str, utf16: &mut Option<usize>| {
                if range.length == 0 {
                    *status = "empty_range";
                    *utf16 = Some(0);
                    return Some(String::new());
                }
                let text: Option<Retained<NSAttributedString>> =
                    unsafe { msg_send![self.object, attributedSubstringFromRange: range] };
                let text = text.map(|t| t.string().to_string());
                *status = if text.is_some() { "value" } else { "nil" };
                if started.is_some() {
                    *utf16 = text.as_ref().map(|text| text.encode_utf16().count());
                }
                text
            };
            let Some(before) = read(preceding, &mut before_status, &mut before_utf16) else {
                outcome = "before_nil";
                return None;
            };
            let Some(after) = read(following, &mut after_status, &mut after_utf16) else {
                outcome = "after_nil";
                return None;
            };
            Some(SurroundingText { before, after })
        })();
        if let Some(started) = started {
            let elapsed_us = started.elapsed().as_micros() as u64;
            let app = self.bundle_identifier();
            tracing::debug!(
                app = app.as_deref().unwrap_or("unknown"),
                phase,
                document_length = length,
                length_unknown = length == NSNotFound as usize,
                selected_location = selected.location,
                selected_length = selected.length,
                marked_location = marked.location,
                marked_length = marked.length,
                before_limit = before,
                after_limit = after,
                ?before_range,
                ?after_range,
                before_status,
                after_status,
                ?before_utf16,
                ?after_utf16,
                outcome,
                elapsed_us,
                "surrounding context probe"
            );
            if result.is_none() {
                self.probe_bounded_surrounding_text(
                    (before, after),
                    phase,
                    app.as_deref().unwrap_or("unknown"),
                    length,
                    selected,
                    marked,
                );
            }
        }
        result
    }

    /// 只探测接口能力；不依赖全文长度，结果不进入引擎，也不记录正文。
    fn probe_bounded_surrounding_text(
        &self,
        limits: (usize, usize),
        phase: &'static str,
        app: &str,
        document_length: usize,
        selected: NSRange,
        marked: NSRange,
    ) {
        let range_end = |range: NSRange| {
            range.location.checked_add(range.length).filter(|&end| {
                range.location < NSNotFound as usize
                    && range.length < NSNotFound as usize
                    && end < NSNotFound as usize
            })
        };
        // Chrome 移动光标后可能保留旧的零长度 marked 范围，此时以当前选区为准。
        let (anchor_source, anchor) = if marked.length > 0 && range_end(marked).is_some() {
            ("marked", marked)
        } else {
            ("selection", selected)
        };
        let Some(end) = range_end(anchor) else {
            tracing::debug!(
                app,
                phase,
                document_length,
                anchor_source,
                ?anchor,
                outcome = "invalid_anchor",
                "bounded surrounding context probe"
            );
            return;
        };
        let start = anchor.location;
        let before_start = start.saturating_sub(limits.0.min(64));
        for (side, requested) in [
            ("before", NSRange::new(before_start, start - before_start)),
            ("after", NSRange::new(end, limits.1.min(32))),
        ] {
            let started = Instant::now();
            let requested_end = range_end(requested);
            let mut actual = NSRange::new(NSNotFound as usize, 0);
            let mut returned_utf16 = None;
            let status = if requested_end.is_none() {
                "invalid_request"
            } else if requested.length == 0 {
                "empty_request"
            } else {
                let text: Option<Retained<NSString>> = unsafe {
                    msg_send![self.object, stringFromRange: requested, actualRange: &mut actual]
                };
                // 仅查询 NSString 的长度，不把正文转成 Rust 字符串或写入日志。
                returned_utf16 = text.as_ref().map(|text| unsafe {
                    let length: usize = msg_send![&**text, length];
                    length
                });
                match returned_utf16 {
                    None => "nil",
                    Some(0) => "empty_value",
                    Some(_) => "value",
                }
            };
            let actual_end = range_end(actual);
            let actual_range_valid = actual_end.is_some();
            let length_matches = returned_utf16 == Some(actual.length) && actual_range_valid;
            let within_request = requested_end
                .zip(actual_end)
                .is_some_and(|(want, got)| actual.location >= requested.location && got <= want);
            let touches_anchor = actual_range_valid
                && if side == "before" {
                    actual_end == Some(start)
                } else {
                    actual.location == end
                };
            tracing::debug!(
                app,
                phase,
                document_length,
                anchor_source,
                anchor_location = anchor.location,
                anchor_length = anchor.length,
                side,
                ?requested,
                ?actual,
                ?returned_utf16,
                status,
                actual_range_valid,
                length_matches,
                within_request,
                touches_anchor,
                elapsed_us = started.elapsed().as_micros() as u64,
                "bounded surrounding context probe"
            );
        }
    }

    /// 正在输入的应用的 bundle identifier（`com.apple.Terminal`），按应用改行为用；应用没给返回 `None`。
    pub fn bundle_identifier(&self) -> Option<String> {
        let bundle: Option<Retained<NSString>> =
            unsafe { msg_send![self.object, bundleIdentifier] };
        bundle.map(|b| b.to_string()).filter(|b| !b.is_empty())
    }

    /// 光标（marked text 起点）所在行在屏幕坐标系里的矩形，用来定位候选窗口。
    /// 应用不支持时返回零矩形，窗口就会落在屏幕左下角，至少看得见。
    pub fn caret_rect(&self) -> NSRect {
        let mut rect = NSRect::ZERO;
        unsafe {
            let _: Option<Retained<NSDictionary>> = msg_send![self.object, attributesForCharacterIndex: 0usize, lineHeightRectangle: &mut rect];
        }
        rect
    }
}
