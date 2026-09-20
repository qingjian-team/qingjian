//! 按键处理：键码 / 字符解析在 [`codes`]，分流在 [`input`]，「修饰键 + 数字」快捷键在 [`shortcut`]，
//! 一次按键的结果是 [`Effect`]。

mod codes;
mod effect;
mod input;
mod shortcut;

pub(super) use self::effect::Effect;

/// 先行上屏的文本与原本透传的字符合并交付，保证应用接收顺序与空格数量。
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

#[cfg(test)]
mod tests;
