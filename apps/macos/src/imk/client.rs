//! 对 IMK 客户端对象（实现 `IMKTextInput` 协议的代理）的薄封装。
//!
//! objc2-input-method-kit 没有为 IMKTextInput 生成绑定，这里用 `msg_send!` 直接发消息。

use objc2::msg_send;
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, Bool};
use objc2_foundation::{
    NSAttributedString, NSDictionary, NSNotFound, NSPoint, NSRange, NSRect, NSString,
};
use qingjian_core::SurroundingText;

/// `{NSNotFound, 0}`：不指定替换范围，由客户端使用当前选区或 marked text。
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

    /// IMK 客户端按屏幕坐标判断点击是否落在 marked text 内，保留内部点击行为。
    pub fn point_in_marked_text(&self, point: NSPoint) -> bool {
        let mut inside = Bool::NO;
        // IMKInputSession.h：kIMKNearestBoundaryMode = 1 << 1；非跟踪模式，不改变选区。
        let index: isize = unsafe {
            msg_send![self.object, characterIndexForPoint: point, tracking: 2isize, inMarkedRange: &mut inside]
        };
        index != NSNotFound && inside.as_bool()
    }

    /// 切换模式时仅提交非空组句。空缓冲不能给客户端发文本写入，否则会替换当前选区。
    pub fn commit_pending_raw(&self, raw: &str) {
        if !raw.is_empty() {
            // insertText 已解除 marked text；不能再 setMarkedText 空串，以免重入后误删新选区。
            self.insert_text(raw);
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
        if selected.location == NSNotFound as usize {
            return None;
        }
        if length == 0 {
            return (selected.location == 0 && selected.length == 0).then(|| SurroundingText {
                before: String::new(),
                after: String::new(),
            });
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

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use objc2::rc::Retained;
    use objc2::{AnyThread, DeclaredClass, define_class, msg_send};
    use objc2_foundation::{NSObject, NSObjectProtocol, NSRange, NSString};

    use super::TextClient;

    define_class!(
        // SAFETY: NSObject 允许子类化；仅在测试当前线程访问，不跨线程共享。
        #[unsafe(super(NSObject))]
        #[name = "QingjianTestSelectionClient"]
        #[ivars = RefCell<Vec<String>>]
        struct SelectionClient;

        impl SelectionClient {
            #[unsafe(method(length))]
            fn length(&self) -> usize { 0 }

            #[unsafe(method(selectedRange))]
            fn selected_range(&self) -> NSRange { NSRange::new(0, 0) }

            #[unsafe(method(markedRange))]
            fn marked_range(&self) -> NSRange { NSRange::new(usize::MAX, 0) }

            #[unsafe(method(insertText:replacementRange:))]
            fn insert_text(&self, text: &NSString, _range: NSRange) {
                self.ivars().borrow_mut().push(format!("insert:{}", text));
            }

            #[unsafe(method(setMarkedText:selectionRange:replacementRange:))]
            fn set_marked_text(&self, text: &NSString, _selection: NSRange, _replacement: NSRange) {
                self.ivars().borrow_mut().push(format!("mark:{}", text));
            }
        }

        unsafe impl NSObjectProtocol for SelectionClient {}
    );

    impl SelectionClient {
        fn new() -> Retained<Self> {
            let this = Self::alloc().set_ivars(RefCell::new(Vec::new()));
            unsafe { msg_send![super(this), init] }
        }
    }

    #[test]
    fn confirmed_empty_document_is_context_not_unsupported() {
        let mock = SelectionClient::new();
        let context = TextClient::new(&mock)
            .surrounding_text(4096, 0)
            .expect("空文档有已确认前文");
        assert!(context.before.is_empty());
        assert!(context.after.is_empty());
    }

    #[test]
    fn empty_mode_switch_never_writes_to_selected_application_text() {
        let mock = SelectionClient::new();
        let client = TextClient::new(&mock);
        client.commit_pending_raw("");
        // 任意 insertText 或 setMarkedText 都可能覆盖用户选区；空缓冲必须完全没有写入。
        assert!(mock.ivars().borrow().is_empty());
    }

    #[test]
    fn mode_switch_commits_raw_once_without_followup_marked_text_write() {
        let mock = SelectionClient::new();
        let client = TextClient::new(&mock);
        client.commit_pending_raw("nihao");
        assert_eq!(&*mock.ivars().borrow(), &["insert:nihao"]);
    }
}
