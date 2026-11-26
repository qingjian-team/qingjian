use serde::{Deserialize, Serialize};

use super::modifiers::KeyModifiers;

/// DLL 从 TSF `OnKeyDown` / `OnTestKeyDown` 抓到的一次按键，发给 Server 判定。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyEvent {
    /// Windows 虚拟键码（`VK_*`）。翻页、方向键、退格、回车等靠它区分。
    pub virtual_key: u32,

    /// 这次按键产生的字符（`ToUnicode` 的结果）；功能键没有字符时为 `None`。
    pub character: Option<char>,

    /// 按下时的修饰键状态。
    pub modifiers: KeyModifiers,

    /// 这一记是不是**按住不放的自动重复**（Windows 是 `lParam` 的 bit 30）。
    ///
    /// 退格用它区分「手一直按着」和「手一下一下点」：前者在组句删空后要停在输入法里，
    /// 后者隔多近都照删。别的键暂时用不到，但协议里一并带上，省得以后再加一次。
    #[serde(default)]
    pub repeat: bool,
}

impl KeyEvent {
    pub fn new(virtual_key: u32, character: Option<char>, modifiers: KeyModifiers) -> Self {
        Self {
            virtual_key,
            character,
            modifiers,
            repeat: false,
        }
    }

    /// 标成一次自动重复。
    pub fn with_repeat(mut self, repeat: bool) -> Self {
        self.repeat = repeat;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeat_defaults_to_false() {
        let event = KeyEvent::new(0x08, None, KeyModifiers::default());
        assert!(!event.repeat, "构造出来默认不是重复");
        assert!(event.with_repeat(true).repeat);
    }

    /// 旧版本的 DLL 不带这个字段，反序列化时该当成 `false`，不能整条消息解不出来。
    #[test]
    fn old_payload_without_repeat_still_parses() {
        let json = r#"{"virtual_key":8,"character":null,"modifiers":{}}"#;
        let event: KeyEvent = serde_json::from_str(json).expect("旧字段该能解");
        assert_eq!(event.virtual_key, 8);
        assert!(!event.repeat);
    }
}
