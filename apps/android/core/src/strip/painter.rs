//! 候选条的画笔：记住最新一帧要显示的内容，壳要画时按宽度倍数与深浅色出一张位图。

use qingjian_render::{
    FontLibrary, Frame as RenderFrame, Layout, Mode, Preedit, Rendered, Renderer, Theme,
};

use super::{StripHit, StripMeta, has_emoji, rows};
use crate::appearance;
use crate::error::AndroidError;
use crate::frame::Frame;

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
            preedit: (!frame.preedit.is_empty())
                .then(|| Preedit::plain(&frame.preedit, frame.preedit.chars().count())),
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
        };
    }

    /// 画最新一帧。`scale` 是 dp → 像素的倍数（`displayMetrics.density`）。
    pub fn paint(
        &mut self,
        highlighted: Option<usize>,
        scale: f32,
        dark: bool,
    ) -> Result<StripMeta, AndroidError> {
        self.content.highlighted = highlighted.filter(|&i| i < self.content.rows.len());
        let theme = if dark { &self.dark } else { &self.light };
        let rendered = self
            .renderer
            .render(&self.content, Layout::Strip, theme, scale)?;
        let offset = (rendered.content_x, rendered.content_y);
        let background = theme.variable("surface").map_or(0, appearance::argb);
        let meta = StripMeta {
            width: rendered.pixmap.width(),
            height: rendered.pixmap.height(),
            candidates_top: rendered.candidates_top(),
            background,
            hits: rendered
                .hits
                .iter()
                .filter_map(|region| StripHit::new(region, offset))
                .collect(),
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
