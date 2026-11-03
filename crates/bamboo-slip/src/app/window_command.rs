//! 应用要面板窗口做的事。

/// 坐标与大小都是物理像素。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowCommand {
    /// 摆到这个位置并显示、拿焦点；`from` 给了就从那里滑过来。
    Show {
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        from: Option<(i32, i32)>,
    },

    Hide,
}
