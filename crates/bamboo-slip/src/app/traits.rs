//! 使用方的应用：状态只在 `update` 里改，界面每次都由 `view` 按当前状态整棵建出来。

use taffy::NodeId;

use super::Waker;
use crate::error::RenderError;
use crate::text::TextPainter;
use crate::ui::{Key, Ui};

pub trait App {
    /// 控件产出、`update` 处理的消息。
    type Message: Clone + 'static;

    /// 建整窗的场景树；根节点就是窗口大小（`size`，像素），`scale` 是点 → 像素的倍数。
    fn view(
        &self,
        ui: &mut Ui<Self::Message>,
        scale: f32,
        size: (u32, u32),
    ) -> Result<NodeId, RenderError>;

    /// 处理一条消息；返回 `false` 表示要退出。
    fn update(&mut self, message: Self::Message) -> bool;

    /// 第一帧之前调一次：加载字体、换字族表这类文字画笔的设置。
    fn setup(&mut self, text: &mut TextPainter) {
        let _ = text;
    }

    /// 开窗后调一次，交给应用一个能从别的线程叫醒窗口的 [`Waker`]；不开窗口时不调。
    fn set_waker(&mut self, waker: Waker) {
        let _ = waker;
    }

    /// 有人调了 [`Waker::wake`]：看看后台的结果，返回要不要重建界面。
    fn wake(&mut self) -> bool {
        false
    }

    /// 要醒目标出、并滚进视野的那一项（搜索跳过去的设置行）；每次变成新的 key 时滚一次。
    fn spotlight(&self) -> Option<Key> {
        None
    }

    /// 窗口进出前台。用户切到别的应用改了文件再切回来时，可以借这个时机重新读盘。
    fn set_active(&mut self, active: bool) {
        let _ = active;
    }

    /// 系统的深浅色变了（开窗时也调一次）。
    fn set_system_dark(&mut self, dark: bool) {
        let _ = dark;
    }
}
