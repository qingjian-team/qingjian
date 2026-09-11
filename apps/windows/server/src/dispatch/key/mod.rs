//! 按键处理：键码 / 字符解析在 [`codes`]，分流在 [`input`]，「修饰键 + 数字」快捷键在 [`shortcut`]，
//! 一次按键的结果是 [`Effect`]。

mod codes;
mod effect;
mod input;
mod shortcut;

pub(super) use self::codes::{ESCAPE, RETURN};
pub(super) use self::effect::Effect;

/// 把先行上屏的文本接到本次结果前面。Windows 放行是同步的、上屏走异步编辑会话，
/// 分两步会让应用先插这个键再插词，所以本该放行的键改由我们连同前缀一起插入。
fn with_prefix(prefix: Option<String>, effect: Effect, c: char) -> Effect {
    let Some(mut prefix) = prefix else {
        return effect;
    };
    match effect {
        Effect::Changed(commit) => {
            prefix.push_str(commit.as_deref().unwrap_or_default());
        }
        Effect::Navigated => {}
        Effect::Passthrough => prefix.push(c),
    }
    Effect::Changed(Some(prefix))
}
