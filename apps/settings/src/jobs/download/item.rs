//! 正在下的是什么：卡片与词库行按它认出自己那一个。

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Item {
    /// 模型仓库 `index.json` 里的 id。
    Model(String),

    /// 词库仓库 `index.json` 里的 id。
    Dict(String),
}
