//! 系统界面里的候选窗画法：轻量 GDI 自绘，不链渲染库。
//!
//! Server 自己的候选窗缺省走 `qingjian-render` 出位图（可套用户主题皮肤）。这条链路不适合放进 TSF DLL：
//! DLL 会被加载进每个应用进程，多链一个渲染库就多一份体积，而 TSF 进程里本来也读不到用户主题目录
//! （AppContainer 宿主尤其读不到），皮肤根本画不出来。这里改成 Server 里那条 `renderer = "system"`
//! 的 GDI 画法的精简版：只用系统字族与内置配色，外观与 Server 窗口不必一致，选词、翻页、高亮、
//! 矩阵这些行为一致。
//!
//! 一次绘制的链路：
//! 协议帧 →（[`View`] 换算）→ 量尺寸（[`preferred_size`]）→ 铺底 + 落笔（[`draw`]）→
//! 贴给分层窗（[`canvas`]）。落笔同时记下命中区，点击经
//! [`CandidateHit`](qingjian_platform::protocol::CandidateHit) 回 Server，选词仍走 Server 那条链路。

pub(crate) mod canvas;
mod draw;
mod row;
mod theme;
mod view;

pub(crate) use canvas::Canvas;
pub(crate) use draw::{paint, preferred_size};
pub(crate) use theme::Theme;
pub(crate) use view::{HitRegion, HitTarget, View};
