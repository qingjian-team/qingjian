//! 符号面板的按键与点击：组句中高亮还在第一个时按 ↑ 打开（长按连发不算），开着时方向键在格子里移动、
//! Tab 换组、`[` `]` 换分类、空格 / 回车 / 双击上屏、Esc / 退格回到候选；敲字母先收起面板再照常组句。

use super::*;
use crate::candidates::PanelMouse;

/// 面板里按键归谁：`Some(吃没吃)` 是面板处理了，`None` 是面板已收起、按键照常往下走。
type PanelKey = Option<bool>;

impl QingjianInputController {
    /// 组句中、高亮还在第一页第一个而且这轮没动过：打开面板。返回开没开。
    pub(super) fn open_panel(&self, client: TextClient<'_>) -> bool {
        let opened = host::with(|h| {
            let untouched = !h.session.navigated && h.session.page == 0 && h.session.grid.is_none();
            if !untouched || !h.engine.open_symbol_panel() {
                return false;
            }
            h.load_panel();
            true
        })
        .unwrap_or(false);
        if opened {
            tracing::debug!("打开符号面板");
            self.render(client);
        }
        opened
    }

    /// 菜单里的「符号面板…」：不管在不在组句都打开，没敲字时没有搜索组。
    pub(super) fn open_panel_from_menu(&self) {
        // SAFETY: IMKInputController 的 `client` 返回当前会话的文本客户端（实现 IMKTextInput 的代理），可能为 nil
        let client: Option<Retained<AnyObject>> = unsafe { msg_send![self, client] };
        let Some(client) = client else {
            return;
        };
        let client = TextClient::new(&client);
        let opened = host::with(|h| {
            if !h.engine.open_symbol_panel() {
                return false;
            }
            h.load_panel();
            true
        })
        .unwrap_or(false);
        if opened {
            self.render(client);
        }
    }

    pub(super) fn handle_panel_key(
        &self,
        event: &NSEvent,
        key: u16,
        pressed: Modifiers,
        client: TextClient<'_>,
    ) -> PanelKey {
        if pressed.command || pressed.control {
            return Some(false);
        }
        let moved = match key {
            126 => Some(host::with(|h| h.session.move_rows(-1))),
            125 => Some(host::with(|h| h.session.move_rows(1))),
            123 => Some(host::with(|h| h.session.move_cells(-1))),
            124 => Some(host::with(|h| h.session.move_cells(1))),
            116 => Some(host::with(|h| h.session.move_screens(-1))),
            121 => Some(host::with(|h| h.session.move_screens(1))),
            _ => None,
        };
        if let Some(moved) = moved {
            if moved.unwrap_or(false) {
                self.render(client);
            }
            return Some(true);
        }
        match key {
            48 => {
                let delta = if pressed.shift { -1 } else { 1 };
                self.switch_panel(|panel| panel.switch_group(delta), client);
                return Some(true);
            }
            49 | 36 | 76 => {
                let index = host::with(|h| h.session.highlighted).unwrap_or(0);
                self.commit_index(index, client);
                return Some(true);
            }
            53 | 51 => {
                self.close_panel(client);
                return Some(true);
            }
            _ => {}
        }
        let text = event
            .characters()
            .map(|t| t.to_string())
            .unwrap_or_default();
        let mut chars = text.chars();
        let (Some(c), None) = (chars.next(), chars.next()) else {
            return Some(true);
        };
        let page_keys = host::with(|h| h.page_keys).unwrap_or(qingjian_platform::DEFAULT_PAGE_KEYS);
        if c == '[' || c == page_keys.0 {
            self.switch_panel(|panel| panel.switch_section(-1), client);
            return Some(true);
        }
        if c == ']' || c == page_keys.1 {
            self.switch_panel(|panel| panel.switch_section(1), client);
            return Some(true);
        }
        // 数字键选高亮那一行的第几格
        if let Some(digit) = c.to_digit(10).filter(|d| *d > 0) {
            let index = host::with(|h| {
                let columns = h.session.layout.page_size();
                let index = h.session.highlighted / columns * columns + digit as usize - 1;
                (digit as usize <= columns && h.session.candidate(index).is_some()).then_some(index)
            })
            .flatten();
            if let Some(index) = index {
                self.commit_index(index, client);
            }
            return Some(true);
        }
        if c.is_ascii_alphabetic() {
            // 接着敲字：收起面板，这个字照常进组句
            host::with(|h| h.engine.close_symbol_panel());
            return None;
        }
        Some(true)
    }

    /// 换组或换分类，换了就把格子换成新分类的条目重画。
    fn switch_panel(
        &self,
        switch: impl FnOnce(&mut qingjian_core::SymbolPanel) -> bool,
        client: TextClient<'_>,
    ) {
        let switched = host::with(|h| {
            let switched = h.engine.symbol_panel_mut().is_some_and(switch);
            if switched {
                h.load_panel();
            }
            switched
        })
        .unwrap_or(false);
        if switched {
            self.render(client);
        }
    }

    /// 收起面板回到候选：拼音还在，重新查一遍。
    fn close_panel(&self, client: TextClient<'_>) {
        host::with(|h| h.engine.close_symbol_panel());
        self.refresh(client);
    }

    /// 鼠标：单击格子挪高亮、双击上屏，点胶囊换组，点标签换分类，滚轮滚视口，点 ✕ 收起。
    pub(super) fn panel_mouse_event(&self, hit: PanelMouse, client: TextClient<'_>) {
        if !host::with(|h| h.engine.symbol_panel().is_some()).unwrap_or(false) {
            return;
        }
        match hit {
            PanelMouse::Cell { offset, double } => {
                let Some(index) = host::with(|h| h.session.index_in_view(offset)).flatten() else {
                    return;
                };
                if double {
                    self.commit_index(index, client);
                } else if host::with(|h| h.session.select(index)).unwrap_or(false) {
                    self.render(client);
                }
            }
            PanelMouse::Group(group) => {
                self.switch_panel(|panel| panel.select_group(group), client);
            }
            PanelMouse::Section(section) => {
                self.switch_panel(|panel| panel.select_section(section), client);
            }
            PanelMouse::Scroll(rows) => {
                if host::with(|h| h.session.scroll_rows(rows)).unwrap_or(false) {
                    self.render(client);
                }
            }
            PanelMouse::Close => self.close_panel(client),
        }
    }
}
