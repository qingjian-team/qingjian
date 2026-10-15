//! 符号面板：组句中高亮还在第一个、没动过时按 ↑ 打开（按键在 [`super::key`]），也能从悬浮条与任务栏图标菜单打开，
//! 那时没在组句就没有搜索组。分类与搜索在 Core（[`qingjian_core::SymbolPanel`]），当前分类的格子在 [`PanelGrid`]，
//! 画面由 UI 线程按 [`PanelFrame`] 自绘；鼠标经 [`PanelMouse`] 回来。

mod frame;
mod grid;
mod mouse;

use qingjian_core::SymbolPanel;
use qingjian_platform::protocol::SessionId;

pub use self::frame::{PanelCell, PanelFrame};
pub(super) use self::grid::PanelGrid;
pub use self::mouse::PanelMouse;
use super::Router;

impl Router {
    /// ↑ 打开：组句中、高亮在第一个而且这轮没动过。返回开没开。
    pub(super) fn open_panel_from_key(&mut self) -> bool {
        if self.navigated || self.highlight != 0 || self.translation.is_some() || !self.composing()
        {
            return false;
        }
        self.open_panel()
    }

    /// 悬浮条、任务栏图标菜单：不管在不在组句都打开，上屏到 `session`（最近取过模式的那个应用）。
    pub(super) fn open_panel_for(&mut self, session: Option<SessionId>) {
        if let Some(session) = session {
            self.ensure_focus(session);
        }
        if self.focused.is_none() || self.translation.is_some() || self.panel_open() {
            return;
        }
        if self.open_panel() {
            self.show_frame();
        }
    }

    fn open_panel(&mut self) -> bool {
        if !self.engine.open_symbol_panel() {
            return false;
        }
        tracing::debug!("打开符号面板");
        self.load_panel();
        true
    }

    /// 格子换成当前分类的条目，高亮回到第一格。
    fn load_panel(&mut self) {
        self.panel = self
            .engine
            .symbol_panel()
            .map(|panel| PanelGrid::new(panel.section().items.clone(), panel.section().columns()));
    }

    pub(super) fn panel_open(&self) -> bool {
        self.panel.is_some() && self.engine.symbol_panel().is_some()
    }

    pub(super) fn close_panel(&mut self) {
        self.engine.close_symbol_panel();
        self.panel = None;
    }

    /// 换组或换分类，换了就把格子换成新分类的条目。返回换没换。
    pub(super) fn switch_panel(&mut self, switch: impl FnOnce(&mut SymbolPanel) -> bool) -> bool {
        let switched = self.engine.symbol_panel_mut().is_some_and(switch);
        if switched {
            self.load_panel();
        }
        switched
    }

    /// 上屏第 `index` 个条目：Core 连同整段搜索词一起吃掉，面板随之收起。
    pub(super) fn commit_panel(&mut self, index: usize) -> Option<String> {
        let candidate = self.panel.as_ref()?.candidate(index)?.clone();
        self.panel = None;
        Some(self.engine.commit(&candidate))
    }

    /// 鼠标：单击格子挪高亮、双击上屏，点胶囊换组，点标签换分类，滚轮滚视口，点 ✕ 收起。
    pub fn handle_panel_mouse(&mut self, hit: PanelMouse) {
        if !self.panel_open() {
            return;
        }
        let changed = match hit {
            PanelMouse::Cell { offset, double } => {
                let Some(index) = self.panel.as_ref().and_then(|g| g.index_in_view(offset)) else {
                    return;
                };
                if double {
                    self.click_commit_panel(index);
                    return;
                }
                self.panel.as_mut().is_some_and(|g| g.select(index))
            }
            PanelMouse::Scroll(rows) => self.panel.as_mut().is_some_and(|g| g.scroll(rows)),
            PanelMouse::Group(group) => self.switch_panel(|panel| panel.select_group(group)),
            PanelMouse::Section(section) => {
                self.switch_panel(|panel| panel.select_section(section))
            }
            PanelMouse::Close => {
                self.close_panel();
                self.recompose();
                true
            }
        };
        if changed {
            self.show_frame();
        }
    }

    /// 双击上屏：同点候选一样挂到会话上等 DLL 取走。
    fn click_commit_panel(&mut self, index: usize) {
        let Some(session) = self.click_session() else {
            return;
        };
        let Some(text) = self.commit_panel(index) else {
            return;
        };
        tracing::debug!("双击上屏面板条目");
        self.deliver_clicked(session, text);
    }

    /// 按当前状态重画（面板或候选）。
    fn show_frame(&mut self) {
        let shown = self.self_drawn_frame();
        self.reconcile_candidates(&shown);
    }

    /// 面板这一帧；没开着返回 `None`。
    pub(super) fn panel_frame(&self) -> Option<PanelFrame> {
        let panel = self.engine.symbol_panel()?;
        let grid = self.panel.as_ref()?;
        let groups = panel.groups();
        let (sections, section) = panel.group_sections();
        Some(PanelFrame {
            groups: groups.iter().map(|g| g.label().to_owned()).collect(),
            group: groups.iter().position(|g| *g == panel.group()).unwrap_or(0),
            sections: sections.iter().map(|s| (*s).to_owned()).collect(),
            section,
            cells: grid.cells(),
            columns: grid.columns(),
            highlight: grid.highlight_in_view(),
            footer: grid.footer(),
            appearance: self.config.appearance,
        })
    }

    /// 面板开着时画面板（不画候选窗）；没变就不重画。没有光标矩形（从菜单打开）时由 UI 摆在鼠标旁。
    pub(super) fn reconcile_panel(&mut self, frame: PanelFrame) {
        let rect = self.last_rect;
        if self
            .last_panel
            .as_ref()
            .is_some_and(|(f, r)| *f == frame && *r == rect)
        {
            return;
        }
        if self.last_panel.is_none() {
            self.candidates.hide();
            self.last_shown = None;
        }
        self.candidates.show_panel(frame.clone(), rect);
        self.last_panel = Some((frame, rect));
    }

    pub(super) fn hide_panel_window(&mut self) {
        if self.last_panel.take().is_some() {
            self.candidates.hide_panel();
        }
    }
}

#[cfg(test)]
mod tests;
