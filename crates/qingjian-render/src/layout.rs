//! 候选的排布。与配置里的 `LayoutMode` 一一对应，渲染器不依赖配置 crate，由壳换算。

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Layout {
    /// 竖排：一行一个候选，译文在同一行右侧。
    #[default]
    Vertical,

    /// 横排：候选排成一行，只给高亮那个在下面显示译文；帧的 `columns` 不为 0 时展开成多行矩阵。
    Horizontal,
}
