//! 符号面板：组句中按 ↑（高亮还在第一个、没动过）打开，按「搜索 / 符号 / 表情」三组分类列出符号、emoji 与颜文字。
//! 数据编进程序（`crates/qingjian-core/data/panel/`，`tools/corpus/symbol_panel.py` 生成）；搜索拿当前候选词查 CLDR 中文关键词。
//! 面板只管内容与分类切换，格子里怎么移动高亮交给 [`crate::Grid`]，与横排矩阵同一套。

mod catalog;
mod group;
mod section;
mod state;

pub(crate) use catalog::{CATALOG, KEYWORDS};
pub use group::PanelGroup;
pub use section::{PANEL_COLUMNS, PanelSection};
pub use state::SymbolPanel;
