//! 符号面板的呈现：当前分类的条目排成格子，顶上两行标签，信息行放高亮条目的名字；
//! 翻译组的格子区换成翻译框（原文、拼音与当前一页候选、状态）。

use qingjian_core::{GRID_ROWS, PanelGroup, SymbolPanel};
use qingjian_render::{PanelCell, PanelFrame, PanelTranslate};

use crate::candidates::Frame;
use crate::host::Host;

const HINT: &str = "Tab 换组 · [ ] 换类 · 双击或空格上屏";

const TRANSLATE_HINT: &str = "Tab 换组 · [ ] 换语言 · 回车上屏";

impl Host {
    /// 面板刚打开或换了分类：格子换成当前分类的条目，高亮回到第一格。翻译组没有格子，会话里放拼音的候选。
    /// 返回停没停在翻译组（停在那里时壳要重查候选，见控制器的 `refresh`）。
    pub fn load_panel(&mut self) -> bool {
        if self.engine.panel_translating() {
            self.session.collapse();
            return true;
        }
        let Some(section) = self.engine.symbol_panel().map(|p| p.section().clone()) else {
            return false;
        };
        let columns = section.columns();
        self.session.open_panel(section.items, columns);
        false
    }

    /// 面板这一帧：视口里的几行格子与标签。格子不标序号，选择靠方向键加空格 / 回车。
    pub(super) fn render_panel(&mut self) {
        let Some(panel) = self.engine.symbol_panel() else {
            return;
        };
        let groups = panel.groups();
        let (sections, section) = panel.group_sections();
        let translating = panel.group() == PanelGroup::Translate;
        let grid = self.session.grid_cells();
        let (cells, columns) = match grid {
            Some(grid) => grid,
            None if translating => (Vec::new(), 1),
            None => return,
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
            hint: if translating { TRANSLATE_HINT } else { HINT }.to_owned(),
            translate: translating.then(|| self.panel_translate(panel)),
        };
        let frame = Frame {
            mode: self.input_mode(),
            panel: Some(panel),
            ..Frame::default()
        };
        self.window.show(frame, self.anchor);
    }

    /// 翻译框：原文、还在打的拼音与它当前一页的候选。
    fn panel_translate(&self, panel: &SymbolPanel) -> PanelTranslate {
        let size = self.session.layout.page_size();
        let first = self.session.page * size;
        PanelTranslate {
            source: panel.translate_source().to_owned(),
            cursor: panel.translate_cursor(),
            selection: panel.translate_selection(),
            placeholder: panel.translate_placeholder().to_owned(),
            composing: self.engine.composition().text().to_owned(),
            candidates: (first..first + size)
                .map_while(|index| self.session.candidate(index))
                .map(|candidate| candidate.text)
                .collect(),
            highlight: self.session.highlighted.saturating_sub(first),
            status: panel.translate_line(),
        }
    }
}
