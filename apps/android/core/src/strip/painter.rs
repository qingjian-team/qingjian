//! 候选条的画笔：记住最新一帧要显示的内容，壳要画时按宽度倍数与深浅色出一张位图。

use qingjian_render::{
    FontLibrary, Frame as RenderFrame, HitTarget, Layout, Mode, Rendered, Renderer, StatusCell,
    Theme,
};

use super::{StripHit, StripMeta, StripPage, has_emoji, preedit, rows};
use crate::appearance;
use crate::error::AndroidError;
use crate::frame::Frame;
use crate::session::PhysicalPage;

/// 中日同形字按简体选字形；学习语言是日语时译文里的汉字也用简体字形，与桌面端一致。
const LOCALE: &str = "zh-CN";

pub struct StripPainter {
    renderer: Renderer,

    light: Theme,

    dark: Theme,

    /// 字号档：候选字与译文一起的倍数，1 是主题自己的字号。
    text_scale: f32,

    /// 最新一帧要画的内容。
    content: RenderFrame,

    /// `content` 里有 emoji 的候选下标。
    emoji: Vec<usize>,

    /// 上次画的位图，等壳来拷。
    last: Option<Rendered>,
}

impl StripPainter {
    /// 加载字体（只按清单读几个系统字体文件，不扫目录）；在装配 Engine 的后台线程里建。
    pub fn new() -> Result<Self, AndroidError> {
        let renderer = Renderer::new(FontLibrary::system(LOCALE)?);
        // 高亮移动的过渡要壳按帧回调，候选条先不播
        Ok(Self {
            renderer,
            light: Theme::light().with_animations(false),
            dark: Theme::dark().with_animations(false),
            text_scale: 1.0,
            content: RenderFrame::default(),
            emoji: Vec::new(),
            last: None,
        })
    }

    /// 换主题（内置主题 id，没有的退回缺省）与字号档；下一帧起生效。
    pub fn set_appearance(&mut self, theme: &str, text_scale: f32) {
        let sized = |dark: bool| {
            let theme = appearance::theme(theme, dark);
            theme
                .with_text_sizes(appearance::text_sizes(&theme, text_scale))
                .with_animations(false)
        };
        self.light = sized(false);
        self.dark = sized(true);
        self.text_scale = text_scale;
    }

    /// 记下一帧要显示的拼音、提示与候选。
    pub fn show(&mut self, frame: &Frame) {
        self.emoji = frame
            .candidates
            .iter()
            .enumerate()
            .filter(|(_, candidate)| has_emoji(&candidate.text))
            .map(|(i, _)| i)
            .collect();
        self.content = RenderFrame {
            preedit: (!frame.preedit.is_empty()).then(|| preedit(frame)),
            rows: rows(&frame.candidates, self.text_scale, |text, size| {
                self.renderer.measure_points(text, size)
            }),
            highlighted: None,
            footer: None,
            previous_page: false,
            next_page: false,
            sentence: None,
            status: (!frame.notice.is_empty()).then(|| frame.notice.clone()),
            mode: Mode::default(),
            columns: 0,
            column_ems: Vec::new(),
            reversed: false,
        };
    }

    /// 画最新一帧。`scale` 是 dp → 像素的倍数（`displayMetrics.density`）。
    pub fn paint(
        &mut self,
        highlighted: Option<usize>,
        scale: f32,
        dark: bool,
    ) -> Result<StripMeta, AndroidError> {
        self.paint_layout(highlighted, scale, dark, None)
    }

    /// 物理键盘沿用桌面竖排主题，数字与高亮均为当前页下标。
    pub fn paint_physical(
        &mut self,
        page: &PhysicalPage,
        scale: f32,
        dark: bool,
    ) -> Result<StripMeta, AndroidError> {
        self.paint_layout(Some(page.highlighted()), scale, dark, Some(page))
    }

