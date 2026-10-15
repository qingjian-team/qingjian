//! 画布的裁剪区：一个矩形（可带圆角）。所有画的操作都只落在里面；嵌套的裁剪取交集。
//! 遮罩要的时候才建：整个落在安全区（矩形扣掉圆角）里的操作不用遮罩，整个在外面的直接跳过。

use std::cell::OnceCell;

use tiny_skia::{FillRule, Mask, Path, Transform};

/// 左、上、右、下（像素）。
pub(super) type Edges = (f32, f32, f32, f32);

pub(super) struct Clip {
    /// 外接的整数像素范围（左、上、右、下，右下不含），逐像素混合的快路径按它裁。
    pub(super) bounds: (i32, i32, i32, i32),

    /// 不用遮罩就能直接画的范围：每一层的矩形扣掉圆角后的交集。
    safe: Edges,

    /// 从外到里每一层的形状，建遮罩时取交集。
    paths: Vec<Path>,

    /// 形状不是对齐像素的矩形：逐像素混合时还要乘遮罩。
    pub(super) shaped: bool,

    size: (u32, u32),

    mask: OnceCell<Option<Mask>>,
}

impl Clip {
    pub(super) fn new(
        parent: Option<&Clip>,
        path: Path,
        bounds: (i32, i32, i32, i32),
        safe: Edges,
        shaped: bool,
        size: (u32, u32),
    ) -> Self {
        let mut paths = parent.map_or_else(Vec::new, |parent| parent.paths.clone());
        paths.push(path);
        let (bounds, safe) = match parent {
            Some(parent) => (
                (
                    bounds.0.max(parent.bounds.0),
                    bounds.1.max(parent.bounds.1),
                    bounds.2.min(parent.bounds.2),
                    bounds.3.min(parent.bounds.3),
                ),
                (
                    safe.0.max(parent.safe.0),
                    safe.1.max(parent.safe.1),
                    safe.2.min(parent.safe.2),
                    safe.3.min(parent.safe.3),
                ),
            ),
            None => (bounds, safe),
        };
        Self {
            bounds,
            safe,
            paths,
            shaped: shaped || parent.is_some_and(|parent| parent.shaped),
            size,
            mask: OnceCell::new(),
        }
    }

    /// 一个操作画到 `edges` 这个范围里要不要遮罩：`None` 整个在裁剪区外（跳过），`Some(false)` 不用，`Some(true)` 要。
    pub(super) fn needs(&self, edges: Edges) -> Option<bool> {
        let (left, top, right, bottom) = edges;
        let b = self.bounds;
        if right <= b.0 as f32 || bottom <= b.1 as f32 || left >= b.2 as f32 || top >= b.3 as f32 {
            return None;
        }
        let s = self.safe;
        Some(!(left >= s.0 && top >= s.1 && right <= s.2 && bottom <= s.3))
    }

    pub(super) fn mask(&self) -> Option<&Mask> {
        self.mask
            .get_or_init(|| {
                let mut mask = Mask::new(self.size.0, self.size.1)?;
                for (index, path) in self.paths.iter().enumerate() {
                    if index == 0 {
                        mask.fill_path(path, FillRule::Winding, true, Transform::identity());
                    } else {
                        mask.intersect_path(path, FillRule::Winding, true, Transform::identity());
                    }
                }
                Some(mask)
            })
            .as_ref()
    }

    pub(super) fn paths(&self) -> &[Path] {
        &self.paths
    }

    /// `(x, y)` 处的遮罩值（0–255）。
    pub(super) fn coverage(&self, x: i32, y: i32) -> u8 {
        self.mask().map_or(255, |mask| {
            mask.data()[y as usize * mask.width() as usize + x as usize]
        })
    }
}
