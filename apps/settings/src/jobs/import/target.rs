//! 导入的形码词库记给哪套方案。

pub struct CodeTarget {
    /// 方案 id，写进词库文件头。
    pub id: String,

    /// 方案名，导入结果里给人看。
    pub label: String,
}