    /// 临时中英提示复用桌面状态条的单格与主题，不改当前候选帧。
    pub fn paint_mode(
        &mut self,
        label: &str,
        english: bool,
        scale: f32,
        dark: bool,
    ) -> Result<StripMeta, AndroidError> {
        let theme = if dark { &self.dark } else { &self.light };
        let mode = Mode {
            english,
            ..Mode::default()
        };
        let rendered = self
            .renderer
            .render_status(&[StatusCell::text(label, true)], &mode, theme, scale)?
            .rendered;
        let meta = StripMeta {
            width: rendered.pixmap.width(),
            height: rendered.pixmap.height(),
            content_x: rendered.content_x,
            content_y: rendered.content_y,
            content_width: rendered.content_width,
            content_height: rendered.content_height,
            candidates_top: 0,
            background: theme.variable("surface").map_or(0, appearance::argb),
            hits: Vec::new(),
            pages: Vec::new(),
            preedit_edges: Vec::new(),
            preedit_top: 0.0,
            preedit_bottom: 0.0,
            emoji: Vec::new(),
        };
        self.last = Some(rendered);
        Ok(meta)
    }

    fn paint_layout(
        &mut self,
        highlighted: Option<usize>,
        scale: f32,
        dark: bool,
        page: Option<&PhysicalPage>,
    ) -> Result<StripMeta, AndroidError> {
        let mut content = self.content.clone();
        let start = page.map_or(0, PhysicalPage::start);
        if let Some(page) = page {
            let count = content.rows.len();
            content.rows = content
                .rows
                .into_iter()
                .skip(start)
                .take(PhysicalPage::SIZE)
                .enumerate()
                .map(|(i, mut row)| {
                    row.index = (i + 1).to_string();
                    row
                })
                .collect();
            content.previous_page = page.previous();
            content.next_page = page.next();
            content.footer = (!content.rows.is_empty()).then(|| {
                format!(
                    "{}/{}",
                    start / PhysicalPage::SIZE + 1,
                    count.div_ceil(PhysicalPage::SIZE).max(1)
                )
            });
        }
        content.highlighted = highlighted.filter(|&i| i < content.rows.len());
        let theme = if dark { &self.dark } else { &self.light };
        let rendered = self.renderer.render(
            &content,
            if page.is_some() {
                Layout::Vertical
            } else {
                Layout::Strip
            },
            theme,
            scale,
        )?;
        let offset = (rendered.content_x, rendered.content_y);
        let background = theme.variable("surface").map_or(0, appearance::argb);
        let meta = StripMeta {
            width: rendered.pixmap.width(),
            height: rendered.pixmap.height(),
            content_x: rendered.content_x,
            content_y: rendered.content_y,
            content_width: rendered.content_width,
            content_height: rendered.content_height,
            candidates_top: rendered.candidates_top(),
            background,
            hits: rendered
                .hits
                .iter()
                .filter_map(|region| StripHit::new(region, offset))
                .map(|mut hit| {
                    hit.candidate += start;
                    hit
                })
                .collect(),
            pages: rendered
                .hits
                .iter()
                .filter_map(|region| {
                    let HitTarget::Page(step) = region.target else {
                        return None;
                    };
                    Some(StripPage {
                        step,
                        x: region.x + offset.0 as f32,
                        y: region.y + offset.1 as f32,
                        width: region.width,
                        height: region.height,
                    })
                })
                .collect(),
            preedit_edges: rendered.preedit.as_ref().map_or_else(Vec::new, |hit| {
                hit.edges.iter().map(|x| x + offset.0 as f32).collect()
            }),
            preedit_top: rendered
                .preedit
                .as_ref()
                .map_or(0.0, |hit| hit.y + offset.1 as f32),
            preedit_bottom: rendered
                .preedit
                .as_ref()
                .map_or(0.0, |hit| hit.y + hit.height + offset.1 as f32),
            emoji: self.emoji.clone(),
        };
        self.last = Some(rendered);
        Ok(meta)
    }

    /// 把上次画的像素拷进 `out`；没画过或 `out` 不够大返回 `false`。
    pub fn copy_pixels(&self, out: &mut [u8]) -> bool {
        let Some(rendered) = &self.last else {
            return false;
        };
        let data = rendered.pixmap.data();
        match out.get_mut(..data.len()) {
            Some(target) => {
                target.copy_from_slice(data);
                true
            }
            None => false,
        }
    }
}
