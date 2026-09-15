//! NSEvent 分发与快捷键优先级；输入与候选操作复用控制器。

use super::*;

impl QingjianInputController {
    /// Shift 在松开时识别；普通按键先匹配配置快捷键，再分发命令键与文本。
    pub(super) fn dispatch_event(&self, event: &NSEvent, client: TextClient<'_>) -> bool {
        if event.r#type() == NSEventType::FlagsChanged {
            let tapped = self.ivars().borrow_mut().flags_changed(
                event.keyCode(),
                event.modifierFlags(),
                event.timestamp(),
            );
            if tapped && host::with(|h| h.mode_switch_keys).flatten() == Some(ModeSwitch::Shift) {
                self.switch_mode(client);
                return true;
            }
            return false;
        }
        self.ivars().borrow_mut().cancel();
        if matches!(
            event.r#type(),
            NSEventType::LeftMouseDown | NSEventType::RightMouseDown | NSEventType::OtherMouseDown
        ) {
            // 订阅 flagsChanged 后 IMK 不再提供缺省鼠标收尾；点击先结束组句，事件仍交还应用。
            host::with(|h| h.end_translation());
            self.commit_raw(client);
            return false;
        }
        if event.r#type() != NSEventType::KeyDown {
            return false;
        }
        let flags = event.modifierFlags();
        let (command, control, option, shift) = (
            flags.contains(NSEventModifierFlags::Command),
            flags.contains(NSEventModifierFlags::Control),
            flags.contains(NSEventModifierFlags::Option),
            flags.contains(NSEventModifierFlags::Shift),
        );
        let key = event.keyCode();
        let pressed = Modifiers {
            option,
            shift,
            control,
            command,
        };
        // 提示在显示：敲任何键先收掉，键照常处理
        host::with(|h| h.clear_notice());
        let typed = event
            .charactersIgnoringModifiers()
            .map(|c| c.to_string().to_ascii_lowercase());
        let switching = host::with(|h| h.mode_switch_keys)
            .flatten()
            .and_then(ModeSwitch::combo);
        if switching.is_some_and(|combo| {
            pressed == combo.modifiers && typed.as_deref() == Some(combo.key.to_string().as_str())
        }) {
            // 长按只切一次，但重复事件仍要吞掉，不能漏进应用。
            if !event.isARepeat() {
                self.switch_mode(client);
            }
            return true;
        }
        // 翻译选中文字进行中：回车 / 空格 / 1 接受，Esc 放弃，其他键放弃后照常交给应用
        if host::with(|h| h.translation.is_some()).unwrap_or(false) {
            return self.handle_translation_review(key, client);
        }
        // 翻译快捷键（不在组句中）：读应用里的选区，交给云端
        let combo = host::with(|h| h.translate_keys).unwrap_or_default();
        if pressed == combo.modifiers
            && typed.as_deref().and_then(|t| t.chars().next()) == Some(combo.key)
            && !host::with(|h| !h.engine.composition().is_empty()).unwrap_or(false)
        {
            return self.translate_selection(client);
        }
        // 修饰键 + 数字：按配置的两组组合上屏第一 / 第二个译词（缺省 ⌥ 与 ⇧⌥）、删候选（缺省 ⇧）。
        // 只在组句中认：不在组句时 ⇧4 就是 `$`，得走下面的标点转换（中文模式出 ￥、⇧6 出 ……、⇧1 出 ！），
        // 以前在这里被截走后原样还给应用，全角转换就没机会做了。
        // 表达式模式（`v2^3`）里 ⇧+数字打的是 `^ * ( )`，不当快捷键
        let composing = host::with(|h| !h.engine.composition().is_empty()).unwrap_or(false);
        let expression = composing && host::with(|h| h.engine.expression_mode()).unwrap_or(false);
        if composing
            && !expression
            && !pressed.is_empty()
            && let Some(digit) = digit_key(key)
        {
            let (first, second) = host::with(|h| h.translation_keys).unwrap_or_default();
            if pressed == first {
                return self.handle_translation_key(digit, 0, client);
            }
            if pressed == second {
                return self.handle_translation_key(digit, 1, client);
            }
            if pressed == host::with(|h| h.delete_keys).unwrap_or_default() {
                return self.handle_delete_key(digit, client);
            }
        }
        let selector = match key {
            36 | 76 => Some(sel!(insertNewline:)),
            48 if shift => Some(sel!(insertBacktab:)),
            48 => Some(sel!(insertTab:)),
            51 if option => Some(sel!(deleteWordBackward:)),
            51 if command => Some(sel!(deleteToBeginningOfLine:)),
            51 => Some(sel!(deleteBackward:)),
            117 => Some(sel!(deleteForward:)),
            53 => Some(sel!(cancelOperation:)),
            126 => Some(sel!(moveUp:)),
            125 => Some(sel!(moveDown:)),
            123 if command => Some(sel!(moveToLeftEndOfLine:)),
            124 if command => Some(sel!(moveToRightEndOfLine:)),
            123 if option => Some(sel!(moveWordLeft:)),
            124 if option => Some(sel!(moveWordRight:)),
            123 => Some(sel!(moveLeft:)),
            124 => Some(sel!(moveRight:)),
            116 => Some(sel!(pageUp:)),
            121 => Some(sel!(pageDown:)),
            115 => Some(sel!(moveToBeginningOfLine:)),
            119 => Some(sel!(moveToEndOfLine:)),
            _ => None,
        };
        if let Some(selector) = selector {
            return self.handle_command(selector, client);
        }
        if command || control {
            return false;
        }
        match event.characters() {
            Some(text) if !text.is_empty() => self.handle_text(&text.to_string(), shift, client),
            _ => false,
        }
    }

    /// 两种切换方式共用收尾，未完成输入原样上屏，不丢字。
    fn switch_mode(&self, client: TextClient<'_>) {
        host::with(|h| h.end_translation());
        self.commit_raw(client);
        modifiers::toggle_mode();
        host::with(|h| {
            h.engine.set_english_mode(false);
            h.engine.break_chain();
            h.indicator.update();
        });
    }
}
