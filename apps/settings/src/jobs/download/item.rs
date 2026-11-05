//! 正在下的是什么：卡片与词库行按它认出自己那一个。

use qingjian_platform::LocalModel;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Item {
    Model(LocalModel),

    /// 词库仓库 `index.json` 里的 id。
    Dict(String),
}
