//! 翻译组当前的状态，壳拿去写提示行。

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TranslateStatus {
    /// 框里没字，或者字太少还不值得翻译。
    Idle,

    /// 等停键或等译文回来。
    Waiting,

    /// 译文对得上框里的原文与目标语言，回车就上屏。
    Ready,

    /// 云端没给出译文，或者翻译不可用（云服务关了、私密输入中）。
    Failed,
}
