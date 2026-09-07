/// preedit（marked text）片段的种类，壳按它选样式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkedKind {
    /// 用户敲的、参与本次候选的拼音（已按音节用 `'` 切开）。
    Typed,

    /// 光标停在中间时，作用域之后剩下的拼音：只显示，不参与候选，画淡一点。
    Rest,

    /// 拼写纠错里被改掉的原字母：画删除线，提示用户我们改了什么。纠错功能接入前不会出现。
    Corrected,
}
