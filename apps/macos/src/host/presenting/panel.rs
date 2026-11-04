//! 符号面板的呈现：当前分类的条目排成格子，顶上两行标签，信息行放高亮条目的名字。

use qingjian_core::GRID_ROWS;
use qingjian_render::{PanelCell, PanelFrame};

use crate::candidates::Frame;
use crate::host::Host;

const HINT: &str = "Tab 换组 · [ ] 换类 · 双击或空格上屏";

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
        let (sections, section) = panel.group_sections();
        let Some((cells, columns)) = self.session.grid_cells() else {
            return;
        };
        let top = self.session.grid.map_or(0, |grid| grid.top());
        let size = self.session.layout.page_size();
        let pages = self.session.pages();
        let panel = PanelFrame {
            groups: groups.iter().map(|g| g.label().to_owned()).collect(),
            group: groups.iter().position(|g| *g == panel.group()).unwrap_or(0),
            sections: sections.iter().map(|s| (*s).to_owned()).collect(),
            section,
            cells: cells
                .iter()
                .map(|cell| {
                    cell.candidate().map(|candidate| PanelCell {
                        text: candidate.text.clone(),
                        name: candidate.reading.clone(),
                    })
                })
                .collect(),
            columns,
            rows: GRID_ROWS,
            highlight: self.session.highlighted.saturating_sub(top * size),
            footer: (pages > 1).then(|| format!("{}/{pages}", self.session.page + 1)),
            hint: HINT.to_owned(),
        };
        let frame = Frame {
            mode: self.input_mode(),
            panel: Some(panel),
            ..Frame::default()
        };
        self.window.show(frame, self.anchor);
    }
}
