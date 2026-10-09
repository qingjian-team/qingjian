//! 渲染器：一帧 + 排布 + 主题 → 位图。排版与 macOS 壳的 `CandidateView` 一致：顶部拼音行，竖排一行一个候选、横排排成一行。
//!
//! 先按帧建一棵场景树（[`Scene`]），Taffy 算布局，再按树序画。内部全用像素：主题里的点数进来先乘缩放倍数。
//! 文字节点的盒子顶边就是行框顶边，字形在行高里垂直居中。

mod animate;
mod build;
mod hit;
mod partial;
mod rendered;
mod retained;
mod status;

use std::path::PathBuf;
use std::time::Instant;

use crate::canvas::Canvas;
use crate::color::Color;
use crate::error::RenderError;
use crate::fonts::FontLibrary;
use crate::frame::Frame;
use crate::layout::Layout;
use crate::scene::Scene;
use crate::text::{TextPainter, TextStyle};
use crate::theme::{FontSpec, Theme};

use build::Builder;
use retained::Retained;

pub use hit::{HitRegion, HitTarget};
pub use rendered::Rendered;
pub use status::{RenderedStatus, StatusCell};

/// 光学字号（点）：20 pt 以下 CoreText 给系统字体用的就是这一档。
const OPTICAL_SIZE: f32 = 17.0;

/// 候选词的文字样式名：状态条之外只有核对字体回退时直接用到。
const CANDIDATE_FONT: &str = "candidate";

pub struct Renderer {
    /// 文字测绘。
    text: TextPainter,

    /// 上一帧候选窗（过渡配对与动画帧重画用）；窗口隐藏后清掉。
    last: Option<Retained>,

    /// 系统开了「减少动态效果」：不播过渡，循环动画停在开头。
    reduce_motion: bool,

    /// 最近一帧的主题关了动画（设置里的开关），效果同上。
    animations_off: bool,

    /// 循环动画的时钟起点：窗口出现后的第一帧；打字过程中不重置，窗口收起（`forget`）归零。
    clock: Option<Instant>,
}

impl Renderer {
    pub fn new(library: FontLibrary) -> Self {
        let mut text = TextPainter::new(library);
        text.set_optical_size(Some(OPTICAL_SIZE));
        Self {
            text,
            last: None,
            reduce_motion: false,
            animations_off: false,
            clock: None,
        }
    }

    /// 加载主题要的字体：随主题带的文件，加上样式里写到、字体库里还没有的系统字族（`family_files` 由壳按平台的字体登记查文件）。
    /// 换主题时调；同一个文件只加载一次。字体没装的样式按回退链往后找，最后用界面字体。
    pub fn load_theme_fonts(
        &mut self,
        theme: &Theme,
        mut family_files: impl FnMut(&str) -> Vec<PathBuf>,
    ) {
        self.text.load_fonts(theme.font_files());
        for family in theme.font_families() {
            if !self.text.has_family(&family) {
                self.text.load_fonts(&family_files(&family));
            }
        }
    }

    /// 画一帧。`scale` 是点 → 像素的倍数（Retina 为 2）；位图按画出范围开（投影、伸出的装饰），根节点的盒子是内容区。
    /// 主题里带过渡的节点与上一帧配对，位置变了就从旧位置出发，[`Rendered::next_frame`] 告诉壳多久后要下一帧。
    pub fn render(
        &mut self,
        frame: &Frame,
        layout: Layout,
        theme: &Theme,
        scale: f32,
    ) -> Result<Rendered, RenderError> {
        self.render_at(frame, layout, theme, scale, Instant::now())
    }

