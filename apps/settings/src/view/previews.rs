//! 候选窗预览：用 qingjian-render 按主题、外观、排布画样例帧，同样的参数只画一次。

use std::collections::HashMap;
use std::sync::Arc;

use bamboo_slip::FontLibrary;
use qingjian_render::{Frame, Layout, Pixmap, Renderer, Theme, sample};

/// 一张预览的参数：主题 id、深色、排布、显示几个候选、倍数与字号（位模式）。
type PreviewKey = (String, bool, bool, usize, u32, [u32; 2]);

pub struct Previews {
    renderer: Option<Renderer>,

    cache: HashMap<PreviewKey, Arc<Pixmap>>,
}

impl Previews {
    pub fn new() -> Self {
        let renderer = match FontLibrary::system("zh-CN") {
            Ok(library) => Some(Renderer::new(library)),
            Err(error) => {
                tracing::warn!(%error, "字体库加载失败，不画候选窗预览");
                None
            }
        };
        Self {
            renderer,
            cache: HashMap::new(),
        }
    }

    /// `theme` 已换好外观与字号；`dark` 只用来区分缓存。画不出来时为 `None`。
    pub fn get(
        &mut self,
        theme: &Theme,
        dark: bool,
        horizontal: bool,
        rows: usize,
        scale: f32,
        sizes: [f32; 2],
    ) -> Option<Arc<Pixmap>> {
        let key = (
            theme.id().to_owned(),
            dark,
            horizontal,
            rows,
            scale.to_bits(),
            sizes.map(f32::to_bits),
        );
        if let Some(pixmap) = self.cache.get(&key) {
            return Some(Arc::clone(pixmap));
        }
        let renderer = self.renderer.as_mut()?;
        renderer.load_theme_fonts(theme, |_| Vec::new());
        // 每张预览都从头画，不与上一张配对过渡
        renderer.forget();
        let layout = if horizontal {
            Layout::Horizontal
        } else {
            Layout::Vertical
        };
        let frame = shorter(rows);
        match renderer.render(&frame, layout, &theme.with_animations(false), scale) {
            Ok(rendered) => {
                let pixmap = Arc::new(rendered.pixmap);
                self.cache.insert(key, Arc::clone(&pixmap));
                Some(pixmap)
            }
            Err(error) => {
                tracing::warn!(%error, theme = theme.id(), "候选窗预览画不出来");
                None
            }
        }
    }
}

impl Previews {
    /// 同 [`Self::get`]，但要放进 `fit`（宽、高，像素）里：大了就按比例缩小倍数重画（字仍清楚），不拉伸位图。
    /// 缩放比按 5% 一档取，窗口拖动时不会每个像素宽度都重画一张。
    #[allow(clippy::too_many_arguments)]
    pub fn fitted(
        &mut self,
        theme: &Theme,
        dark: bool,
        horizontal: bool,
        rows: usize,
        scale: f32,
        sizes: [f32; 2],
        fit: (f32, f32),
    ) -> Option<Arc<Pixmap>> {
        let full = self.get(theme, dark, horizontal, rows, scale, sizes)?;
        let ratio = (fit.0 / full.width() as f32).min(fit.1 / full.height() as f32);
        if ratio >= 1.0 {
            return Some(full);
        }
        let ratio = ((ratio * 20.0).floor() / 20.0).max(0.05);
        self.get(theme, dark, horizontal, rows, scale * ratio, sizes)
    }
}

/// 真机敲 nihao 的第一页，只留前 `rows` 个候选。
fn shorter(rows: usize) -> Frame {
    let mut frame = sample::nihao();
    frame.rows.truncate(rows);
    frame
}
