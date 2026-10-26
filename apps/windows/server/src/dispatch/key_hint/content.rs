//! 键位提示画哪一种键盘。

use qingjian_core::root_chart::RootChart;
use qingjian_core::shuangpin::KeyHint;

#[derive(Debug, Clone, PartialEq)]
pub enum KeyHintContent {
    /// 双拼键位，标着这一键按下后能接哪些韵母。
    Shuangpin(KeyHint),

    /// 五笔字根图（只有 86 版）。
    Roots(&'static RootChart),
}