    /// 同 [`Self::render`]，时间由调用方给（测试用固定时刻）。
    pub fn render_at(
        &mut self,
        frame: &Frame,
        layout: Layout,
        theme: &Theme,
        scale: f32,
        now: Instant,
    ) -> Result<Rendered, RenderError> {
        self.text.use_families(theme.families());
        self.animations_off = !theme.animations();
        let mut scene = Scene::new();
        let mut builder = Builder {
            scene: &mut scene,
            theme,
            scale,
            text: &mut self.text,
            cell_nodes: Vec::new(),
            candidate_nodes: Vec::new(),
            sense_nodes: Vec::new(),
        };
        let root = builder.window(frame, layout)?;
        let (candidate_nodes, sense_nodes) = (builder.candidate_nodes, builder.sense_nodes);
        let (content_width, content_height) = scene.layout(root, &mut self.text)?;
        let hits = hit_regions(&scene, root, &candidate_nodes, &sense_nodes)?;
        let keyed = scene.keyed(root)?;
        let transitions = self.transitions(&keyed, layout, scale, now);
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
            layout,
            scale,
            size: (
                (extent.right + x).ceil() as u32,
                (extent.bottom + y).ceil() as u32,
            ),
            origin: (x, y),
            content: (content_width.ceil(), content_height.ceil()),
            placements: keyed
                .iter()
                .map(|keyed| (keyed.key.clone(), keyed.placement))
                .collect(),
            transitions,
            animated,
            partial: None,
            hits,
        };
        if self.clock.is_none() {
            self.clock = Some(now);
        }
        let rendered = self.paint_retained(&mut retained, now);
        self.last = Some(retained);
        rendered
    }

    /// 按场景画出来的范围开位图（投影、伸出窗口的装饰都在里面），根节点的盒子是内容区。
    fn rasterize(
        &mut self,
        scene: &Scene,
        root: taffy::NodeId,
        (content_width, content_height): (f32, f32),
        scale: f32,
    ) -> Result<Rendered, RenderError> {
        let extent = scene.extent(root)?;
        let (x, y) = ((-extent.left).ceil(), (-extent.top).ceil());
        let width = (extent.right + x).ceil();
        let height = (extent.bottom + y).ceil();
        let mut canvas = Canvas::new(width as u32, height as u32)?;
        scene.paint(root, &mut canvas, &mut self.text, x, y)?;
        Ok(Rendered {
            pixmap: canvas.into_pixmap(),
            content_x: x as u32,
            content_y: y as u32,
            content_width: content_width as u32,
            content_height: content_height as u32,
            scale,
            next_frame: None,
            hits: Vec::new(),
        })
    }

    /// 量一段文字在 `size` 点字号下的宽度（点），与原生排版的数值对照用。
    pub fn measure_points(&mut self, text: &str, size: f32) -> f32 {
        let style = TextStyle::new(FontSpec::new(size, size), size, Color::rgb(0, 0, 0), 1.0);
        self.text.measure(text, &style).width
    }

    /// 每个字形用到的字族名（按候选词的字号），验证回退链用。
    pub fn trace_families(&mut self, text: &str, theme: &Theme) -> Vec<String> {
        self.text.use_families(theme.families());
        let font = theme.font(CANDIDATE_FONT);
        let style = TextStyle::new(font, font.size, Color::rgb(0, 0, 0), theme.text_gamma());
        self.text.trace_families(text, &style)
    }
}

/// 各候选与各条译词的点击区域（内容区像素）：同一个候选 / 同一条译词的节点取外接矩形，译词排在前面先命中。
fn hit_regions(
    scene: &Scene,
    root: taffy::NodeId,
    candidates: &[(usize, taffy::NodeId)],
    senses: &[(usize, usize, taffy::NodeId)],
) -> Result<Vec<HitRegion>, RenderError> {
    let mut regions: Vec<HitRegion> = Vec::new();
    let mut add = |target: HitTarget, node: taffy::NodeId| -> Result<(), RenderError> {
        let rect = scene.rect_in(node, root)?;
        match regions.iter_mut().find(|region| region.target == target) {
            Some(region) => *region = region.union(rect),
            None => regions.push(HitRegion {
                x: rect.0,
                y: rect.1,
                width: rect.2,
                height: rect.3,
                target,
            }),
        }
        Ok(())
    };
    for &(candidate, sense, node) in senses {
        add(HitTarget::Translation { candidate, sense }, node)?;
    }
    for &(candidate, node) in candidates {
        add(HitTarget::Candidate(candidate), node)?;
    }
    Ok(regions)
}
