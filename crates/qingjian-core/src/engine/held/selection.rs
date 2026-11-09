use crate::composition::Composition;

/// 一段拼音分几次选完时，前面选好、还没交给应用的一次选择。
#[derive(Debug, Clone)]
pub(in crate::engine) struct HeldSelection {
    /// 交给应用的文本（含自动空格补的前导空格）。
    pub text: String,

    /// 这次选择吃掉的那段拼音，撤回时原样接回缓冲区开头。
    pub keys: Composition,
}
