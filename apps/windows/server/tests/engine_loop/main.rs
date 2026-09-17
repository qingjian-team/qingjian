//! 不经传输层，直接把协议消息喂给 Router 的闭环测试；用样例词库，跨平台可跑。 按主题分模块，公共零件在 `support`。

mod composing;
mod english;
mod modes;
mod rescoring;
mod shortcuts;
mod status;
mod support;
