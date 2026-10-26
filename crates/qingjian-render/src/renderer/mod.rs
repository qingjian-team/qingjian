//! 渲染器：一帧 + 排布 + 主题 → 位图。排版与 macOS 壳的 `CandidateView` 一致：顶部拼音行，竖排一行一个候选、横排排成一行。
//!
//! 先按帧和主题模板建一棵场景树（[`Scene`]），布局后算出点击区域，再交给引擎的 [`Stage`] 画、做过渡与动画。
//! 内部全用像素：主题里的点数进来先乘缩放倍数。

mod build;
mod hit;
mod keyboard;
mod menu;
mod rendered;
mod status;

use std::path::PathBuf;
use std::time::Instant;

use bamboo_slip::taffy::NodeId;
use bamboo_slip::{
    Color, FontLibrary, FontSpec, RenderError, Scene, Stage, TextPainter, TextStyle,
};

use crate::frame::Frame;
use crate::layout::Layout;
use crate::theme::Theme;

use build::Builder;

pub use hit::{HitRegion, HitTarget};
pub use rendered::Rendered;
pub use status::{RenderedStatus, StatusCell};

/// 光学字号（点）：20 pt 以下 CoreText 给系统字体用的就是这一档。
const OPTICAL_SIZE: f32 = 17.0;

/// 候选词的文字样式名：状态条之外只有核对字体回退时直接用到。
const CANDIDATE_FONT: &str = "candidate";

pub struct Renderer {
    /// 引擎：画位图、留上一帧做过渡与动画。
    stage: Stage,

    /// 上一帧候选窗的点击区域，动画帧照抄（布局不变）。
    hits: Vec<HitRegion>,
}

impl Renderer {
    pub fn new(library: FontLibrary) -> Self {
        let mut stage = Stage::new(library);
        stage.text().set_optical_size(Some(OPTICAL_SIZE));
        Self {
            stage,
            hits: Vec::new(),
        }
    }

    fn text(&mut self) -> &mut TextPainter {
        self.stage.text()
    }

    /// 加载主题要的字体：随主题带的文件，加上样式里写到、字体库里还没有的系统字族（`family_files` 由壳按平台的字体登记查文件）。
    /// 换主题时调；同一个文件只加载一次。字体没装的样式按回退链往后找，最后用界面字体。
    pub fn load_theme_fonts(
        &mut self,
        theme: &Theme,
        mut family_files: impl FnMut(&str) -> Vec<PathBuf>,
    ) {
        self.text().load_fonts(theme.font_files());
        for family in theme.font_families() {
            if !self.text().has_family(&family) {
                self.text().load_fonts(&family_files(&family));
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
        self.text().use_families(theme.families());
        self.stage.set_animations(theme.animations());
        let mut scene = Scene::new();
        let mut builder = Builder {
            scene: &mut scene,
            theme,
            scale,
            text: self.stage.text(),
            cell_nodes: Vec::new(),
            candidate_nodes: Vec::new(),
            sense_nodes: Vec::new(),
        };
        let root = builder.window(frame, layout)?;
        let (candidate_nodes, sense_nodes) = (builder.candidate_nodes, builder.sense_nodes);
        let content = scene.layout(root, self.stage.text())?;
        self.hits = hit_regions(&scene, root, &candidate_nodes, &sense_nodes)?;
        // 排布变了位置不可比，不与上一帧配对过渡
        let painted = self
            .stage
            .present(scene, root, content, layout as u64, scale, now)?;
        Ok(Rendered::new(painted, self.hits.clone()))
    }

    /// 跟随系统的「减少动态效果」：开着时过渡直接跳到终点。
    pub fn set_reduce_motion(&mut self, reduce: bool) {
        self.stage.set_reduce_motion(reduce);
    }

    /// 忘掉上一帧（窗口隐藏时调）：下次显示不从旧位置过渡，循环动画从头播。
    pub fn forget(&mut self) {
        self.stage.forget();
    }

    /// 有过渡或循环动画在播时，按 `now` 重画一帧（不重建树、不排版）；没有在播的返回 `None`。
    pub fn tick_at(&mut self, now: Instant) -> Result<Option<Rendered>, RenderError> {
        let painted = self.stage.tick_at(now)?;
        Ok(painted.map(|painted| Rendered::new(painted, self.hits.clone())))
    }

    /// 同 [`Self::tick_at`]，用当前时间。
    pub fn tick(&mut self) -> Result<Option<Rendered>, RenderError> {
        self.tick_at(Instant::now())
    }

    /// 量一段文字在 `size` 点字号下的宽度（点），与原生排版的数值对照用。
    pub fn measure_points(&mut self, text: &str, size: f32) -> f32 {
        let style = TextStyle::new(FontSpec::new(size, size), size, Color::rgb(0, 0, 0), 1.0);
        self.text().measure(text, &style).width
    }

    /// 每个字形用到的字族名（按候选词的字号），验证回退链用。
    pub fn trace_families(&mut self, text: &str, theme: &Theme) -> Vec<String> {
        self.text().use_families(theme.families());
        let font = theme.font(CANDIDATE_FONT);
        let style = TextStyle::new(font, font.size, Color::rgb(0, 0, 0), theme.text_gamma());
        self.text().trace_families(text, &style)
    }
}

/// 各候选与各条译词的点击区域（内容区像素）：同一个候选 / 同一条译词的节点取外接矩形，译词排在前面先命中。
fn hit_regions(
    scene: &Scene,
    root: NodeId,
    candidates: &[(usize, NodeId)],
    senses: &[(usize, usize, NodeId)],
) -> Result<Vec<HitRegion>, RenderError> {
    let mut regions: Vec<HitRegion> = Vec::new();
    let mut add = |target: HitTarget, node: NodeId| -> Result<(), RenderError> {
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
