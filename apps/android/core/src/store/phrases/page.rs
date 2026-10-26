//! 交给壳的常用语整页。

use qingjian_learning::Phrase;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct PhrasePage {
    /// 分组，「默认」在第一个。
    pub groups: Vec<String>,

    /// 全部常用语，靠前的常用；壳按分组筛。
    pub phrases: Vec<Phrase>,
}
