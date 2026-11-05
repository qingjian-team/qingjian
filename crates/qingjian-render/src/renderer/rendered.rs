//! 渲染结果：位图加内容区（阴影边之外的那块）的位置。

use bamboo_slip::{Painted, Pixmap};

use super::{HitRegion, HitTarget, PreeditHit};
use crate::MaterialBacking;

/// 不透明度低于它（约 5%）的投影外圈在浅色底上也几乎看不出，避让时不算。
const VISIBLE_ALPHA: u8 = 12;

pub struct Rendered {
    /// 预乘 RGBA 位图，含阴影边。
    pub pixmap: Pixmap,

    /// 内容区左上角在位图里的像素坐标。
    pub content_x: u32,

    pub content_y: u32,

    /// 内容区像素宽高（窗口该有的大小）。
    pub content_width: u32,

    pub content_height: u32,

    /// 渲染用的倍数，壳把像素换回点用。
    pub scale: f32,

    /// 还有动画在播：隔这么久调 [`crate::Renderer::tick`] 要下一帧；`None` 表示画完了，不用再要。
    pub next_frame: Option<std::time::Duration>,

    /// 候选窗口里可点的区域，译词在前（先命中）；状态条为空，它按 [`crate::RenderedStatus::cell_edges`] 分格。
    pub hits: Vec<HitRegion>,

    /// 壳要在内容区底下垫的系统材质；`None` 是不透明窗口，阴影也在位图里。
    pub material: Option<MaterialBacking>,

    /// 拼音行上各字之间的位置，点拼音行挪光标用；这一帧没画拼音行时为 `None`。
    pub preedit: Option<PreeditHit>,
}

impl Rendered {
    pub(crate) fn new(painted: Painted, hits: Vec<HitRegion>) -> Self {
        Self {
            pixmap: painted.pixmap,
            content_x: painted.content_x,
            content_y: painted.content_y,
            content_width: painted.content_width,
            content_height: painted.content_height,
            scale: painted.scale,
            next_frame: painted.next_frame,
            hits,
            material: None,
            preedit: None,
        }
    }

    pub(crate) fn with_preedit(self, preedit: Option<PreeditHit>) -> Self {
        Self { preedit, ..self }
    }

    pub(crate) fn with_material(self, material: Option<MaterialBacking>) -> Self {
        Self { material, ..self }
    }

    /// 内容区里一点（点，左上为原点）点中了什么。
    pub fn hit_at(&self, x: f32, y: f32) -> Option<HitTarget> {
        let (x, y) = (x * self.scale, y * self.scale);
        self.hits
            .iter()
            .find(|region| region.contains(x, y))
            .map(|region| region.target)
    }

    /// 候选最上沿在位图里的像素纵坐标；没有候选时是位图高。候选条的壳以它为界：上面的拼音行钉住，下面的候选随滚动。
    pub fn candidates_top(&self) -> u32 {
        let top = self
            .hits
            .iter()
            .filter(|region| matches!(region.target, HitTarget::Candidate(_)))
            .map(|region| region.y)
            .fold(f32::INFINITY, f32::min);
        if top.is_finite() {
            self.content_y + top.max(0.0) as u32
        } else {
            self.pixmap.height()
        }
    }

    /// 位图在内容区上、下伸出去、看得见的像素数（投影外圈淡到看不出的那截不算）。
    /// 壳把窗口贴在输入行上下时按它避让：只让开看得见的投影，不按模糊半径的理论范围空出一大段。
    pub fn visible_overhang(&self) -> (u32, u32) {
        let (width, height) = (self.pixmap.width() as usize, self.pixmap.height() as usize);
        let data = self.pixmap.data();
        let visible = |row: usize| {
            data[row * width * 4..(row + 1) * width * 4]
                .chunks_exact(4)
                .any(|pixel| pixel[3] >= VISIBLE_ALPHA)
        };
        let top = (self.content_y as usize).min(height);
        let bottom = (top + self.content_height as usize).min(height);
        let above = (0..top)
            .find(|&row| visible(row))
            .map_or(0, |row| top - row);
        let below = (bottom..height)
            .rev()
            .find(|&row| visible(row))
            .map_or(0, |row| row + 1 - bottom);
        (above as u32, below as u32)
    }

    /// 内容区宽高换回点。
    pub fn content_size_points(&self) -> (f32, f32) {
        (
            self.content_width as f32 / self.scale,
            self.content_height as f32 / self.scale,
        )
    }
}
