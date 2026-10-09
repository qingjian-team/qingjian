//! 带姿态节点的图层按什么算：父节点左上角与整张画布大小（缓存按它们对应，局部重画往小图上画时也不变）。

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct LayerPlace {
    /// 父节点左上角（整张画布像素）。
    pub(crate) parent: (f32, f32),

    /// 整张画布宽高。
    pub(crate) size: (u32, u32),
}
