use serde::{Deserialize, Serialize};

use crate::config::Modifiers;

/// 一次按键按下时的修饰键状态（Windows 语义）。后两位不是物理修饰键，是输入法状态：
/// `caps` 是 Caps Lock 锁定位（管大小写、亮着即临时英文大写），`english_mode` 是 Shift 单击切出来的持久中英模式。
///
/// 普通字符键通常都是 `false`，能干净序列化——不像配置层的 `Modifiers`（走字符串、空值不合法，
/// 那个是给快捷键配置用的）。协议要能表达「没有修饰键」，所以自带这个而不复用它。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct KeyModifiers {
    /// Ctrl
    pub ctrl: bool,

    /// Shift
    pub shift: bool,

    /// Alt
    pub alt: bool,

    /// Win（⊞）
    pub win: bool,

    /// Caps Lock 亮着（锁定状态，不是按着）。管字母大小写；亮着时无论中英文模式都直接出大写英文（微软拼音式）。
    #[serde(default)]
    pub caps: bool,

    /// 持久的英文模式（Windows 单击 Shift 切换，DLL 记状态）。中文模式为 `false`。macOS 壳不用这个字段。
    #[serde(default)]
    pub english_mode: bool,
}

impl KeyModifiers {
    /// 按着的物理修饰键组合（去掉 Caps Lock 与中英模式这两个输入法状态位），拿来与配置的快捷键比。
    pub fn chord(self) -> Self {
        Self {
            caps: false,
            english_mode: false,
            ..self
        }
    }

    /// 有没有按着 Ctrl / Alt / Win（Shift 不算：它只改字符大小写与标点）。
    pub fn has_command_key(self) -> bool {
        self.ctrl || self.alt || self.win
    }
}

/// 配置里的修饰键组合按 macOS 命名（option / command），落到 Windows 键位：⌥ 是 Alt、⌃ 是 Ctrl、⌘ 是 Win。
impl From<Modifiers> for KeyModifiers {
    fn from(modifiers: Modifiers) -> Self {
        Self {
            ctrl: modifiers.control,
            shift: modifiers.shift,
            alt: modifiers.option,
            win: modifiers.command,
            caps: false,
            english_mode: false,
        }
    }
}
