//! 原生分页直接映射为显示帧；总页数未知时只用下一页存在性，不遍历词库。
use super::Frame;
use crate::config::{LayoutMode, PreeditMode, ThemeMode};
use qingjian_core::Query;

impl Frame {
    pub fn from_rime(
        query: Query,
        preedit_mode: PreeditMode,
        layout: LayoutMode,
        theme: ThemeMode,
    ) -> Self {
        let menu = query.rime_menu.as_ref().expect("原生查询包含菜单");
        if query.marked_text().is_empty() && query.candidates.items.is_empty() {
            return Self::default();
        }
        Self {
            preedit: query.marked_segments().iter().map(Into::into).collect(),
            cursor: query.marked_cursor(),
            highlight: if query.candidates.items.is_empty() {
                usize::MAX
            } else {
                menu.highlighted
            },
            page: menu.page,
            page_count: menu.page + 1 + usize::from(!menu.last_page),
            candidates: query.candidates,
            preedit_mode,
            layout,
            theme,
            aux_code_show: false,
            sentence: None,
            notice: None,
        }
    }
}
