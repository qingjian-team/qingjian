//! 符号面板的呈现：当前分类的条目排成格子，顶上两行标签，信息行放高亮条目的名字。

use crate::candidates::{Frame, PanelTabs, Row};
use crate::host::Host;

impl Host {
    /// 面板刚打开或换了分类：格子换成当前分类的条目，高亮回到第一格。
    pub fn load_panel(&mut self) {
        let Some(section) = self.engine.symbol_panel().map(|p| p.section().clone()) else {
            return;
        };
        let columns = section.columns();
        self.session.open_panel(section.items, columns);
    }

    /// 面板这一帧：视口里的几行格子与标签。格子不标序号，选择靠方向键加空格 / 回车。
    pub(super) fn render_panel(&mut self) {
        let Some(panel) = self.engine.symbol_panel() else {
            return;
        };
        let groups = panel.groups();
        let tabs = PanelTabs {
            groups: groups.iter().map(|g| g.label().to_owned()).collect(),
            group: groups.iter().position(|g| *g == panel.group()).unwrap_or(0),
            sections: panel
                .group_sections()
                .0
                .iter()
                .map(|s| (*s).to_owned())
                .collect(),
            section: panel.group_sections().1,
        };
        let Some((cells, columns)) = self.session.grid_cells() else {
            return;
        };
        let top = self.session.grid.map_or(0, |grid| grid.top());
        let rows = cells
            .iter()
            .map(|cell| match cell.candidate() {
                Some(candidate) => {
                    let mut row =
                        Row::from_candidate(0, candidate, self.translation_pos, self.aux_code_show);
                    row.index = String::new();
                    row
                }
                None => Row {
                    index: String::new(),
                    text: String::new(),
                    code: None,
                    annotation: Vec::new(),
                    cloud: false,
                },
            })
            .collect();
        let size = self.session.layout.page_size();
        let pages = self.session.pages();
        let frame = Frame {
            preedit: None,
            rows,
            highlighted: self.session.highlighted.saturating_sub(top * size),
            columns,
            column_ems: Vec::new(),
            footer: (pages > 1).then(|| format!("{}/{pages}", self.session.page + 1)),
            pages_around: (self.session.page > 0, self.session.page + 1 < pages),
            sentence: None,
            status: None,
            mode: self.input_mode(),
            panel: Some(tabs),
            keyboard: None,
        };
        self.window.show(frame, self.anchor);
    }
}
