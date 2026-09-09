use serde::{Deserialize, Serialize};

/// Server 对一次按键的处置：DLL 据此决定 TSF 里 `OnKeyDown` 返回吃掉还是放行。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KeyOutcome {
    /// 输入法消费了这次按键（进了组句缓冲或触发了上屏 / 翻页等）；DLL 吃掉，应用收不到。
    Consumed,

    /// 输入法不处理（如没在组句时的普通字符、快捷键）；DLL 放行给应用。
    Passthrough,
}
