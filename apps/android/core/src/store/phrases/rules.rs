//! 常用语编辑框要守的规则，与 `qingjian_learning::PhraseBook` 落盘时的一致。

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct PhraseRules {
    /// 没指定分组时归的那组，总排第一。
    pub default_group: &'static str,

    /// 输入码最长几个字母。
    pub max_code: usize,
}
