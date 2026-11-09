//! 找到的一套双拼方案：键位已经读好、校验过。

use std::sync::Arc;

use qingjian_core::ShuangpinLayout;

use crate::SchemeMeta;

/// 一套可用的双拼方案。键位表很小，列表时就整个读进来。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShuangpinSchemeEntry {
    /// 配置里 `[general] scheme` 写的值，也是方案的目录名。
    pub id: String,

    pub label: String,

    pub meta: SchemeMeta,

    pub layout: Arc<ShuangpinLayout>,
}
