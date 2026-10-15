//! 舞台：把排好版的场景树画成位图，并留住上一帧做过渡与动画。
//!
//! 使用方建树、调 [`Scene::layout`]（这时可以按布局结果算点击区域），再交给 [`Stage::present`]；
//! 带配对名的节点与上一帧同名节点比位置，变了就从旧位置过渡过去，[`Painted::next_frame`] 告诉使用方多久后调 [`Stage::tick`]。

mod animate;
mod painted;
mod partial;
mod retained;

use std::time::Instant;

use taffy::NodeId;

use crate::canvas::Canvas;
use crate::error::RenderError;
use crate::fonts::FontLibrary;
use crate::scene::Scene;
use crate::text::TextPainter;

pub use painted::Painted;
use retained::Retained;

pub struct Stage {
    /// 文字测绘。
    text: TextPainter,

    /// 上一帧（过渡配对与动画帧重画用）；窗口隐藏后清掉。
    last: Option<Retained>,

    /// 系统开了「减少动态效果」：不播过渡，循环动画停在开头。
    reduce_motion: bool,

    /// 使用方关了动画，效果同上。
    animations_off: bool,

    /// 循环动画的时钟起点：窗口出现后的第一帧；之后的帧不重置，[`Self::forget`] 归零。
    clock: Option<Instant>,
}

impl Stage {
    pub fn new(library: FontLibrary) -> Self {
        Self {
            text: TextPainter::new(library),
            last: None,
            reduce_motion: false,
            animations_off: false,
            clock: None,
        }
    }

    /// 建树、布局、量字都要用它。
    pub fn text(&mut self) -> &mut TextPainter {
        &mut self.text
    }

    /// 关掉动画：过渡直接跳到终点，循环动画停在第一帧。
    pub fn set_animations(&mut self, on: bool) {
        self.animations_off = !on;
    }

    /// 画一帧。`root` 的布局已算好，`content` 是它的宽高（像素）；`scale` 是点 → 像素的倍数，原样记进结果。
    /// `compat` 由使用方定：与上一帧不同（换了排布这类位置不可比的情况）就不配对过渡。
    pub fn present(
        &mut self,
        scene: Scene,
        root: NodeId,
        content: (f32, f32),
        compat: u64,
        scale: f32,
        now: Instant,
    ) -> Result<Painted, RenderError> {
        let keyed = scene.keyed(root)?;
        let transitions = self.transitions(&keyed, compat, scale, now);
        let animated = scene.animated(root)?;
        // 画出范围按起止两头的并算，循环动画按一轮里能到的最远处算：动画中途位图不变大小
        let mut extent = scene.extent(root)?;
        for transition in &transitions {
            let from = transition.from;
            let reach = scene.reach(transition.node).unwrap_or(0.0);
            extent.include((from.x, from.y, from.width, from.height), reach);
        }
        for (node, keyframes) in &animated {
            let parent = scene.parent_origin(*node, root)?;
            let own = scene.subtree_extent(*node, parent.0, parent.1)?;
            let (cx, cy) = scene.center(*node, parent.0, parent.1)?;
            let (dx, dy, grow) = keyframes.reach();
            // 旋转取外接圆，缩放按最大倍数，都绕节点中心
            let (half_w, half_h) = ((own.right - own.left) / 2.0, (own.bottom - own.top) / 2.0);
            let (half_w, half_h) = if keyframes.rotates() {
                let r = (half_w * half_w + half_h * half_h).sqrt();
                (r, r)
            } else {
                (half_w, half_h)
            };
            let (ox, oy) = (
                (own.left + own.right) / 2.0 - cx,
                (own.top + own.bottom) / 2.0 - cy,
            );
            let reach_x = (ox.abs() + half_w) * grow.max(1.0) + dx;
            let reach_y = (oy.abs() + half_h) * grow.max(1.0) + dy;
            extent.include(
                (cx - reach_x, cy - reach_y, reach_x * 2.0, reach_y * 2.0),
                0.0,
            );
        }
        let (x, y) = ((-extent.left).ceil(), (-extent.top).ceil());
        let mut retained = Retained {
            scene,
            root,
            compat,
            scale,
            size: (
                (extent.right + x).ceil() as u32,
                (extent.bottom + y).ceil() as u32,
            ),
            origin: (x, y),
            content: (content.0.ceil(), content.1.ceil()),
            placements: keyed
                .iter()
                .map(|keyed| (keyed.key.clone(), keyed.placement))
                .collect(),
            anchors: keyed
                .iter()
                .map(|keyed| (keyed.key.clone(), keyed.anchor))
                .collect(),
            transitions,
            animated,
            partial: None,
        };
        if self.clock.is_none() {
            self.clock = Some(now);
        }
        let painted = self.paint_retained(&mut retained, now);
        self.last = Some(retained);
        painted
    }

    /// 只画一次、不留帧：按场景画出来的范围开位图（投影、伸出的装饰都在里面），根节点的盒子是内容区。
    pub fn rasterize(
        &mut self,
        scene: &Scene,
        root: NodeId,
        (content_width, content_height): (f32, f32),
        scale: f32,
    ) -> Result<Painted, RenderError> {
        let extent = scene.extent(root)?;
        let (x, y) = ((-extent.left).ceil(), (-extent.top).ceil());
        let width = (extent.right + x).ceil();
        let height = (extent.bottom + y).ceil();
        let mut canvas = Canvas::new(width as u32, height as u32)?;
        scene.paint(root, &mut canvas, &mut self.text, x, y)?;
        Ok(Painted {
            pixmap: canvas.into_pixmap(),
            content_x: x as u32,
            content_y: y as u32,
            content_width: content_width as u32,
            content_height: content_height as u32,
            scale,
            next_frame: None,
        })
    }
}
