//! 候选窗的外观、布局与字体：主题、排版方向、渲染器、字号字重、动画与键位提示。
//!
//! 这一组是 [`super::GeneralConfig`] 的扁平子集，序列化时由 `#[serde(flatten)]` 并回
//! `[general]` 分节，键名与旧扁平结构完全一致；这里只是按域把字段归到一处，不改变配置写法。

use serde::{Deserialize, Serialize};

use super::super::{Appearance, CandidateRenderer, FontSize, KeyHintMode, LayoutMode};
use super::{DEFAULT_KEY_HINT_OPACITY, DEFAULT_THEME};

/// 候选窗外观与布局设置（`[general]` 下的显示相关键）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct GeneralDisplay {
    /// 候选窗口外观，读用 [`super::GeneralConfig::appearance`]。文件里没写时为 `None`（字段级缺省），好认出旧写法：
    /// 2026-09-18 之前外观写在 `theme` 里（`theme = "dark"`）。
    #[serde(default)]
    pub appearance: Option<Appearance>,

    /// 候选窗口主题 id（主题目录名），读用 [`super::GeneralConfig::theme_id`]。旧写法里这一项是外观，见上。
    pub theme: String,

    /// 候选窗口竖排 / 横排。
    pub layout: LayoutMode,

    /// 横排时上 / 下键把单行展开成多行矩阵（左 / 右键改为移动候选高亮，Esc 第一下先收回）。缺省关：横排下的按键与以前一样。
    pub horizontal_grid: bool,

    /// 横排时翻页键（主翻页键、额外翻页键、PageUp / PageDown、Tab）也把单行展开成矩阵，与 `↓` 展开同一套。
    /// 缺省关：翻页键照旧整页翻（#805）。
    pub grid_page_keys: bool,

    /// 竖排的候选窗口放在光标上方时（下方放不下）候选倒着排，1 在最下面、紧挨输入行。缺省关：1 总在最上面。
    pub vertical_reverse_above: bool,

    /// 候选窗口由青简渲染器还是系统原生绘制。
    pub renderer: CandidateRenderer,

    /// 候选窗口字体的字族名；空为系统字体。只对青简渲染器生效，没装这个字体时回到系统字体。
    pub font: String,

    /// 候选字字号（点），盖过主题的；0 用主题的。不限范围，不是正数的当 0。只对青简渲染器生效。
    pub candidate_font_size: FontSize,

    /// 译文字号（点），同上。
    pub annotation_font_size: FontSize,

    /// 候选字字重（100–900，与 CSS 的 `font-weight` 一样），盖过主题 `candidate` 样式的；0 用主题的。
    /// 不在范围里的当 0，其余取最近的整百。只对青简渲染器生效。
    pub candidate_font_weight: u16,

    /// 候选窗口的过渡与循环动画。关掉与系统「减弱动态效果」一样：高亮直接跳过去，循环动画停在第一帧。
    pub animations: bool,

    /// 双拼键位提示：在候选窗旁画出当前方案的键位，敲了声母后标出能接的韵母键。见 [`KeyHintMode`]。
    pub key_hint: KeyHintMode,

    /// 键位提示窗口的不透明度（百分比，30–100），见 [`super::GeneralConfig::key_hint_opacity`]。
    pub key_hint_opacity: u8,

    /// 键位提示键盘的大小（百分比，60–150，在候选字号的比例之上再缩放），见 [`super::GeneralConfig::key_hint_size`]。
    pub key_hint_size: u8,
}

impl Default for GeneralDisplay {
    fn default() -> Self {
        Self {
            appearance: Some(Appearance::default()),
            theme: DEFAULT_THEME.to_owned(),
            layout: LayoutMode::default(),
            horizontal_grid: false,
            grid_page_keys: false,
            vertical_reverse_above: false,
            renderer: CandidateRenderer::default(),
            font: String::new(),
            candidate_font_size: FontSize::default(),
            annotation_font_size: FontSize::default(),
            candidate_font_weight: 0,
            animations: true,
            key_hint: KeyHintMode::default(),
            key_hint_opacity: DEFAULT_KEY_HINT_OPACITY,
            key_hint_size: 100,
        }
    }
}
