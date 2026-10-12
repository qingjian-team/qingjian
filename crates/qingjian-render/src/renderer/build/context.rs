//! 实例化时的数据上下文：整帧，加上 `repeat` 展开中的那一项（候选，或状态条的一格）。显示条件（`when`）与绑定（`bind`）都在这里求值。
//!
//! 名字先在候选项里找，再在整帧里找；不认识的条件为假、不认识的绑定为空（节点不画）。

use super::cell::CellContext;
use super::row::RowContext;
use crate::frame::{Frame, Row, Tone};
use crate::renderer::StatusCell;

#[derive(Debug, Clone, Copy)]
pub(super) struct Context<'a> {
    pub(super) frame: &'a Frame,

    /// 在 `repeat` 里时绑定的那一项。
    pub(super) row: Option<RowContext<'a>>,

    /// 状态条的格子（`repeat` 绑定 `cells`）；画候选窗口时为空。
    pub(super) cells: &'a [StatusCell],

    /// 在 `repeat` 里时绑定的那一格。
    pub(super) cell: Option<CellContext<'a>>,
}

impl<'a> Context<'a> {
    pub(super) fn new(frame: &'a Frame) -> Self {
        Self {
            frame,
            row: None,
            cells: &[],
            cell: None,
        }
    }

    /// 状态条：整帧只用到输入状态，格子供 `repeat` 展开。
    pub(super) fn status(frame: &'a Frame, cells: &'a [StatusCell]) -> Self {
        Self {
            cells,
            ..Self::new(frame)
        }
    }

    /// 展开格子时每一格的上下文。
    pub(super) fn with_cell(self, cell: &'a StatusCell, index: usize, count: usize) -> Self {
        Self {
            cell: Some(CellContext { cell, index, count }),
            ..self
        }
    }

    /// 展开列表时每一项的上下文。
    pub(super) fn with_row(self, row: &'a Row, index: usize, count: usize) -> Self {
        Self {
            row: Some(RowContext { row, index, count }),
            ..self
        }
    }

    /// `when` 求值：`a|b` 任一成立，`!a` 取反。
    pub(super) fn holds(&self, condition: &str) -> bool {
        condition.split('|').any(|term| {
            let term = term.trim();
            match term.strip_prefix('!') {
                Some(name) => !self.flag(name),
                None => self.flag(term),
            }
        })
    }

    fn flag(&self, name: &str) -> bool {
        if let Some(CellContext { cell, index, count }) = self.cell {
            match name {
                "emphasized" => {
                    return matches!(
                        cell,
                        StatusCell::Text {
                            emphasized: true,
                            ..
                        }
                    );
                }
                "gear" => return *cell == StatusCell::Gear,
                "first" => return index == 0,
                "last" => return index + 1 == count,
                _ => {}
            }
        }
        if let Some(RowContext { row, index, count }) = self.row {
            match name {
                "highlighted" => return self.frame.highlighted == Some(index),
                "cloud" => return row.cloud,
                "annotation" => return !row.annotation.is_empty(),
                "code" => return row.code.is_some(),
                "first" => return index == 0,
                "last" => return index + 1 == count,
                _ => {}
            }
        }
        let frame = self.frame;
        match name {
            "preedit" => frame.preedit.is_some(),
            "trailing" => frame.trailing().is_some(),
            "trailing.cloud" => frame.trailing().is_some_and(|(_, cloud)| cloud),
            "page" => frame.footer.is_some(),
            "candidates" => !frame.rows.is_empty(),
            "annotations" => frame.rows.iter().any(|row| !row.annotation.is_empty()),
            "highlighted" => self.highlighted().is_some(),
            "highlighted.annotation" => self
                .highlighted()
                .is_some_and(|row| !row.annotation.is_empty()),
            "mode.english" => frame.mode.english,
            "mode.traditional" => frame.mode.traditional,
            "mode.full_width" => frame.mode.full_width,
            "mode.scheme" => !frame.mode.scheme.is_empty(),
            _ => false,
        }
    }

    /// 文字绑定。
    pub(super) fn text(&self, name: &str) -> Option<&'a str> {
        if let Some(CellContext {
            cell: StatusCell::Text { text, .. },
            ..
        }) = self.cell
            && name == "text"
        {
            return Some(text);
        }
        if let Some(RowContext { row, .. }) = self.row {
            match name {
                "index" => return Some(&row.index),
                "text" => return Some(&row.text),
                "code" => return row.code.as_deref(),
                _ => {}
            }
        }
        match name {
            "page" => self.frame.footer.as_deref(),
            "trailing.text" => self.frame.trailing().map(|(text, _)| text),
            "mode.scheme" => Some(self.frame.mode.scheme.as_str()).filter(|s| !s.is_empty()),
            _ => None,
        }
    }

    /// 译文片段绑定。
    pub(super) fn annotation(&self, name: &str) -> Option<&'a [(String, Tone)]> {
        match (name, self.row) {
            ("annotation", Some(RowContext { row, .. })) => Some(&row.annotation),
            ("highlighted.annotation", _) => {
                self.highlighted().map(|row| row.annotation.as_slice())
            }
            _ => None,
        }
    }

    /// 列表绑定。
    pub(super) fn list(&self, name: &str) -> Option<&'a [Row]> {
        (name == "candidates").then_some(self.frame.rows.as_slice())
    }

    fn highlighted(&self) -> Option<&'a Row> {
        self.frame.highlighted.and_then(|i| self.frame.rows.get(i))
    }
}

#[cfg(test)]
mod tests {
    use super::Context;
    use crate::frame::{Frame, Mode};

    #[test]
    fn mode_conditions_and_scheme_binding() {
        let frame = Frame {
            mode: Mode {
                english: true,
                traditional: false,
                full_width: true,
                scheme: "小鹤双拼".to_owned(),
            },
            ..Frame::default()
        };
        let ctx = Context::new(&frame);
        assert!(ctx.holds("mode.english") && !ctx.holds("!mode.english"));
        assert!(!ctx.holds("mode.traditional") && ctx.holds("mode.full_width"));
        assert!(ctx.holds("mode.scheme") && !Context::new(&Frame::default()).holds("mode.scheme"));
        assert_eq!(ctx.text("mode.scheme"), Some("小鹤双拼"));
        assert_eq!(Context::new(&Frame::default()).text("mode.scheme"), None);
    }
}
