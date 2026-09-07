//! 对 IMK 客户端对象（实现 `IMKTextInput` 协议的代理）的薄封装。
//!
//! objc2-input-method-kit 没有为 IMKTextInput 生成绑定，这里用 `msg_send!` 直接发消息。

use objc2::msg_send;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_foundation::{NSAttributedString, NSDictionary, NSNotFound, NSRange, NSRect, NSString};
use qingjian_core::SurroundingText;

/// `{NSNotFound, 0}`：不替换任何已有文本，插到当前位置。
const NO_REPLACEMENT: NSRange = NSRange::new(NSNotFound as usize, 0);

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
    /// 应用不支持 `attributedSubstringFromRange:`（不少 Electron / 终端）时返回 `None`，由 Core 退回本地历史。
    pub fn surrounding_text(&self, before: usize, after: usize) -> Option<SurroundingText> {
        let (length, selected, marked): (usize, NSRange, NSRange) = unsafe {
            (
                msg_send![self.object, length],
                msg_send![self.object, selectedRange],
                msg_send![self.object, markedRange],
            )
        };
        if selected.location == NSNotFound as usize || length == 0 {
            return None;
        }
        // 组句中光标在 marked text 里；上下文以 marked text 为界
        let (start, end) = if marked.location == NSNotFound as usize {
            (selected.location, selected.location + selected.length)
        } else {
            (marked.location, marked.location + marked.length)
        };
        let start = start.min(length);
        let end = end.min(length);
        let before_range = NSRange::new(
            start.saturating_sub(before),
            start - start.saturating_sub(before),
        );
        let after_range = NSRange::new(end, after.min(length - end));
        let read = |range: NSRange| -> Option<String> {
            if range.length == 0 {
                return Some(String::new());
            }
            let text: Option<Retained<NSAttributedString>> =
                unsafe { msg_send![self.object, attributedSubstringFromRange: range] };
            text.map(|t| t.string().to_string())
        };
        Some(SurroundingText {
            before: read(before_range)?,
            after: read(after_range)?,
        })
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
