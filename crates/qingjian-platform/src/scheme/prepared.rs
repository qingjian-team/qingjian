//! 后台读好的方案；应用时不再访问文件系统。

use qingjian_core::CodeScheme;

use super::SchemeSlot;
use crate::Scheme;

/// 方案描述与码表的加载结果，可以跨线程交给输入引擎。
pub struct PreparedScheme {
    /// 加载成功的方案身份与文件指纹。
    pub(super) slot: SchemeSlot,

    /// 配置选择的拼音侧方案。
    pub(super) pinyin: Scheme,

    /// 新读的码表；身份存在但这里为空时，沿用引擎已装的码表。
    pub(super) code: Option<CodeScheme>,
}

impl PreparedScheme {
    /// 配置选择的拼音侧方案。
    pub fn pinyin(&self) -> Scheme {
        self.pinyin
    }
}
