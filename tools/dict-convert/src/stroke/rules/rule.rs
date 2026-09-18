//! 大陆笔顺覆盖规则：模式、替换与说明。

use std::collections::HashSet;

use crate::stroke::rules::anchor::Anchor;

/// 一条部件重写规则：CNS（台湾序）里的 `from` 模式就是大陆的 `to`。
pub(super) struct Rule {
    /// 部件名（覆盖表里的标识，报错与日志用）。
    pub(super) name: String,

    /// CNS 序列里的部件笔画模式。
    pub(super) from: String,

    /// 大陆规范里的部件笔画模式。
    pub(super) to: String,

    /// 匹配位置。
    pub(super) anchor: Anchor,

    /// 例外字：这些字里命中的模式不是该部件，规则不适用。
    pub(super) skips: HashSet<char>,
}
