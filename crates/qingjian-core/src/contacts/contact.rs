//! 壳交来的一个联系人。

use serde::Deserialize;

/// 一个联系人：名字，与上屏名字后接着给出的内容（电话、邮箱），壳按系统通讯录里的顺序给。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Contact {
    /// 通讯录里显示的名字。
    pub name: String,

    /// 电话、邮箱这些，原样上屏。
    #[serde(default)]
    pub details: Vec<String>,
}
