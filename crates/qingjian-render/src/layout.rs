//! 候选的排布。竖排、横排与配置里的 `LayoutMode` 一一对应，渲染器不依赖配置 crate，由壳换算；候选条只给手机壳用。

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Layout {
    /// 竖排：一行一个候选，译文在同一行右侧。
    #[default]
    Vertical,

    /// 横排：候选排成一行，只给高亮那个在下面显示译文；帧的 `columns` 不为 0 时展开成多行矩阵。
    Horizontal,

    /// 手机候选条：上面一行拼音，下面候选横排、每个候选的译文就在它下面；没有序号与投影。
    /// 位图比视口宽时壳横向滚动，拼音行那一截（[`crate::Rendered::candidates_top`] 以上）不跟着滚。
    Strip,
}
