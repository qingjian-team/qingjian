//! macOS 物理按键映射到 Rime keysym；原生处理器决定组合键与中英切换。
use super::{QingjianInputController, TextClient, host, secure_input};
use objc2_app_kit::{NSEvent, NSEventType};
use qingjian_platform::protocol::KeyEvent;

impl QingjianInputController {
    pub(super) fn dispatch_rime_event(&self, event: &NSEvent, client: TextClient<'_>) -> bool {
        if secure_input::enabled() {
            host::with(|h| {
                h.engine.clear();
                h.cancel_prediction();
                h.window.hide();
            });
            client.set_marked_text("", 0);
            return false;
        }
        let kind = event.r#type();
        if !matches!(
            kind,
            NSEventType::KeyDown | NSEventType::KeyUp | NSEventType::FlagsChanged
        ) {
            return false;
        }
        self.note_application(&client);
        // AppKit 的字符接口只用于 KeyDown/KeyUp，FlagsChanged 没有字符。
        let character = if kind == NSEventType::FlagsChanged {
            None
        } else {
            event
                .charactersIgnoringModifiers()
                .and_then(|s| s.to_string().chars().next())
        };
        let Some(native) = KeyEvent::from_macos_rime(
            event.keyCode(),
            character,
            event.modifierFlags().bits() as u64,
            kind == NSEventType::KeyUp,
            kind == NSEventType::FlagsChanged,
        ) else {
            return false;
        };
        let (key, mask) = native.rime_key();
        let result = host::with(|h| h.engine.process_rime_key(key, mask)).flatten();
        let Some((consumed, commit)) = result else {
            return false;
        };
        if let Some(text) = &commit {
            client.insert_text(text);
        }
        self.refresh(client);
        consumed
    }
}
