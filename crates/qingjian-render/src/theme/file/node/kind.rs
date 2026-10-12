//! 节点种类（JSON 里的 `type`）与各自的属性。

use serde::Deserialize;

use super::NodeSpec;
use super::border::BorderSpec;
use super::caret::CaretSpec;
use super::direction::Direction;
use super::fill::FillSpec;
use super::table::TableSpec;
use super::tone_filter::ToneFilter;
use crate::scene::Icon;
use crate::theme::file::{ColorSpec, FontRef};

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub(crate) enum NodeKind {
    /// 框：按方向排子节点（或排成表格），可以有填充、边框与圆角。
    Frame {
        #[serde(default)]
        direction: Direction,

        fill: Option<FillSpec>,

        border: Option<BorderSpec>,

        #[serde(default)]
        radius: f32,

        table: Option<TableSpec>,

        #[serde(default)]
        children: Vec<NodeSpec>,
    },

    /// 一行文字：`bind` 绑定数据，或 `text` 写死。
    Text {
        bind: Option<String>,

        text: Option<String>,

        /// `text.styles` 里的样式名，或在样式上改字号 / 字重。
        font: FontRef,

        color: ColorSpec,

        /// 描边。
        stroke: Option<BorderSpec>,
    },

    /// 矢量图标，在盒子里垂直居中。
    Icon {
        icon: Icon,

        size: f32,

        color: ColorSpec,
    },

    /// 拼音行：各段按状态着色，光标画在光标位置。
    Preedit {
        font: FontRef,

        /// 已敲的拼音。
        typed: ColorSpec,

        /// 光标后未确认的拼音。
        rest: ColorSpec,

        /// 纠错改掉的字母，带删除线。
        struck: ColorSpec,

        caret: CaretSpec,

        /// 拼音的描边。
        stroke: Option<BorderSpec>,
    },

    /// 一串译文片段，按深浅着色。
    Annotation {
        bind: String,

        font: FontRef,

        /// 译文。
        gloss: ColorSpec,

        /// 生词的译文。
        fresh: ColorSpec,

        /// 淡色片段（假名注音）；词性与分隔符没单独写颜色时也用它。
        faint: ColorSpec,

        /// 词性；不写用 `faint`。
        pos: Option<ColorSpec>,

        /// 义项间的分隔；不写用 `faint`。
        separator: Option<ColorSpec>,

        /// 只画这几种片段（词性、译文分两行排时各写一个节点）；不写全画。
        tones: Option<Vec<ToneFilter>>,

        /// 只画前几个义项（按分隔数）；不写全画。
        senses: Option<usize>,

        /// 描边。
        stroke: Option<BorderSpec>,
    },

    /// 引用一个组件；这里写的盒子属性盖过组件根节点的。
    Use { component: String },

    /// 按列表数据把组件重复多份，每份绑定一项。
    Repeat { bind: String, component: String },
}
