//! 符号面板开着时的按键：方向键在格子里移动、翻页键（PgUp / PgDn）整屏翻，Tab / Shift+Tab 换组，
//! `[` `]` 与配置的翻页键换分类，空格 / 回车 / 数字上屏，Esc / 退格收起回到候选；敲字母先收起面板再照常组句。
//! 与 macOS 壳的 `handle_panel_key` 对齐。

use qingjian_core::GRID_ROWS;
use qingjian_platform::protocol::KeyEvent;

use super::{Effect, codes};
use crate::dispatch::Router;

impl Router {
    /// 面板开着时先过这里：`Some` 是面板处理了，`None` 是面板没开或已收起、按键照常往下走。
    pub(super) fn apply_panel_key(&mut self, event: &KeyEvent) -> Option<Effect> {
        if !self.panel_open() {
            return None;
        }
        if event.modifiers.has_command_key() {
            return Some(Effect::Passthrough);
        }
        let grid = self.panel.as_mut()?;
        let screen = GRID_ROWS as isize;
        let moved = match event.virtual_key {
            codes::UP => Some(grid.move_rows(-1)),
            codes::DOWN => Some(grid.move_rows(1)),
            codes::LEFT => Some(grid.move_cells(-1)),
            codes::RIGHT => Some(grid.move_cells(1)),
            codes::PRIOR => Some(grid.move_rows(-screen)),
            codes::NEXT => Some(grid.move_rows(screen)),
            _ => None,
        };
        if moved.is_some() {
            return Some(Effect::Navigated);
        }
        match event.virtual_key {
            codes::TAB => {
                let delta = if event.modifiers.shift { -1 } else { 1 };
                self.switch_panel(|panel| panel.switch_group(delta));
                return Some(Effect::Navigated);
            }
            codes::RETURN => return Some(self.commit_panel_highlight()),
            codes::ESCAPE | codes::BACK => {
                self.close_panel();
                return Some(Effect::Changed(None));
            }
            _ => {}
        }
        let Some(c) = event.character.filter(|c| !c.is_control()) else {
            return Some(Effect::Navigated);
        };
        if c == ' ' {
            return Some(self.commit_panel_highlight());
        }
        // 数字选高亮那一行的第几格；那一格空着就不算
        if let Some(digit) = codes::digit(event) {
            let index = self.panel.as_ref().and_then(|g| g.index_in_row(digit));
            return Some(match index.and_then(|index| self.commit_panel(index)) {
                Some(text) => Effect::Changed(Some(text)),
                None => Effect::Navigated,
            });
        }
        let step = match c {
            '[' => Some(-1),
            ']' => Some(1),
            _ => self.page_step(event),
        };
        if let Some(step) = step {
            self.switch_panel(|panel| panel.switch_section(step));
            return Some(Effect::Navigated);
        }
        if c.is_ascii_alphabetic() {
            self.close_panel();
            return None;
        }
        Some(Effect::Navigated)
    }

    fn commit_panel_highlight(&mut self) -> Effect {
        let index = self.panel.as_ref().map_or(0, |grid| grid.highlight());
        match self.commit_panel(index) {
            Some(text) => Effect::Changed(Some(text)),
            None => Effect::Navigated,
        }
    }
}
