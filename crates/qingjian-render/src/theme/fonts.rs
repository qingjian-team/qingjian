//! 主题的文字样式与素材访问器：命名样式、字号比例、字族表、随主题带的字体与图片缓存。

use std::path::PathBuf;
use std::sync::Arc;

use bamboo_slip::{FamilyTable, FontSpec};

use super::assets::Assets;
use super::file::FontRef;
use super::text_sizes;
use super::Theme;

/// 引用不到的文字样式退回这个（点）。
const FALLBACK_FONT: FontSpec = FontSpec::new(16.0, 19.0);

impl Theme {
    /// 主题自己写的字重（不算设置里的）：设置界面拿它当字重框的初始值。
    pub fn theme_weight(&self, name: &str) -> Option<u16> {
        self.families.style(name).map(|spec| spec.weight.0)
    }

    /// 设置里的候选字号相对主题的比例：表格行高与键位提示键盘跟着它缩放。
    pub(crate) fn candidate_scale(&self) -> f32 {
        self.sizes
            .ratio("candidate", |name| self.theme_size(name))
    }

    /// 主题自己写的字号（不算设置里的）：设置界面拿它当字号框的初始值。
    pub fn theme_size(&self, name: &str) -> Option<f32> {
        self.families.style(name).map(|spec| spec.size)
    }

    /// 样式里写到的系统字族名（不含 `system`）。壳按名字查出字体文件，交给 [`crate::Renderer::load_theme_fonts`]。
    pub fn font_families(&self) -> Vec<String> {
        self.families.table().names()
    }

    /// 随主题带的字体文件（已检查在主题目录里）。
    pub fn font_files(&self) -> &[PathBuf] {
        self.assets().fonts()
    }

    pub(super) fn assets(&self) -> &Assets {
        self.assets.get_or_init(|| {
            if self.embedded.is_empty() {
                Assets::default()
            } else {
                Assets::embedded(&self.file, self.embedded)
            }
        })
    }

    /// 命名文字样式（点）。
    pub(crate) fn font(&self, name: &str) -> FontSpec {
        let spec = self.style(name).unwrap_or(FALLBACK_FONT);
        text_sizes::scaled(spec, self.sizes.ratio(name, |base| self.theme_size(base)))
    }

    /// 主题里的命名样式，候选字换上设置里的字重。
    fn style(&self, name: &str) -> Option<FontSpec> {
        let spec = self.families.style(name)?;
        Some(match self.candidate_weight {
            Some(weight) if name == "candidate" => FontSpec { weight, ..spec },
            _ => spec,
        })
    }

    /// 字族表，画笔按 [`bamboo_slip::FamilyId`] 查回退链；两份 `Theme` 共用同一张表时指针相同。
    pub(crate) fn families(&self) -> &Arc<FamilyTable> {
        self.families.table()
    }

    /// 节点里的 `font`：样式名，或在样式上改字号 / 字重。
    /// 在样式上写死的字号也按设置里那个样式的比例缩放（手机候选条在 `candidate` 上写 19 点，用户调大候选字号时它跟着大）。
    pub(crate) fn font_ref(&self, font: &FontRef) -> FontSpec {
        let Some(name) = font.base() else {
            return font.apply(FALLBACK_FONT);
        };
        let ratio = self.sizes.ratio(name, |base| self.theme_size(base));
        let base = self.style(name).unwrap_or(FALLBACK_FONT);
        text_sizes::scaled(font.apply(base), ratio)
    }

    /// 当前外观下的文字覆盖率 gamma。
    pub(crate) fn text_gamma(&self) -> f32 {
        self.file.text.gamma.get(self.dark)
    }
}
